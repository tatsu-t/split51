use anyhow::Result;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu, CheckMenuItem, MenuId};
use tray_icon::{TrayIcon, TrayIconBuilder, Icon};
use std::collections::HashMap;
use crate::config::ChannelSource;

pub enum TrayCommand {
    ToggleEnabled,
    ToggleSwapChannels,
    ToggleStartup,
    SetVolume(f32),
    SetBalance(f32),
    TestMainLeft,     // Test FL on main speakers
    TestMainRight,    // Test FR on main speakers
    TestSubLeft,      // Test L on 2nd output (routed)
    TestSubRight,     // Test R on 2nd output (routed)
    SetLeftSource(ChannelSource),
    SetRightSource(ChannelSource),
    ToggleLeftMute,
    ToggleRightMute,
    SetLeftVolume(f32),
    SetRightVolume(f32),
    SelectSourceDevice(String),
    SelectTargetDevice(String),
    Quit,
}

pub struct TrayManager {
    tray_icon: TrayIcon,
    toggle_item: MenuItem,
    swap_item: CheckMenuItem,
    startup_item: CheckMenuItem,
    left_mute_item: CheckMenuItem,
    right_mute_item: CheckMenuItem,
    volume_items: HashMap<MenuId, f32>,
    balance_items: HashMap<MenuId, f32>,
    left_volume_items: HashMap<MenuId, f32>,
    right_volume_items: HashMap<MenuId, f32>,
    source_device_items: HashMap<MenuId, String>,
    target_device_items: HashMap<MenuId, String>,
    source_menu_items: Vec<(MenuId, MenuItem, String)>,
    target_menu_items: Vec<(MenuId, MenuItem, String)>,
    toggle_id: MenuId,
    swap_id: MenuId,
    startup_id: MenuId,
    quit_id: MenuId,
    test_main_left_id: MenuId,
    test_main_right_id: MenuId,
    test_sub_left_id: MenuId,
    test_sub_right_id: MenuId,
    left_rl_id: MenuId,
    left_rr_id: MenuId,
    right_rl_id: MenuId,
    right_rr_id: MenuId,
    left_mute_id: MenuId,
    right_mute_id: MenuId,
}

impl TrayManager {
    pub fn new(
        source_devices: &[String],
        target_devices: &[String],
        current_source: Option<&str>,
        current_target: Option<&str>,
        current_volume: f32,
        current_balance: f32,
        current_left_source: ChannelSource,
        current_right_source: ChannelSource,
        current_left_volume: f32,
        current_right_volume: f32,
        left_muted: bool,
        right_muted: bool,
        enabled: bool,
        swap_channels: bool,
        startup_enabled: bool,
    ) -> Result<Self> {
        // Create menu items
        let toggle_text = if enabled { "Disable Routing" } else { "Enable Routing" };
        let toggle_item = MenuItem::new(toggle_text, true, None);

        // Swap channels checkbox
        let swap_item = CheckMenuItem::new("Swap L/R Channels", true, swap_channels, None);
        
        // Startup checkbox
        let startup_item = CheckMenuItem::new("Start with Windows", true, startup_enabled, None);

        // Source device submenu with checkmarks
        let source_submenu = Submenu::new("Source Device (Loopback)", true);
        let mut source_device_items = HashMap::new();
        let mut source_menu_items = Vec::new();
        for device in source_devices {
            let is_current = current_source.map(|s| s == device).unwrap_or(false);
            let label = if is_current { format!("[*] {}", device) } else { device.clone() };
            let item = MenuItem::new(&label, true, None);
            source_device_items.insert(item.id().clone(), device.clone());
            source_menu_items.push((item.id().clone(), item.clone(), device.clone()));
            source_submenu.append(&item)?;
        }

        // Target device submenu with checkmarks
        let target_submenu = Submenu::new("Target Device (Output)", true);
        let mut target_device_items = HashMap::new();
        let mut target_menu_items = Vec::new();
        for device in target_devices {
            let is_current = current_target.map(|t| t == device).unwrap_or(false);
            let label = if is_current { format!("[*] {}", device) } else { device.clone() };
            let item = MenuItem::new(&label, true, None);
            target_device_items.insert(item.id().clone(), device.clone());
            target_menu_items.push((item.id().clone(), item.clone(), device.clone()));
            target_submenu.append(&item)?;
        }

        // Master Volume submenu
        let volume_submenu = Submenu::new("Master Volume", true);
        let mut volume_items = HashMap::new();
        let current_vol_pct = (current_volume * 100.0).round() as i32;
        for v in [25, 50, 75, 100, 125, 150] {
            let is_current = v == current_vol_pct;
            let label = if is_current { format!("[*] {}%", v) } else { format!("{}%", v) };
            let item = MenuItem::new(&label, true, None);
            volume_items.insert(item.id().clone(), v as f32 / 100.0);
            volume_submenu.append(&item)?;
        }

        // Balance submenu
        let balance_submenu = Submenu::new("Balance", true);
        let mut balance_items = HashMap::new();
        let balance_values = [
            ("Full Left", -1.0),
            ("50% Left", -0.5),
            ("Center", 0.0),
            ("50% Right", 0.5),
            ("Full Right", 1.0),
        ];
        for (label, value) in balance_values {
            let is_current = (current_balance - value).abs() < 0.1;
            let text = if is_current { format!("[*] {}", label) } else { label.to_string() };
            let item = MenuItem::new(&text, true, None);
            balance_items.insert(item.id().clone(), value);
            balance_submenu.append(&item)?;
        }

        // Left Speaker submenu
        let left_submenu = Submenu::new("Left Speaker", true);
        let left_rl_current = matches!(current_left_source, ChannelSource::RL);
        let left_rl_label = if left_rl_current { "[*] Source: RL (Rear Left)" } else { "Source: RL (Rear Left)" };
        let left_rr_label = if !left_rl_current { "[*] Source: RR (Rear Right)" } else { "Source: RR (Rear Right)" };
        let left_rl = MenuItem::new(left_rl_label, true, None);
        let left_rr = MenuItem::new(left_rr_label, true, None);
        let left_mute = CheckMenuItem::new("Mute", true, left_muted, None);
        left_submenu.append(&left_rl)?;
        left_submenu.append(&left_rr)?;
        left_submenu.append(&PredefinedMenuItem::separator())?;
        left_submenu.append(&left_mute)?;
        
        // Left volume
        let left_vol_submenu = Submenu::new("Volume", true);
        let mut left_volume_items = HashMap::new();
        let current_left_vol_pct = (current_left_volume * 100.0).round() as i32;
        for v in [25, 50, 75, 100, 125, 150] {
            let is_current = v == current_left_vol_pct;
            let label = if is_current { format!("[*] {}%", v) } else { format!("{}%", v) };
            let item = MenuItem::new(&label, true, None);
            left_volume_items.insert(item.id().clone(), v as f32 / 100.0);
            left_vol_submenu.append(&item)?;
        }
        left_submenu.append(&left_vol_submenu)?;

        // Right Speaker submenu
        let right_submenu = Submenu::new("Right Speaker", true);
        let right_rr_current = matches!(current_right_source, ChannelSource::RR);
        let right_rl_label = if !right_rr_current { "[*] Source: RL (Rear Left)" } else { "Source: RL (Rear Left)" };
        let right_rr_label = if right_rr_current { "[*] Source: RR (Rear Right)" } else { "Source: RR (Rear Right)" };
        let right_rl = MenuItem::new(right_rl_label, true, None);
        let right_rr = MenuItem::new(right_rr_label, true, None);
        let right_mute = CheckMenuItem::new("Mute", true, right_muted, None);
        right_submenu.append(&right_rl)?;
        right_submenu.append(&right_rr)?;
        right_submenu.append(&PredefinedMenuItem::separator())?;
        right_submenu.append(&right_mute)?;

        // Right volume
        let right_vol_submenu = Submenu::new("Volume", true);
        let mut right_volume_items = HashMap::new();
        let current_right_vol_pct = (current_right_volume * 100.0).round() as i32;
        for v in [25, 50, 75, 100, 125, 150] {
            let is_current = v == current_right_vol_pct;
            let label = if is_current { format!("[*] {}%", v) } else { format!("{}%", v) };
            let item = MenuItem::new(&label, true, None);
            right_volume_items.insert(item.id().clone(), v as f32 / 100.0);
            right_vol_submenu.append(&item)?;
        }
        right_submenu.append(&right_vol_submenu)?;

        // Speaker test submenu
        let test_submenu = Submenu::new("Speaker Test", true);
        let test_main_left = MenuItem::new("Main Left (FL)", true, None);
        let test_main_right = MenuItem::new("Main Right (FR)", true, None);
        let test_sub_left = MenuItem::new("Sub Left (L)", true, None);
        let test_sub_right = MenuItem::new("Sub Right (R)", true, None);
        test_submenu.append(&test_main_left)?;
        test_submenu.append(&test_main_right)?;
        test_submenu.append(&PredefinedMenuItem::separator())?;
        test_submenu.append(&test_sub_left)?;
        test_submenu.append(&test_sub_right)?;

        let quit_item = MenuItem::new("Quit", true, None);

        // Store IDs for event handling
        let toggle_id = toggle_item.id().clone();
        let swap_id = swap_item.id().clone();
        let startup_id = startup_item.id().clone();
        let quit_id = quit_item.id().clone();
        let test_main_left_id = test_main_left.id().clone();
        let test_main_right_id = test_main_right.id().clone();
        let test_sub_left_id = test_sub_left.id().clone();
        let test_sub_right_id = test_sub_right.id().clone();
        let left_rl_id = left_rl.id().clone();
        let left_rr_id = left_rr.id().clone();
        let right_rl_id = right_rl.id().clone();
        let right_rr_id = right_rr.id().clone();
        let left_mute_id = left_mute.id().clone();
        let right_mute_id = right_mute.id().clone();

        // Build menu
        let menu = Menu::new();
        menu.append(&toggle_item)?;
        menu.append(&swap_item)?;
        menu.append(&startup_item)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&source_submenu)?;
        menu.append(&target_submenu)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&volume_submenu)?;
        menu.append(&balance_submenu)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&left_submenu)?;
        menu.append(&right_submenu)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&test_submenu)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&quit_item)?;

        // Create tray icon
        let icon = create_default_icon()?;
        let tray_icon = TrayIconBuilder::new()
            .with_tooltip("tatsu-audioapp - Audio Router")
            .with_icon(icon)
            .with_menu(Box::new(menu.clone()))
            .build()?;

        Ok(Self {
            tray_icon,
            toggle_item,
            swap_item,
            startup_item,
            left_mute_item: left_mute,
            right_mute_item: right_mute,
            volume_items,
            balance_items,
            left_volume_items,
            right_volume_items,
            source_device_items,
            target_device_items,
            source_menu_items,
            target_menu_items,
            toggle_id,
            swap_id,
            startup_id,
            quit_id,
            test_main_left_id,
            test_main_right_id,
            test_sub_left_id,
            test_sub_right_id,
            left_rl_id,
            left_rr_id,
            right_rl_id,
            right_rr_id,
            left_mute_id,
            right_mute_id,
        })
    }

    /// Update tray icon and tooltip based on enabled state
    pub fn set_enabled(&mut self, enabled: bool) {
        let text = if enabled { "Disable Routing" } else { "Enable Routing" };
        self.toggle_item.set_text(text);
        
        let tooltip = if enabled {
            "tatsu-audioapp - Routing Active"
        } else {
            "tatsu-audioapp - Routing Disabled"
        };
        self.tray_icon.set_tooltip(Some(tooltip)).ok();
        
        // Change icon color based on state
        if let Ok(icon) = if enabled { create_enabled_icon() } else { create_disabled_icon() } {
            self.tray_icon.set_icon(Some(icon)).ok();
        }
    }

    /// Update startup checkbox
    pub fn set_startup(&mut self, enabled: bool) {
        self.startup_item.set_checked(enabled);
    }

    /// Update swap checkbox
    pub fn set_swap(&mut self, swap: bool) {
        self.swap_item.set_checked(swap);
    }

    /// Update mute checkboxes
    pub fn set_left_mute(&mut self, muted: bool) {
        self.left_mute_item.set_checked(muted);
    }

    pub fn set_right_mute(&mut self, muted: bool) {
        self.right_mute_item.set_checked(muted);
    }

    /// Update source device menu checkmarks
    pub fn set_current_source(&mut self, device: Option<&str>) {
        for (_, item, name) in &self.source_menu_items {
            let is_current = device.map(|d| d == name).unwrap_or(false);
            let label = if is_current { format!("[*] {}", name) } else { name.clone() };
            item.set_text(&label);
        }
    }

    /// Update target device menu checkmarks
    pub fn set_current_target(&mut self, device: Option<&str>) {
        for (_, item, name) in &self.target_menu_items {
            let is_current = device.map(|d| d == name).unwrap_or(false);
            let label = if is_current { format!("[*] {}", name) } else { name.clone() };
            item.set_text(&label);
        }
    }

    pub fn handle_menu_event(&self, event: &MenuEvent) -> Option<TrayCommand> {
        if event.id == self.toggle_id {
            Some(TrayCommand::ToggleEnabled)
        } else if event.id == self.swap_id {
            Some(TrayCommand::ToggleSwapChannels)
        } else if event.id == self.startup_id {
            Some(TrayCommand::ToggleStartup)
        } else if event.id == self.quit_id {
            Some(TrayCommand::Quit)
        } else if event.id == self.test_main_left_id {
            Some(TrayCommand::TestMainLeft)
        } else if event.id == self.test_main_right_id {
            Some(TrayCommand::TestMainRight)
        } else if event.id == self.test_sub_left_id {
            Some(TrayCommand::TestSubLeft)
        } else if event.id == self.test_sub_right_id {
            Some(TrayCommand::TestSubRight)
        } else if event.id == self.left_rl_id {
            Some(TrayCommand::SetLeftSource(ChannelSource::RL))
        } else if event.id == self.left_rr_id {
            Some(TrayCommand::SetLeftSource(ChannelSource::RR))
        } else if event.id == self.right_rl_id {
            Some(TrayCommand::SetRightSource(ChannelSource::RL))
        } else if event.id == self.right_rr_id {
            Some(TrayCommand::SetRightSource(ChannelSource::RR))
        } else if event.id == self.left_mute_id {
            Some(TrayCommand::ToggleLeftMute)
        } else if event.id == self.right_mute_id {
            Some(TrayCommand::ToggleRightMute)
        } else if let Some(&vol) = self.volume_items.get(&event.id) {
            Some(TrayCommand::SetVolume(vol))
        } else if let Some(&bal) = self.balance_items.get(&event.id) {
            Some(TrayCommand::SetBalance(bal))
        } else if let Some(&vol) = self.left_volume_items.get(&event.id) {
            Some(TrayCommand::SetLeftVolume(vol))
        } else if let Some(&vol) = self.right_volume_items.get(&event.id) {
            Some(TrayCommand::SetRightVolume(vol))
        } else if let Some(device) = self.source_device_items.get(&event.id) {
            Some(TrayCommand::SelectSourceDevice(device.clone()))
        } else if let Some(device) = self.target_device_items.get(&event.id) {
            Some(TrayCommand::SelectTargetDevice(device.clone()))
        } else {
            None
        }
    }
}

fn create_default_icon() -> Result<Icon> {
    create_enabled_icon()
}

fn create_enabled_icon() -> Result<Icon> {
    // Create a simple 16x16 RGBA icon (green - active)
    let size = 16;
    let mut rgba = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            // Create a simple speaker-like pattern
            let in_speaker = (x >= 2 && x <= 6 && y >= 4 && y <= 11) ||
                            (x >= 6 && x <= 10 && y >= 2 && y <= 13) ||
                            (x >= 10 && x <= 13 && (y == 4 || y == 7 || y == 10));
            if in_speaker {
                rgba[idx] = 50;      // R
                rgba[idx + 1] = 200; // G (brighter green for enabled)
                rgba[idx + 2] = 80;  // B
                rgba[idx + 3] = 255; // A
            } else {
                rgba[idx + 3] = 0; // Transparent
            }
        }
    }
    Icon::from_rgba(rgba, size as u32, size as u32).map_err(|e| anyhow::anyhow!("Icon error: {}", e))
}

fn create_disabled_icon() -> Result<Icon> {
    // Create a simple 16x16 RGBA icon (gray - disabled)
    let size = 16;
    let mut rgba = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            // Create a simple speaker-like pattern
            let in_speaker = (x >= 2 && x <= 6 && y >= 4 && y <= 11) ||
                            (x >= 6 && x <= 10 && y >= 2 && y <= 13) ||
                            (x >= 10 && x <= 13 && (y == 4 || y == 7 || y == 10));
            if in_speaker {
                rgba[idx] = 120;     // R (gray)
                rgba[idx + 1] = 120; // G
                rgba[idx + 2] = 120; // B
                rgba[idx + 3] = 255; // A
            } else {
                rgba[idx + 3] = 0; // Transparent
            }
        }
    }
    Icon::from_rgba(rgba, size as u32, size as u32).map_err(|e| anyhow::anyhow!("Icon error: {}", e))
}
