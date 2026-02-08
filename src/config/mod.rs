use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ChannelSource {
    FL,  // Front Left (index 0) - for stereo clone
    FR,  // Front Right (index 1) - for stereo clone
    RL,  // Rear Left (index 2)
    RR,  // Rear Right (index 3)
}

impl Default for ChannelSource {
    fn default() -> Self {
        ChannelSource::RL
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub source: ChannelSource,  // Which source channel to use
    pub volume: f32,            // Individual volume (0.0 - 2.0)
    pub muted: bool,            // Mute this channel
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            source: ChannelSource::RL,
            volume: 1.0,
            muted: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub source_device: Option<String>,
    pub target_device: Option<String>,
    pub volume: f32,
    pub balance: f32,  // -1.0 (full left) to 1.0 (full right), 0.0 = center
    pub enabled: bool,
    pub swap_channels: bool,
    pub clone_stereo: bool,  // Use FL/FR instead of RL/RR
    pub left_channel: ChannelConfig,   // Left speaker settings
    pub right_channel: ChannelConfig,  // Right speaker settings
    // DSP settings
    pub delay_ms: f32,       // Delay in milliseconds (0-200)
    pub eq_enabled: bool,
    pub eq_low: f32,         // -12.0 to +12.0 dB
    pub eq_mid: f32,         // -12.0 to +12.0 dB
    pub eq_high: f32,        // -12.0 to +12.0 dB
    pub upmix_enabled: bool, // Pseudo-surround from stereo
    pub upmix_strength: f32, // 0.0 to 1.0
    pub sync_master_volume: bool, // Sync with Windows master volume
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            source_device: None,
            target_device: None,
            volume: 1.0,
            balance: 0.0,
            enabled: true,
            swap_channels: false,
            clone_stereo: false,
            left_channel: ChannelConfig {
                source: ChannelSource::RL,
                volume: 1.0,
                muted: false,
            },
            right_channel: ChannelConfig {
                source: ChannelSource::RR,
                volume: 1.0,
                muted: false,
            },
            delay_ms: 0.0,
            eq_enabled: false,
            eq_low: 0.0,
            eq_mid: 0.0,
            eq_high: 0.0,
            upmix_enabled: false,
            upmix_strength: 4.0,  // 4x for matching main volume
            sync_master_volume: true,  // Default: sync with Windows volume
        }
    }
}

impl AppConfig {
    pub fn config_path() -> Result<PathBuf> {
        let exe_path = std::env::current_exe().context("Failed to get executable path")?;
        let config_path = exe_path
            .parent()
            .context("Failed to get executable directory")?
            .join("config.toml");
        Ok(config_path)
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if path.exists() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config from {:?}", path))?;
            let config: AppConfig =
                toml::from_str(&content).context("Failed to parse config file")?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write config to {:?}", path))?;
        Ok(())
    }
}
