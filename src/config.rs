use anyhow::Result;
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub docker: DockerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u64,
    #[serde(default = "default_view")]
    pub default_view: String,
    #[serde(default = "default_confirm_delete")]
    pub confirm_delete: bool,
    #[serde(default = "default_auto_refresh")]
    pub auto_refresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_show_header")]
    pub show_header: bool,
    #[serde(default = "default_show_footer")]
    pub show_footer: bool,
    #[serde(default = "default_show_logs_pane")]
    pub show_logs_pane: bool,
    #[serde(default = "default_logs_buffer_size")]
    pub logs_buffer_size: usize,
    #[serde(default = "default_datetime_format")]
    pub datetime_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    #[serde(default = "default_docker_socket")]
    pub socket: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            ui: UiConfig::default(),
            docker: DockerConfig::default(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            refresh_interval: default_refresh_interval(),
            default_view: default_view(),
            confirm_delete: default_confirm_delete(),
            auto_refresh: default_auto_refresh(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            show_header: default_show_header(),
            show_footer: default_show_footer(),
            show_logs_pane: default_show_logs_pane(),
            logs_buffer_size: default_logs_buffer_size(),
            datetime_format: default_datetime_format(),
        }
    }
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            socket: default_docker_socket(),
            timeout: default_timeout(),
        }
    }
}

// Default value functions
fn default_refresh_interval() -> u64 {
    5
}
fn default_view() -> String {
    "containers".to_string()
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
fn default_show_header() -> bool {
    true
}
fn default_show_footer() -> bool {
    true
}
fn default_show_logs_pane() -> bool {
    false
}
fn default_logs_buffer_size() -> usize {
    1000
}
fn default_datetime_format() -> String {
    "%Y-%m-%d %H:%M:%S".to_string()
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
fn default_timeout() -> u64 {
    30
}

impl Config {
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

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();
        
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        
        Ok(())
    }

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
        assert_eq!(config.general.confirm_delete, true);
        assert_eq!(config.ui.theme, "dark");
        assert_eq!(config.ui.logs_buffer_size, 1000);
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