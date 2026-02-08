mod loopback;

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use parking_lot::RwLock;
use ringbuf::{HeapRb, traits::{Consumer, Split}};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tracing::{info, error};
use crate::config::{ChannelConfig, ChannelSource};

pub use loopback::LoopbackCapture;

pub struct AudioDevice {
    pub name: String,
    pub channels: u16,
    pub sample_rate: u32,
}

/// Minimal struct for playing test tones from a background thread
pub struct TestTonePlayer {
    host: cpal::Host,
    swap_channels: Arc<RwLock<bool>>,
    target_device_name: Option<String>,
}

impl TestTonePlayer {
    fn find_output_device(&self, name: &str) -> Option<Device> {
        self.host.output_devices().ok()?.find(|d| {
            d.name().map(|n| n.contains(name)).unwrap_or(false)
        })
    }

    pub fn play_test_tone_sub(&self, left_channel: bool) -> Result<()> {
        let target_name = self.target_device_name.as_ref()
            .context("No target device configured. Start routing first.")?;
        
        let swap = *self.swap_channels.read();
        let actual_left = if swap { !left_channel } else { left_channel };
        
        self.play_tone_on_device(target_name, actual_left, "Sub", left_channel)
    }

    pub fn play_test_tone_main(&self, left_channel: bool, source_name: &str) -> Result<()> {
        self.play_tone_on_device(source_name, left_channel, "Main", left_channel)
    }

    fn play_tone_on_device(&self, device_name: &str, actual_left_channel: bool, label: &str, display_left: bool) -> Result<()> {
        let output_device = self.find_output_device(device_name)
            .context(format!("Output device not found: {}", device_name))?;

        let output_supported = output_device.default_output_config()?;
        let sample_rate = output_supported.sample_rate().0 as f32;
        
        let output_config = StreamConfig {
            channels: 2,
            sample_rate: cpal::SampleRate(sample_rate as u32),
            buffer_size: cpal::BufferSize::Default,
        };

        let freq = 440.0;
        let duration_samples = (sample_rate * 0.5) as usize;
        let samples_total = std::sync::Arc::new(AtomicU32::new(0));
        let samples_total_clone = samples_total.clone();

        let stream = output_device.build_output_stream(
            &output_config,
            move |data: &mut [f32], _: &_| {
                for frame in data.chunks_mut(2) {
                    let current = samples_total_clone.fetch_add(1, Ordering::Relaxed) as usize;
                    if current >= duration_samples {
                        frame[0] = 0.0;
                        frame[1] = 0.0;
                    } else {
                        let t = current as f32 / sample_rate;
                        let sample = (t * freq * 2.0 * std::f32::consts::PI).sin() * 0.5;
                        
                        if actual_left_channel {
                            frame[0] = sample;
                            frame[1] = 0.0;
                        } else {
                            frame[0] = 0.0;
                            frame[1] = sample;
                        }
                    }
                }
            },
            move |err| error!("Test tone error: {}", err),
            None,
        )?;

        stream.play()?;
        
        let side = if display_left { "LEFT" } else { "RIGHT" };
        info!("Playing test tone on {} {} for 0.6 sec", label, side);
        
        std::thread::sleep(std::time::Duration::from_millis(600));
        drop(stream);
        
        Ok(())
    }
}

#[derive(Clone)]
pub struct ChannelSettings {
    pub source: ChannelSource,
    pub volume: f32,
    pub muted: bool,
}

impl Default for ChannelSettings {
    fn default() -> Self {
        Self {
            source: ChannelSource::RL,
            volume: 1.0,
            muted: false,
        }
    }
}

pub struct AudioRouter {
    host: cpal::Host,
    output_stream: Option<Stream>,
    loopback: Option<LoopbackCapture>,
    running: Arc<AtomicBool>,
    current_channels: Arc<AtomicU32>,
    volume: Arc<RwLock<f32>>,
    swap_channels: Arc<RwLock<bool>>,
    balance: Arc<RwLock<f32>>,
    left_channel: Arc<RwLock<ChannelSettings>>,
    right_channel: Arc<RwLock<ChannelSettings>>,
    target_device_name: Option<String>,
}

impl AudioRouter {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        Ok(Self {
            host,
            output_stream: None,
            loopback: None,
            running: Arc::new(AtomicBool::new(false)),
            current_channels: Arc::new(AtomicU32::new(2)),
            volume: Arc::new(RwLock::new(1.0)),
            swap_channels: Arc::new(RwLock::new(false)),
            balance: Arc::new(RwLock::new(0.0)),
            left_channel: Arc::new(RwLock::new(ChannelSettings::default())),
            right_channel: Arc::new(RwLock::new(ChannelSettings {
                source: ChannelSource::RR,
                volume: 1.0,
                muted: false,
            })),
            target_device_name: None,
        })
    }

    pub fn list_output_devices(&self) -> Result<Vec<AudioDevice>> {
        let mut devices = Vec::new();
        for device in self.host.output_devices().context("Failed to get output devices")? {
            if let Ok(name) = device.name() {
                if let Ok(config) = device.default_output_config() {
                    devices.push(AudioDevice {
                        name,
                        channels: config.channels(),
                        sample_rate: config.sample_rate().0,
                    });
                }
            }
        }
        Ok(devices)
    }

    pub fn list_input_devices(&self) -> Result<Vec<AudioDevice>> {
        let mut devices = Vec::new();
        for device in self.host.input_devices().context("Failed to get input devices")? {
            if let Ok(name) = device.name() {
                if let Ok(config) = device.default_input_config() {
                    devices.push(AudioDevice {
                        name,
                        channels: config.channels(),
                        sample_rate: config.sample_rate().0,
                    });
                }
            }
        }
        Ok(devices)
    }

    pub fn set_volume(&self, volume: f32) {
        *self.volume.write() = volume;
    }

    pub fn set_swap_channels(&self, swap: bool) {
        *self.swap_channels.write() = swap;
    }

    pub fn set_balance(&self, balance: f32) {
        *self.balance.write() = balance.clamp(-1.0, 1.0);
    }

    pub fn set_left_channel(&self, config: &ChannelConfig) {
        let mut ch = self.left_channel.write();
        ch.source = config.source;
        ch.volume = config.volume;
        ch.muted = config.muted;
    }

    pub fn set_right_channel(&self, config: &ChannelConfig) {
        let mut ch = self.right_channel.write();
        ch.source = config.source;
        ch.volume = config.volume;
        ch.muted = config.muted;
    }

    /// Clone minimal state needed for test tones (thread-safe)
    pub fn clone_for_test(&self) -> TestTonePlayer {
        TestTonePlayer {
            host: cpal::default_host(),
            swap_channels: self.swap_channels.clone(),
            target_device_name: self.target_device_name.clone(),
        }
    }

    pub fn set_left_source(&self, source: ChannelSource) {
        self.left_channel.write().source = source;
    }

    pub fn set_right_source(&self, source: ChannelSource) {
        self.right_channel.write().source = source;
    }

    pub fn set_left_muted(&self, muted: bool) {
        self.left_channel.write().muted = muted;
    }

    pub fn set_right_muted(&self, muted: bool) {
        self.right_channel.write().muted = muted;
    }

    pub fn set_left_volume(&self, volume: f32) {
        self.left_channel.write().volume = volume;
    }

    pub fn set_right_volume(&self, volume: f32) {
        self.right_channel.write().volume = volume;
    }

    #[allow(dead_code)]
    pub fn get_current_channels(&self) -> u32 {
        self.current_channels.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    fn find_output_device(&self, name: &str) -> Option<Device> {
        self.host.output_devices().ok()?.find(|d| {
            d.name().map(|n| n.contains(name)).unwrap_or(false)
        })
    }

    /// Start audio routing using WASAPI Loopback
    /// source_name: Output device to capture from (e.g., "Speakers")
    /// target_name: Output device to play to (e.g., "2nd output")
    pub fn start_loopback(&mut self, source_name: &str, target_name: &str) -> Result<()> {
        self.stop();
        
        info!("Starting loopback routing: {} -> {}", source_name, target_name);

        // Store target device name for test tones
        self.target_device_name = Some(target_name.to_string());

        // Find output device for playback
        let output_device = self.find_output_device(target_name)
            .context(format!("Output device not found: {}", target_name))?;

        info!("Output device: {}", output_device.name()?);

        // Get output config
        let output_supported = output_device.default_output_config()?;
        let sample_rate = output_supported.sample_rate();
        
        let output_config = StreamConfig {
            channels: 2, // Always output stereo
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        // Create ring buffer - 100ms buffer for low latency
        let buffer_samples = (sample_rate.0 as f32 * 0.1) as usize * 2; // 100ms stereo
        let ring_buffer = HeapRb::<f32>::new(buffer_samples);
        let (producer, mut consumer) = ring_buffer.split();

        self.running.store(true, Ordering::Relaxed);

        // Start loopback capture thread
        let mut loopback = LoopbackCapture::new();
        loopback.start(
            source_name,
            sample_rate.0,  // Pass target sample rate for resampling
            producer,
            self.current_channels.clone(),
            self.volume.clone(),
            self.swap_channels.clone(),
            self.balance.clone(),
            self.left_channel.clone(),
            self.right_channel.clone(),
        )?;

        // Build output stream
        let output_stream = output_device.build_output_stream(
            &output_config,
            move |data: &mut [f32], _: &_| {
                for sample in data.iter_mut() {
                    *sample = consumer.try_pop().unwrap_or(0.0);
                }
            },
            move |err| error!("Output stream error: {}", err),
            None,
        )?;

        output_stream.play()?;

        self.output_stream = Some(output_stream);
        self.loopback = Some(loopback);

        info!("Loopback routing started successfully");
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        
        if let Some(mut loopback) = self.loopback.take() {
            loopback.stop();
        }
        if let Some(stream) = self.output_stream.take() {
            drop(stream);
        }
        
        info!("Audio routing stopped");
    }
}
