//! DSP (Digital Signal Processing) module for split51
//! Provides delay, EQ, upmix, and level metering

use std::f32::consts::PI;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Delay buffer for latency compensation
pub struct DelayBuffer {
    buffer: Vec<f32>,
    write_pos: usize,
    delay_samples: usize,
}

impl DelayBuffer {
    pub fn new(max_delay_samples: usize) -> Self {
        Self {
            buffer: vec![0.0; max_delay_samples],
            write_pos: 0,
            delay_samples: 0,
        }
    }

    pub fn set_delay_samples(&mut self, samples: usize) {
        self.delay_samples = samples.min(self.buffer.len());
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        if self.delay_samples == 0 {
            return sample;
        }

        let read_pos = (self.write_pos + self.buffer.len() - self.delay_samples) % self.buffer.len();
        let output = self.buffer[read_pos];
        self.buffer[self.write_pos] = sample;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();
        output
    }
}

/// Biquad filter for EQ and filtering
#[derive(Clone)]
pub struct Biquad {
    b0: f32, b1: f32, b2: f32,
    a1: f32, a2: f32,
    x1: f32, x2: f32,
    y1: f32, y2: f32,
}

impl Biquad {
    pub fn new() -> Self {
        Self {
            b0: 1.0, b1: 0.0, b2: 0.0,
            a1: 0.0, a2: 0.0,
            x1: 0.0, x2: 0.0,
            y1: 0.0, y2: 0.0,
        }
    }

    /// Low-shelf filter
    pub fn low_shelf(freq: f32, gain_db: f32, sample_rate: f32) -> Self {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / 2.0 * (2.0_f32).sqrt();

        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;
        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);

        Self {
            b0: b0 / a0, b1: b1 / a0, b2: b2 / a0,
            a1: a1 / a0, a2: a2 / a0,
            x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0,
        }
    }

    /// High-shelf filter
    pub fn high_shelf(freq: f32, gain_db: f32, sample_rate: f32) -> Self {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / 2.0 * (2.0_f32).sqrt();

        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;
        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);

        Self {
            b0: b0 / a0, b1: b1 / a0, b2: b2 / a0,
            a1: a1 / a0, a2: a2 / a0,
            x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0,
        }
    }

    /// Peaking EQ filter
    pub fn peaking(freq: f32, gain_db: f32, q: f32, sample_rate: f32) -> Self {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;
        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;

        Self {
            b0: b0 / a0, b1: b1 / a0, b2: b2 / a0,
            a1: a1 / a0, a2: a2 / a0,
            x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0,
        }
    }

    /// High-pass filter for upmix
    pub fn highpass(freq: f32, q: f32, sample_rate: f32) -> Self {
        let w0 = 2.0 * PI * freq / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let a0 = 1.0 + alpha;
        let b0 = (1.0 + cos_w0) / 2.0;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) / 2.0;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0, b1: b1 / a0, b2: b2 / a0,
            a1: a1 / a0, a2: a2 / a0,
            x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
                   - self.a1 * self.y1 - self.a2 * self.y2;
        
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        
        output
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

/// 3-band equalizer
pub struct ThreeBandEq {
    low_shelf: Biquad,
    mid_peak: Biquad,
    high_shelf: Biquad,
    sample_rate: f32,
}

impl ThreeBandEq {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            low_shelf: Biquad::low_shelf(200.0, 0.0, sample_rate),
            mid_peak: Biquad::peaking(1000.0, 0.0, 1.0, sample_rate),
            high_shelf: Biquad::high_shelf(4000.0, 0.0, sample_rate),
            sample_rate,
        }
    }

    pub fn set_gains(&mut self, low_db: f32, mid_db: f32, high_db: f32) {
        self.low_shelf = Biquad::low_shelf(200.0, low_db, self.sample_rate);
        self.mid_peak = Biquad::peaking(1000.0, mid_db, 1.0, self.sample_rate);
        self.high_shelf = Biquad::high_shelf(4000.0, high_db, self.sample_rate);
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        let s = self.low_shelf.process(sample);
        let s = self.mid_peak.process(s);
        self.high_shelf.process(s)
    }
}

/// Pseudo-surround upmixer: creates rear channel content from stereo
pub struct Upmixer {
    hp_left: Biquad,
    hp_right: Biquad,
    delay_left: DelayBuffer,
    delay_right: DelayBuffer,
    strength: f32,
}

impl Upmixer {
    pub fn new(sample_rate: u32) -> Self {
        let sr = sample_rate as f32;
        // 10ms delay for spaciousness
        let delay_samples = (sr * 0.010) as usize;
        
        let mut delay_left = DelayBuffer::new(delay_samples * 2);
        let mut delay_right = DelayBuffer::new(delay_samples * 2);
        delay_left.set_delay_samples(delay_samples);
        delay_right.set_delay_samples(delay_samples);
        
        Self {
            // Lower cutoff (150Hz) to preserve more bass
            hp_left: Biquad::highpass(150.0, 0.7, sr),
            hp_right: Biquad::highpass(150.0, 0.7, sr),
            delay_left,
            delay_right,
            strength: 4.0,  // 4x strength for matching main volume
        }
    }

    pub fn set_strength(&mut self, strength: f32) {
        self.strength = strength.clamp(0.0, 10.0);  // Allow higher values
    }

    /// Process stereo input and return rear channel output
    /// Takes FL/FR, returns RL/RR to be mixed with output
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        // High-pass filter to remove sub-bass (keep most audio)
        let hp_l = self.hp_left.process(left);
        let hp_r = self.hp_right.process(right);
        
        // Delay for spaciousness
        let delayed_l = self.delay_left.process(hp_l);
        let delayed_r = self.delay_right.process(hp_r);
        
        // Output at full volume with slight cross-feed
        let rear_l = (delayed_l * 0.9 + delayed_r * 0.1) * self.strength;
        let rear_r = (delayed_r * 0.9 + delayed_l * 0.1) * self.strength;
        
        (rear_l, rear_r)
    }
}

/// Level meter for monitoring audio levels
pub struct LevelMeter {
    left_rms: f32,
    right_rms: f32,
    left_peak: f32,
    right_peak: f32,
    attack: f32,
    release: f32,
}

impl LevelMeter {
    pub fn new() -> Self {
        Self {
            left_rms: 0.0,
            right_rms: 0.0,
            left_peak: 0.0,
            right_peak: 0.0,
            attack: 0.01,   // Fast attack
            release: 0.001, // Slow release
        }
    }

    pub fn process(&mut self, left: f32, right: f32) {
        // RMS with smoothing
        let left_sq = left * left;
        let right_sq = right * right;
        
        let coeff = if left_sq > self.left_rms { self.attack } else { self.release };
        self.left_rms += coeff * (left_sq - self.left_rms);
        
        let coeff = if right_sq > self.right_rms { self.attack } else { self.release };
        self.right_rms += coeff * (right_sq - self.right_rms);
        
        // Peak hold
        let left_abs = left.abs();
        let right_abs = right.abs();
        
        if left_abs > self.left_peak {
            self.left_peak = left_abs;
        } else {
            self.left_peak *= 0.9995; // Peak decay
        }
        
        if right_abs > self.right_peak {
            self.right_peak = right_abs;
        } else {
            self.right_peak *= 0.9995;
        }
    }

    pub fn get_rms_db(&self) -> (f32, f32) {
        let left_db = 20.0 * self.left_rms.sqrt().max(1e-10).log10();
        let right_db = 20.0 * self.right_rms.sqrt().max(1e-10).log10();
        (left_db.max(-60.0), right_db.max(-60.0))
    }

    pub fn get_peak_db(&self) -> (f32, f32) {
        let left_db = 20.0 * self.left_peak.max(1e-10).log10();
        let right_db = 20.0 * self.right_peak.max(1e-10).log10();
        (left_db.max(-60.0), right_db.max(-60.0))
    }
}

/// Shared level values for display (thread-safe)
pub struct SharedLevels {
    // Store as integer (dB * 10) for atomic access
    left_db: AtomicU32,
    right_db: AtomicU32,
}

impl SharedLevels {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            left_db: AtomicU32::new(0),
            right_db: AtomicU32::new(0),
        })
    }

    pub fn update(&self, left_db: f32, right_db: f32) {
        // Convert to positive integer (add 60 to make -60..0 -> 0..60)
        let left = ((left_db + 60.0) * 10.0).clamp(0.0, 600.0) as u32;
        let right = ((right_db + 60.0) * 10.0).clamp(0.0, 600.0) as u32;
        self.left_db.store(left, Ordering::Relaxed);
        self.right_db.store(right, Ordering::Relaxed);
    }

    pub fn get(&self) -> (f32, f32) {
        let left = self.left_db.load(Ordering::Relaxed) as f32 / 10.0 - 60.0;
        let right = self.right_db.load(Ordering::Relaxed) as f32 / 10.0 - 60.0;
        (left, right)
    }
}

/// DSP chain combining all effects
pub struct DspChain {
    pub delay_l: DelayBuffer,
    pub delay_r: DelayBuffer,
    pub eq_l: ThreeBandEq,
    pub eq_r: ThreeBandEq,
    pub upmixer: Upmixer,
    pub meter: LevelMeter,
    pub shared_levels: Arc<SharedLevels>,
    pub delay_ms: f32,
    pub eq_enabled: bool,
    pub upmix_enabled: bool,
    sample_rate: u32,
    update_counter: u32,
    // Cache for EQ settings to avoid unnecessary recalculations
    eq_low_cache: f32,
    eq_mid_cache: f32,
    eq_high_cache: f32,
}

impl DspChain {
    pub fn new(sample_rate: u32, shared_levels: Arc<SharedLevels>) -> Self {
        let max_delay = (sample_rate as f32 * 0.2) as usize; // 200ms max
        
        Self {
            delay_l: DelayBuffer::new(max_delay),
            delay_r: DelayBuffer::new(max_delay),
            eq_l: ThreeBandEq::new(sample_rate as f32),
            eq_r: ThreeBandEq::new(sample_rate as f32),
            upmixer: Upmixer::new(sample_rate),
            meter: LevelMeter::new(),
            shared_levels,
            delay_ms: 0.0,
            eq_enabled: false,
            upmix_enabled: false,
            sample_rate,
            update_counter: 0,
            eq_low_cache: 0.0,
            eq_mid_cache: 0.0,
            eq_high_cache: 0.0,
        }
    }

    pub fn set_delay_ms(&mut self, ms: f32) {
        self.delay_ms = ms;
        let samples = (self.sample_rate as f32 * ms / 1000.0) as usize;
        self.delay_l.set_delay_samples(samples);
        self.delay_r.set_delay_samples(samples);
    }

    pub fn set_eq(&mut self, low_db: f32, mid_db: f32, high_db: f32) {
        // Only recalculate if values changed
        if (low_db - self.eq_low_cache).abs() > 0.1 
            || (mid_db - self.eq_mid_cache).abs() > 0.1 
            || (high_db - self.eq_high_cache).abs() > 0.1 
        {
            self.eq_l.set_gains(low_db, mid_db, high_db);
            self.eq_r.set_gains(low_db, mid_db, high_db);
            self.eq_low_cache = low_db;
            self.eq_mid_cache = mid_db;
            self.eq_high_cache = high_db;
        }
    }

    /// Process a stereo frame (L, R) and return processed (L, R)
    pub fn process(&mut self, left: f32, right: f32) -> (f32, f32) {
        let mut l = left;
        let mut r = right;

        // Apply EQ if enabled
        if self.eq_enabled {
            l = self.eq_l.process(l);
            r = self.eq_r.process(r);
        }

        // Apply delay
        l = self.delay_l.process(l);
        r = self.delay_r.process(r);

        // Update level meter
        self.meter.process(l, r);
        
        // Update shared levels periodically (every 256 samples)
        self.update_counter += 1;
        if self.update_counter >= 256 {
            self.update_counter = 0;
            let (left_db, right_db) = self.meter.get_rms_db();
            self.shared_levels.update(left_db, right_db);
        }

        (l, r)
    }

    /// Get upmixed rear channels from front stereo
    pub fn get_upmix(&mut self, front_l: f32, front_r: f32) -> (f32, f32) {
        if self.upmix_enabled {
            self.upmixer.process(front_l, front_r)
        } else {
            (0.0, 0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay_buffer() {
        let mut delay = DelayBuffer::new(100);
        delay.set_delay_samples(10);
        
        // First 10 samples should be 0
        for _ in 0..10 {
            assert_eq!(delay.process(1.0), 0.0);
        }
        // After delay, should output 1.0
        assert_eq!(delay.process(1.0), 1.0);
    }

    #[test]
    fn test_level_meter() {
        let mut meter = LevelMeter::new();
        for _ in 0..1000 {
            meter.process(0.5, 0.5);
        }
        let (l, r) = meter.get_rms_db();
        // 0.5 amplitude = ~-6 dB
        assert!(l > -10.0 && l < -4.0);
        assert!(r > -10.0 && r < -4.0);
    }
}
