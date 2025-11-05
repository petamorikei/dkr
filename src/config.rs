//! Configuration management for the dkr application

use anyhow::Result;
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main configuration structure
///
/// Loaded from `~/.config/dkr/config.toml`. All fields have defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// General application settings
    #[serde(default)]
    pub general: GeneralConfig,
    /// UI-related settings
    #[serde(default)]
    pub ui: UiConfig,
    /// Docker-specific settings
    #[serde(default)]
    pub docker: DockerConfig,
}

/// General application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Data refresh interval in seconds (default: 5)
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u64,
    /// Whether to show confirmation before deleting resources (default: true)
    #[serde(default = "default_confirm_delete")]
    pub confirm_delete: bool,
    /// Whether to automatically refresh data (default: true)
    #[serde(default = "default_auto_refresh")]
    pub auto_refresh: bool,
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Color theme name (default: "dark")
    #[serde(default = "default_theme")]
    pub theme: String,
}

/// Docker connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    /// Docker socket path
    ///
    /// Default: `/var/run/docker.sock` on Unix, `npipe:////./pipe/docker_engine` on Windows
    #[serde(default = "default_docker_socket")]
    pub socket: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            refresh_interval: default_refresh_interval(),
            confirm_delete: default_confirm_delete(),
            auto_refresh: default_auto_refresh(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
        }
    }
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            socket: default_docker_socket(),
        }
    }
}

// Default value functions
fn default_refresh_interval() -> u64 {
    5
}
fn default_confirm_delete() -> bool {
    true
}
fn default_auto_refresh() -> bool {
    true
}
fn default_theme() -> String {
    "dark".to_string()
}
fn default_docker_socket() -> String {
    #[cfg(unix)]
    {
        "unix:///var/run/docker.sock".to_string()
    }
    #[cfg(windows)]
    {
        "npipe:////./pipe/docker_engine".to_string()
    }
}

impl Config {
    /// Loads configuration from file
    ///
    /// Attempts to load from `~/.config/dkr/config.toml`.
    /// If the file doesn't exist, returns default configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the file exists but cannot be read or parsed.
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Saves configuration to file
    ///
    /// Writes configuration to `~/.config/dkr/config.toml`.
    /// Creates the directory if it doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        
        Ok(())
    }

    /// Returns the path to the configuration file
    ///
    /// Priority order:
    /// 1. `$DKR_CONFIG` environment variable
    /// 2. `~/.config/dkr/config.toml` (platform-specific config directory)
    /// 3. `.dkr.toml` (current directory fallback)
    fn config_path() -> PathBuf {
        if let Ok(path) = std::env::var("DKR_CONFIG") {
            PathBuf::from(path)
        } else if let Some(config_dir) = config_dir() {
            config_dir.join("dkr").join("config.toml")
        } else {
            PathBuf::from(".dkr.toml")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.general.refresh_interval, 5);
        assert!(config.general.confirm_delete);
        assert_eq!(config.ui.theme, "dark");
    }

    #[test]
    fn test_docker_socket_default() {
        let docker_config = DockerConfig::default();
        #[cfg(unix)]
        assert_eq!(docker_config.socket, "unix:///var/run/docker.sock");
        #[cfg(windows)]
        assert_eq!(docker_config.socket, "npipe:////./pipe/docker_engine");
    }
}