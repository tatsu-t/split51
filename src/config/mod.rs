use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ChannelSource {
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
    pub left_channel: ChannelConfig,   // Left speaker settings
    pub right_channel: ChannelConfig,  // Right speaker settings
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
