mod audio;
mod config;
mod tray;

use anyhow::Result;
use audio::AudioRouter;
use config::AppConfig;
use muda::MenuEvent;
use tracing::{info, error, warn};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

fn format_balance(bal: f32) -> String {
    if bal < -0.01 {
        format!("{}% Left", (bal.abs() * 100.0) as i32)
    } else if bal > 0.01 {
        format!("{}% Right", (bal * 100.0) as i32)
    } else {
        "Center".to_string()
    }
}

struct App {
    router: AudioRouter,
    config: AppConfig,
    source_name: String,
    target_name: String,
    tray_manager: Option<tray::TrayManager>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _id: WindowId, _event: WindowEvent) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Process menu events
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if let Some(ref tray_manager) = self.tray_manager {
                if let Some(cmd) = tray_manager.handle_menu_event(&event) {
                    match cmd {
                        tray::TrayCommand::ToggleEnabled => {
                            self.config.enabled = !self.config.enabled;
                            if self.config.enabled {
                                if let Err(e) = self.router.start_loopback(&self.source_name, &self.target_name) {
                                    error!("Failed to start: {}", e);
                                } else {
                                    info!("Routing enabled");
                                }
                            } else {
                                self.router.stop();
                                info!("Routing disabled");
                            }
                            tray_manager.update_toggle_text(self.config.enabled);
                            let _ = self.config.save();
                        }
                        tray::TrayCommand::ToggleSwapChannels => {
                            self.config.swap_channels = !self.config.swap_channels;
                            self.router.set_swap_channels(self.config.swap_channels);
                            tray_manager.update_swap_checked(self.config.swap_channels);
                            info!("Swap channels: {}", self.config.swap_channels);
                            let _ = self.config.save();
                        }
                        tray::TrayCommand::SetVolume(vol) => {
                            self.config.volume = vol;
                            self.router.set_volume(vol);
                            info!("Volume set to {}%", (vol * 100.0) as i32);
                            let _ = self.config.save();
                        }
                        tray::TrayCommand::SetBalance(bal) => {
                            self.config.balance = bal;
                            self.router.set_balance(bal);
                            info!("Balance set to {}", format_balance(bal));
                            let _ = self.config.save();
                        }
                        tray::TrayCommand::SetLeftSource(source) => {
                            self.config.left_channel.source = source;
                            self.router.set_left_source(source);
                            info!("Left source: {:?}", source);
                            let _ = self.config.save();
                        }
                        tray::TrayCommand::SetRightSource(source) => {
                            self.config.right_channel.source = source;
                            self.router.set_right_source(source);
                            info!("Right source: {:?}", source);
                            let _ = self.config.save();
                        }
                        tray::TrayCommand::ToggleLeftMute => {
                            self.config.left_channel.muted = !self.config.left_channel.muted;
                            self.router.set_left_muted(self.config.left_channel.muted);
                            tray_manager.update_left_mute(self.config.left_channel.muted);
                            info!("Left mute: {}", self.config.left_channel.muted);
                            let _ = self.config.save();
                        }
                        tray::TrayCommand::ToggleRightMute => {
                            self.config.right_channel.muted = !self.config.right_channel.muted;
                            self.router.set_right_muted(self.config.right_channel.muted);
                            tray_manager.update_right_mute(self.config.right_channel.muted);
                            info!("Right mute: {}", self.config.right_channel.muted);
                            let _ = self.config.save();
                        }
                        tray::TrayCommand::SetLeftVolume(vol) => {
                            self.config.left_channel.volume = vol;
                            self.router.set_left_volume(vol);
                            info!("Left volume: {}%", (vol * 100.0) as i32);
                            let _ = self.config.save();
                        }
                        tray::TrayCommand::SetRightVolume(vol) => {
                            self.config.right_channel.volume = vol;
                            self.router.set_right_volume(vol);
                            info!("Right volume: {}%", (vol * 100.0) as i32);
                            let _ = self.config.save();
                        }
                        tray::TrayCommand::SelectSourceDevice(device) => {
                            self.source_name = device.clone();
                            self.config.source_device = Some(device.clone());
                            self.router.stop();
                            if self.config.enabled {
                                if let Err(e) = self.router.start_loopback(&self.source_name, &self.target_name) {
                                    error!("Failed to start: {}", e);
                                } else {
                                    info!("Source changed to: {}", device);
                                }
                            }
                            let _ = self.config.save();
                        }
                        tray::TrayCommand::SelectTargetDevice(device) => {
                            self.target_name = device.clone();
                            self.config.target_device = Some(device.clone());
                            self.router.stop();
                            if self.config.enabled {
                                if let Err(e) = self.router.start_loopback(&self.source_name, &self.target_name) {
                                    error!("Failed to start: {}", e);
                                } else {
                                    info!("Target changed to: {}", device);
                                }
                            }
                            let _ = self.config.save();
                        }
                        tray::TrayCommand::TestMainLeft => {
                            if let Err(e) = self.router.play_test_tone_main(true, &self.source_name) {
                                error!("Test tone error: {}", e);
                            }
                        }
                        tray::TrayCommand::TestMainRight => {
                            if let Err(e) = self.router.play_test_tone_main(false, &self.source_name) {
                                error!("Test tone error: {}", e);
                            }
                        }
                        tray::TrayCommand::TestSubLeft => {
                            if let Err(e) = self.router.play_test_tone_sub(true) {
                                error!("Test tone error: {}", e);
                            }
                        }
                        tray::TrayCommand::TestSubRight => {
                            if let Err(e) = self.router.play_test_tone_sub(false) {
                                error!("Test tone error: {}", e);
                            }
                        }
                        tray::TrayCommand::Quit => {
                            info!("Quit requested");
                            self.router.stop();
                            let _ = self.config.save();
                            event_loop.exit();
                        }
                    }
                }
            }
        }
    }
}

fn print_help() {
    println!("tatsu-audioapp - Windows audio routing tool");
    println!();
    println!("USAGE:");
    println!("    tatsu-audioapp [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help       Show this help message");
    println!("    -v, --version    Show version");
    println!("    -l, --list       List available audio devices");
    println!("    -q, --quiet      Suppress startup messages");
    println!();
    println!("The application runs in the system tray. Right-click the icon for settings.");
}

fn print_version() {
    println!("tatsu-audioapp {}", env!("CARGO_PKG_VERSION"));
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    // Handle --help or -h
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return Ok(());
    }
    
    // Handle --version or -v
    if args.iter().any(|a| a == "-v" || a == "--version") {
        print_version();
        return Ok(());
    }
    
    let quiet = args.iter().any(|a| a == "-q" || a == "--quiet");
    let list_only = args.iter().any(|a| a == "-l" || a == "--list");

    // Initialize logging
    tracing_subscriber::fmt::init();
    info!("tatsu-audioapp starting...");

    // Load config
    let mut config = AppConfig::load().unwrap_or_else(|e| {
        warn!("Failed to load config: {}, using defaults", e);
        AppConfig::default()
    });
    info!("Config loaded: {:?}", config);

    // Initialize audio router
    let mut router = AudioRouter::new()?;

    // List available devices
    let output_devices = router.list_output_devices()?;
    let input_devices = router.list_input_devices()?;
    
    if !quiet || list_only {
        println!("\n=== Output Devices ===");
        for (i, device) in output_devices.iter().enumerate() {
            println!(
                "  [{}] {} ({} ch, {} Hz)",
                i, device.name, device.channels, device.sample_rate
            );
        }
        
        println!("\n=== Input Devices (for capture/loopback) ===");
        for (i, device) in input_devices.iter().enumerate() {
            println!(
                "  [{}] {} ({} ch, {} Hz)",
                i, device.name, device.channels, device.sample_rate
            );
        }
    }
    
    if list_only {
        return Ok(());
    }

    // Find source device - now we use output devices for loopback!
    // The source is the main speakers (output device) that we'll capture via WASAPI loopback
    let source_device = output_devices.iter()
        .find(|d| (d.name.contains("Speakers") || d.name.contains("Speaker")) && d.channels >= 4)
        .or_else(|| output_devices.iter().find(|d| d.name.contains("Speakers") || d.name.contains("Speaker")))
        .or_else(|| config.source_device.as_ref().and_then(|name| 
            output_devices.iter().find(|d| d.name.contains(name))
        ));
    
    // Find target device (2nd output)
    let target_device = output_devices.iter()
        .find(|d| d.name.contains("2nd output") || d.name.contains("HD Audio 2nd"))
        .or_else(|| config.target_device.as_ref().and_then(|name| 
            output_devices.iter().find(|d| d.name.contains(name))
        ));

    let (source_name, target_name) = match (source_device, target_device) {
        (Some(src), Some(tgt)) if src.name != tgt.name => {
            if !quiet {
                println!("\nSource (loopback): {} ({} ch)", src.name, src.channels);
                println!("Target (output): {}", tgt.name);
            }
            (src.name.clone(), tgt.name.clone())
        }
        (Some(_), Some(_)) => {
            error!("Source and target device are the same!");
            eprintln!("Error: Cannot route to the same device");
            config.save()?;
            return Ok(());
        }
        (None, _) => {
            error!("Could not find source device");
            eprintln!("Error: No suitable source device found");
            eprintln!("Please set source_device in config.toml");
            
            if let Some(first) = output_devices.first() {
                config.source_device = Some(first.name.clone());
            }
            config.save()?;
            return Ok(());
        }
        (_, None) => {
            error!("Could not find target device");
            eprintln!("Error: No suitable target device found");
            eprintln!("Please configure target_device in config.toml");
            config.save()?;
            return Ok(());
        }
    };

    // Update config
    config.source_device = Some(source_name.clone());
    config.target_device = Some(target_name.clone());

    // Apply config settings
    router.set_volume(config.volume);
    router.set_swap_channels(config.swap_channels);
    router.set_balance(config.balance);
    router.set_left_channel(&config.left_channel);
    router.set_right_channel(&config.right_channel);

    // Start routing if enabled (using WASAPI Loopback)
    if config.enabled {
        match router.start_loopback(&source_name, &target_name) {
            Ok(_) => {
                if !quiet {
                    println!("\nAudio routing started (WASAPI Loopback)");
                    println!("  Swap L/R: {}", config.swap_channels);
                    println!("  Volume: {}%", (config.volume * 100.0) as i32);
                    println!("  Balance: {}", format_balance(config.balance));
                }
            }
            Err(e) => {
                error!("Failed to start routing: {}", e);
                eprintln!("Error: Failed to start routing: {}", e);
            }
        }
    }

    // Set up tray icon
    let device_names: Vec<String> = output_devices.iter().map(|d| d.name.clone()).collect();
    let tray_manager = tray::TrayManager::new(
        &device_names,
        &device_names,
        config.enabled,
        config.swap_channels,
    )?;

    info!("Tray icon initialized, entering main loop");
    if !quiet {
        println!("\nRunning in system tray. Right-click the icon for settings.");
    }

    // Create app state
    let mut app = App {
        router,
        config,
        source_name,
        target_name,
        tray_manager: Some(tray_manager),
    };

    // Run winit event loop for Windows message pump
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop.run_app(&mut app)?;

    info!("tatsu-audioapp stopped");
    Ok(())
}

