//! WASAPI Loopback capture implementation
//! Captures audio from output devices (e.g., Speakers) using Windows Audio Session API

use anyhow::{Context, Result};
use parking_lot::RwLock;
use ringbuf::traits::Producer;
use rubato::{SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction, Resampler};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;
use tracing::{info, error, warn};
use crate::config::ChannelSource;
use crate::dsp::{DspChain, SharedLevels};
use super::ChannelSettings;

use windows::core::PCWSTR;
use windows::Win32::Foundation::WAIT_OBJECT_0;
use windows::Win32::Media::Audio::*;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::System::Com::*;
use windows::Win32::System::Threading::*;

/// DSP configuration for loopback capture
#[derive(Clone)]
pub struct DspConfig {
    pub delay_ms: Arc<RwLock<f32>>,
    pub eq_enabled: Arc<RwLock<bool>>,
    pub eq_low: Arc<RwLock<f32>>,
    pub eq_mid: Arc<RwLock<f32>>,
    pub eq_high: Arc<RwLock<f32>>,
    pub upmix_enabled: Arc<RwLock<bool>>,
    pub upmix_strength: Arc<RwLock<f32>>,
    pub shared_levels: Arc<SharedLevels>,
    /// Master volume from source device (0.0-1.0)
    pub master_volume: Arc<RwLock<f32>>,
    pub sync_master_volume: Arc<RwLock<bool>>,
    /// Master mute state from source device
    pub master_muted: Arc<RwLock<bool>>,
}

impl DspConfig {
    pub fn new() -> Self {
        Self {
            delay_ms: Arc::new(RwLock::new(0.0)),
            eq_enabled: Arc::new(RwLock::new(false)),
            eq_low: Arc::new(RwLock::new(0.0)),
            eq_mid: Arc::new(RwLock::new(0.0)),
            eq_high: Arc::new(RwLock::new(0.0)),
            upmix_enabled: Arc::new(RwLock::new(false)),
            upmix_strength: Arc::new(RwLock::new(0.5)),
            shared_levels: SharedLevels::new(),
            master_volume: Arc::new(RwLock::new(1.0)),
            sync_master_volume: Arc::new(RwLock::new(true)),
            master_muted: Arc::new(RwLock::new(false)),
        }
    }
}

pub struct LoopbackCapture {
    running: Arc<AtomicBool>,
    capture_thread: Option<thread::JoinHandle<()>>,
}

impl LoopbackCapture {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            capture_thread: None,
        }
    }

    pub fn start<P: Producer<Item = f32> + Send + 'static>(
        &mut self,
        device_name: &str,
        target_sample_rate: u32,
        mut producer: P,
        current_channels: Arc<AtomicU32>,
        volume: Arc<RwLock<f32>>,
        swap_channels: Arc<RwLock<bool>>,
        balance: Arc<RwLock<f32>>,
        left_channel: Arc<RwLock<ChannelSettings>>,
        right_channel: Arc<RwLock<ChannelSettings>>,
        dsp_config: DspConfig,
    ) -> Result<()> {
        self.stop();

        let running = self.running.clone();
        running.store(true, Ordering::Relaxed);

        let device_name = device_name.to_string();

        let handle = thread::spawn(move || {
            if let Err(e) = capture_loop(
                &device_name,
                target_sample_rate,
                &mut producer,
                &running,
                &current_channels,
                &volume,
                &swap_channels,
                &balance,
                &left_channel,
                &right_channel,
                &dsp_config,
            ) {
                error!("Loopback capture error: {}", e);
            }
            info!("Loopback capture thread stopped");
        });

        self.capture_thread = Some(handle);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.capture_thread.take() {
            let _ = handle.join();
        }
    }
}

fn find_device_by_name(name: &str) -> Result<IMMDevice> {
    unsafe {
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(
            &MMDeviceEnumerator,
            None,
            CLSCTX_ALL,
        )?;

        let collection = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
        let count = collection.GetCount()?;
        
        // Collect all device IDs and find best match
        let name_lower = name.to_lowercase();
        
        for i in 0..count {
            if let Ok(device) = collection.Item(i) {
                if let Ok(id_ptr) = device.GetId() {
                    let id = id_ptr.to_string()?;
                    let id_lower = id.to_lowercase();
                    
                    // Check if device ID contains key parts of the name
                    // cpal names usually contain the friendly name
                    let name_parts: Vec<&str> = name_lower.split(&[' ', '(', ')', '-'][..])
                        .filter(|s| s.len() > 2)
                        .collect();
                    
                    let matches = name_parts.iter().any(|part| id_lower.contains(part));
                    if matches {
                        info!("Found device: {} (ID contains match)", id);
                        return Ok(device);
                    }
                }
            }
        }

        // Fallback: try to match by device ID
        for i in 0..count {
            if let Ok(device) = collection.Item(i) {
                let id = device.GetId()?.to_string()?;
                
                // cpal device names contain the Windows friendly name
                // Match if the ID contains keywords from the search name
                if id.to_lowercase().contains(&name.to_lowercase()) 
                    || name.to_lowercase().contains("speakers")
                    || name.to_lowercase().contains("speaker") {
                    // Check if this might be our target by examining format
                    let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
                    let format_ptr = client.GetMixFormat()?;
                    let format = *format_ptr;
                    let num_channels = format.nChannels;
                    CoTaskMemFree(Some(format_ptr as *const _ as *const _));
                    
                    // If looking for Speakers with 4ch, prioritize that
                    if name.contains("4 ch") && num_channels >= 4 {
                        return Ok(device);
                    }
                    if name.contains("2 ch") && num_channels == 2 {
                        return Ok(device);
                    }
                }
            }
        }

        // Fallback: try to match by index based on device order
        // The order in WASAPI should match cpal's order
        for i in 0..count {
            if let Ok(device) = collection.Item(i) {
                let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
                let format_ptr = client.GetMixFormat()?;
                let format = *format_ptr;
                let num_channels = format.nChannels;
                CoTaskMemFree(Some(format_ptr as *const _ as *const _));
                
                // Match by channel count as hint
                if name.contains("Speakers") && num_channels >= 4 {
                    info!("Found device by channel count: {} channels", num_channels);
                    return Ok(device);
                }
                if (name.contains("2nd") || name.contains("HD Audio 2nd")) && num_channels == 2 {
                    info!("Found 2nd output device");
                    return Ok(device);
                }
            }
        }

        // Last resort: return first device
        if count > 0 {
            return Ok(collection.Item(0)?);
        }

        anyhow::bail!("Device not found: {}", name)
    }
}

fn capture_loop<P: Producer<Item = f32>>(
    device_name: &str,
    target_sample_rate: u32,
    producer: &mut P,
    running: &AtomicBool,
    current_channels: &AtomicU32,
    volume: &RwLock<f32>,
    swap_channels: &RwLock<bool>,
    balance: &RwLock<f32>,
    left_channel: &RwLock<ChannelSettings>,
    right_channel: &RwLock<ChannelSettings>,
    dsp_config: &DspConfig,
) -> Result<()> {
    // Track buffer overflow warnings (only log once per 1000 drops)
    let mut overflow_counter: u32 = 0;
    
    unsafe {
        // Initialize COM for this thread
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .context("Failed to initialize COM")?;

        let device = find_device_by_name(device_name)?;
        info!("Found loopback device: {}", device_name);

        let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
        
        // Get endpoint volume control for master volume sync
        let endpoint_volume: Option<IAudioEndpointVolume> = 
            device.Activate(CLSCTX_ALL, None).ok();
        
        // Get mix format
        let format_ptr = client.GetMixFormat()?;
        let format = *format_ptr;
        let channels = format.nChannels;
        let sample_rate = format.nSamplesPerSec;
        let bits_per_sample = format.wBitsPerSample;
        let block_align = format.nBlockAlign;
        
        current_channels.store(channels as u32, Ordering::Relaxed);
        info!("Loopback format: {} ch, {} Hz, {} bits", channels, sample_rate, bits_per_sample);
        info!("Target sample rate: {} Hz", target_sample_rate);

        // Initialize for loopback capture
        // AUDCLNT_STREAMFLAGS_LOOPBACK = 0x00020000
        const AUDCLNT_STREAMFLAGS_LOOPBACK: u32 = 0x00020000;
        const AUDCLNT_STREAMFLAGS_EVENTCALLBACK: u32 = 0x00040000;
        
        // 20ms buffer for low latency (200000 * 100ns = 20ms)
        let buffer_duration = 200_000i64;
        
        client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK | AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
            buffer_duration,
            0,
            format_ptr,
            None,
        )?;

        // Set up event handle for buffer notifications
        let event = CreateEventW(None, false, false, PCWSTR::null())?;
        client.SetEventHandle(event)?;

        let capture_client: IAudioCaptureClient = client.GetService()?;

        // Initialize resampler if sample rates differ
        let needs_resample = sample_rate != target_sample_rate;
        let mut resampler: Option<SincFixedIn<f32>> = if needs_resample {
            let params = SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            };
            let resample_ratio = target_sample_rate as f64 / sample_rate as f64;
            info!("Resampler initialized: {} Hz -> {} Hz (ratio: {:.4})", sample_rate, target_sample_rate, resample_ratio);
            Some(SincFixedIn::<f32>::new(
                resample_ratio,
                2.0,  // max relative ratio
                params,
                1024, // chunk size
                2,    // 2 channels (stereo output)
            )?)
        } else {
            None
        };

        // Buffers for resampling
        let mut resample_input: Vec<Vec<f32>> = vec![Vec::new(); 2];

        // Initialize DSP chain
        let mut dsp_chain = DspChain::new(target_sample_rate, dsp_config.shared_levels.clone());
        
        // Counter for master volume updates (every ~100ms instead of every loop)
        let mut master_vol_counter: u32 = 0;

        client.Start()?;
        info!("Loopback capture started");

        while running.load(Ordering::Relaxed) {
            // Update DSP settings from config
            let delay = *dsp_config.delay_ms.read();
            if (delay - dsp_chain.delay_ms).abs() > 0.1 {
                dsp_chain.set_delay_ms(delay);
            }
            dsp_chain.eq_enabled = *dsp_config.eq_enabled.read();
            if dsp_chain.eq_enabled {
                dsp_chain.set_eq(
                    *dsp_config.eq_low.read(),
                    *dsp_config.eq_mid.read(),
                    *dsp_config.eq_high.read(),
                );
            }
            dsp_chain.upmix_enabled = *dsp_config.upmix_enabled.read();
            dsp_chain.upmixer.set_strength(*dsp_config.upmix_strength.read());
            
            // Update master volume and mute state from source device (every ~100ms)
            master_vol_counter += 1;
            if master_vol_counter >= 5 {  // ~100ms at 20ms buffer
                master_vol_counter = 0;
                let sync_master = *dsp_config.sync_master_volume.read();
                if sync_master {
                    if let Some(ref ep_vol) = endpoint_volume {
                        if let Ok(master_vol) = ep_vol.GetMasterVolumeLevelScalar() {
                            *dsp_config.master_volume.write() = master_vol;
                        }
                        if let Ok(muted) = ep_vol.GetMute() {
                            *dsp_config.master_muted.write() = muted.as_bool();
                        }
                    }
                }
            }

            // Wait for buffer event
            let wait_result = WaitForSingleObject(event, 100);
            if wait_result != WAIT_OBJECT_0 {
                continue;
            }

            loop {
                let mut buffer_ptr: *mut u8 = ptr::null_mut();
                let mut frames_available: u32 = 0;
                let mut flags: u32 = 0;

                let hr = capture_client.GetBuffer(
                    &mut buffer_ptr,
                    &mut frames_available,
                    &mut flags,
                    None,
                    None,
                );

                if hr.is_err() || frames_available == 0 {
                    break;
                }

                // Process audio data
                let vol = *volume.read();
                let swap = *swap_channels.read();
                let bal = *balance.read();
                let left_ch = left_channel.read().clone();
                let right_ch = right_channel.read().clone();
                let master_vol = *dsp_config.master_volume.read();
                let master_muted = *dsp_config.master_muted.read();
                let sync_master = *dsp_config.sync_master_volume.read();

                // Convert buffer to f32 samples
                let bytes_per_sample = (bits_per_sample / 8) as usize;
                let data_slice = std::slice::from_raw_parts(
                    buffer_ptr,
                    frames_available as usize * block_align as usize,
                );

                let samples = bytes_to_f32(data_slice, bytes_per_sample);
                // Apply master volume and mute if sync enabled
                let effective_vol = if sync_master {
                    if master_muted { 0.0 } else { vol * master_vol }
                } else { 
                    vol 
                };
                let stereo_output = process_channels(&samples, channels, effective_vol, swap, bal, &left_ch, &right_ch, &mut dsp_chain);

                // Apply resampling if needed
                if let Some(ref mut rs) = resampler {
                    // Split stereo into separate channels
                    for frame in stereo_output.chunks(2) {
                        if frame.len() == 2 {
                            resample_input[0].push(frame[0]);
                            resample_input[1].push(frame[1]);
                        }
                    }

                    // Process when we have enough samples
                    let chunk_size = rs.input_frames_next();
                    while resample_input[0].len() >= chunk_size {
                        // Take chunk_size samples from each channel
                        let left_chunk: Vec<f32> = resample_input[0].drain(..chunk_size).collect();
                        let right_chunk: Vec<f32> = resample_input[1].drain(..chunk_size).collect();
                        
                        let input_chunk = vec![left_chunk, right_chunk];
                        
                        if let Ok(resampled) = rs.process(&input_chunk, None) {
                            // Apply DSP and push to producer
                            let frames = resampled[0].len();
                            for i in 0..frames {
                                let (l, r) = dsp_chain.process(resampled[0][i], resampled[1][i]);
                                if producer.try_push(l).is_err() {
                                    overflow_counter += 1;
                                    if overflow_counter == 1 || overflow_counter % 10000 == 0 {
                                        warn!("Buffer overflow: {} samples dropped (output not consuming fast enough)", overflow_counter);
                                    }
                                }
                                if producer.try_push(r).is_err() {
                                    overflow_counter += 1;
                                }
                            }
                        }
                    }
                } else {
                    // No resampling needed, apply DSP and push directly
                    for frame in stereo_output.chunks(2) {
                        if frame.len() == 2 {
                            let (l, r) = dsp_chain.process(frame[0], frame[1]);
                            if producer.try_push(l).is_err() {
                                overflow_counter += 1;
                                if overflow_counter == 1 || overflow_counter % 10000 == 0 {
                                    warn!("Buffer overflow: {} samples dropped", overflow_counter);
                                }
                            }
                            let _ = producer.try_push(r);
                        }
                    }
                }

                capture_client.ReleaseBuffer(frames_available)?;
            }
        }

        client.Stop()?;
        let _ = windows::Win32::Foundation::CloseHandle(event);
        CoTaskMemFree(Some(format_ptr as *const _ as *const _));
        CoUninitialize();

        Ok(())
    }
}

fn bytes_to_f32(data: &[u8], bytes_per_sample: usize) -> Vec<f32> {
    match bytes_per_sample {
        4 => {
            // 32-bit float
            data.chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect()
        }
        2 => {
            // 16-bit int
            data.chunks_exact(2)
                .map(|b| {
                    let sample = i16::from_le_bytes([b[0], b[1]]);
                    sample as f32 / 32768.0
                })
                .collect()
        }
        3 => {
            // 24-bit int
            data.chunks_exact(3)
                .map(|b| {
                    let sample = ((b[0] as i32) | ((b[1] as i32) << 8) | ((b[2] as i32) << 16)) << 8 >> 8;
                    sample as f32 / 8388608.0
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Extract channels from multichannel audio with per-channel control
/// Balance: -1.0 = full left, 0.0 = center, 1.0 = full right
fn process_channels(
    input: &[f32], 
    channels: u16, 
    volume: f32, 
    swap: bool, 
    balance: f32,
    left_ch: &ChannelSettings,
    right_ch: &ChannelSettings,
    dsp: &mut DspChain,
) -> Vec<f32> {
    if input.is_empty() || channels == 0 {
        return Vec::new();
    }
    
    let frames = input.len() / channels as usize;
    let mut output = Vec::with_capacity(frames * 2);

    // Calculate balance multipliers
    let left_mult = if balance > 0.0 { 1.0 - balance } else { 1.0 };
    let right_mult = if balance < 0.0 { 1.0 + balance } else { 1.0 };

    // Channel indices: FL=0, FR=1, RL=2, RR=3
    let get_channel_idx = |source: ChannelSource, channels: u16| -> usize {
        match source {
            ChannelSource::FL => 0,  // Front Left - always index 0
            ChannelSource::FR => 1,  // Front Right - always index 1
            ChannelSource::RL => if channels >= 4 { 2 } else { 0 },
            ChannelSource::RR => if channels >= 4 { 3 } else { 1 },
        }
    };

    for frame in 0..frames {
        let base = frame * channels as usize;
        
        // Get front channels for upmix (FL=0, FR=1)
        let fl = input.get(base).copied().unwrap_or(0.0);
        let fr = input.get(base + 1).copied().unwrap_or(0.0);
        
        // Get upmix contribution (pseudo surround from front channels)
        let (upmix_l, upmix_r) = dsp.get_upmix(fl, fr);
        
        // Get source samples based on channel settings
        let left_idx = get_channel_idx(left_ch.source, channels);
        let right_idx = get_channel_idx(right_ch.source, channels);
        
        let mut left = if left_ch.muted { 
            0.0 
        } else { 
            input.get(base + left_idx).copied().unwrap_or(0.0) * left_ch.volume
        };
        
        let mut right = if right_ch.muted { 
            0.0 
        } else { 
            input.get(base + right_idx).copied().unwrap_or(0.0) * right_ch.volume
        };
        
        // Add upmix contribution
        left += upmix_l;
        right += upmix_r;
        
        if swap {
            std::mem::swap(&mut left, &mut right);
        }
        
        // Apply final volume and clamp to prevent clipping
        let out_l = (left * volume * left_mult).clamp(-1.0, 1.0);
        let out_r = (right * volume * right_mult).clamp(-1.0, 1.0);
        output.push(out_l);
        output.push(out_r);
    }
    output
}
