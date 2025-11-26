//! Docker container data structures and conversions

use bollard::models::{ContainerInspectResponse, ContainerSummary as BollardContainer};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Summary information about a Docker container
///
/// This is a simplified representation used for list views in the TUI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSummary {
    /// Container ID
    pub id: String,
    /// Container name (without leading slash)
    pub name: String,
    /// Image name the container was created from
    pub image: String,
    /// Human-readable status string
    pub status: String,
    /// Current state of the container
    pub state: ContainerState,
    /// Unix timestamp of when the container was created
    pub created: i64,
    /// Port mappings for the container
    pub ports: Vec<PortMapping>,
}

/// Detailed information about a Docker container
///
/// Contains comprehensive container metadata obtained from Docker inspect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    /// Container ID
    pub id: String,
    /// Container name
    pub name: String,
    /// Image name
    pub image: String,
    /// Status string
    pub status: String,
    /// Current state
    pub state: ContainerState,
    /// Creation timestamp
    pub created: DateTime<Utc>,
    /// When the container was started (if applicable)
    pub started_at: Option<DateTime<Utc>>,
    /// When the container finished (if applicable)
    pub finished_at: Option<DateTime<Utc>>,
    /// Port mappings
    pub ports: Vec<PortMapping>,
    /// Volume mounts
    pub mounts: Vec<MountInfo>,
    /// Network names the container is connected to
    pub networks: Vec<String>,
    /// Command being executed
    pub command: String,
    /// Entrypoint configuration
    pub entrypoint: Vec<String>,
    /// Environment variables
    pub environment: Vec<String>,
}

/// Represents the current state of a Docker container
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ContainerState {
    /// Container is currently running
    Running,
    /// Container is paused
    Paused,
    /// Container is in the process of restarting
    Restarting,
    /// Container has exited
    Exited,
    /// Container is dead (rare state)
    Dead,
    /// Container has been created but not started
    Created,
    /// Unknown or unrecognized state
    Unknown,
}

impl ContainerState {
    /// Returns the string representation of the container state
    pub fn as_str(&self) -> &'static str {
        match self {
            ContainerState::Running => "Running",
            ContainerState::Paused => "Paused",
            ContainerState::Restarting => "Restarting",
            ContainerState::Exited => "Exited",
            ContainerState::Dead => "Dead",
            ContainerState::Created => "Created",
            ContainerState::Unknown => "Unknown",
        }
    }

    /// Returns the state with a Unicode icon prefix for accessibility
    pub fn with_icon(&self) -> &'static str {
        match self {
            ContainerState::Running => "▶ Running",
            ContainerState::Paused => "⏸ Paused",
            ContainerState::Restarting => "↻ Restarting",
            ContainerState::Exited => "■ Exited",
            ContainerState::Dead => "✖ Dead",
            ContainerState::Created => "○ Created",
            ContainerState::Unknown => "? Unknown",
        }
    }
}

impl FromStr for ContainerState {
    type Err = ();

    /// Parses a string into a ContainerState
    ///
    /// Case-insensitive parsing. Unknown states are mapped to `Unknown`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "running" => ContainerState::Running,
            "paused" => ContainerState::Paused,
            "restarting" => ContainerState::Restarting,
            "exited" => ContainerState::Exited,
            "dead" => ContainerState::Dead,
            "created" => ContainerState::Created,
            _ => ContainerState::Unknown,
        })
    }
}

/// Port mapping information for a container
///
/// Represents how a container's internal port is exposed to the host.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Internal container port
    pub private_port: u16,
    /// Host port (if mapped)
    pub public_port: Option<u16>,
    /// Protocol (tcp/udp)
    pub protocol: String,
}

/// Volume mount information for a container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountInfo {
    /// Source path on the host
    pub source: String,
    /// Destination path in the container
    pub destination: String,
    /// Mount mode (e.g., "rw", "ro")
    pub mode: String,
    /// Type of mount (e.g., "bind", "volume")
    pub mount_type: String,
}

impl From<BollardContainer> for ContainerSummary {
    fn from(container: BollardContainer) -> Self {
        let name = container
            .names
            .and_then(|names| names.first().cloned())
            .unwrap_or_default()
            .trim_start_matches('/')
            .to_string();

        let ports = container
            .ports
            .unwrap_or_default()
            .into_iter()
            .map(|p| PortMapping {
                private_port: p.private_port,
                public_port: p.public_port,
                protocol: p
                    .typ
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "tcp".to_string()),
            })
            .collect();

        let state = container
            .state
            .as_deref()
            .unwrap_or("unknown")
            .parse()
            .unwrap_or(ContainerState::Unknown);

        Self {
            id: container.id.unwrap_or_default(),
            name,
            image: container.image.unwrap_or_default(),
            status: container.status.unwrap_or_default(),
            state,
            created: container.created.unwrap_or(0),
            ports,
        }
    }
}

impl From<ContainerInspectResponse> for ContainerInfo {
    fn from(inspect: ContainerInspectResponse) -> Self {
        let name = inspect
            .name
            .unwrap_or_default()
            .trim_start_matches('/')
            .to_string();

        let state_info = inspect.state.unwrap_or_default();
        let state = match &state_info.status {
            Some(status) => status.as_ref().parse().unwrap_or(ContainerState::Unknown),
            None => ContainerState::Unknown,
        };

        let config = inspect.config.unwrap_or_default();

        let ports = if let Some(ref network_settings) = inspect.network_settings {
            if let Some(ports) = &network_settings.ports {
                ports
                    .iter()
                    .filter_map(|(port_proto, bindings)| {
                        let parts: Vec<&str> = port_proto.split('/').collect();
                        // Safely parse port with error handling
                        let port = parts[0].parse::<u16>().ok()?;
                        let protocol = parts.get(1).unwrap_or(&"tcp").to_string();

                        let public_port = bindings
                            .as_ref()
                            .and_then(|b| b.first())
                            .and_then(|b| b.host_port.as_ref())
                            .and_then(|p| p.parse::<u16>().ok());

                        Some(PortMapping {
                            private_port: port,
                            public_port,
                            protocol,
                        })
                    })
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let mounts = inspect
            .mounts
            .unwrap_or_default()
            .into_iter()
            .map(|m| MountInfo {
                source: m.source.unwrap_or_default(),
                destination: m.destination.unwrap_or_default(),
                mode: m.mode.unwrap_or_default(),
                mount_type: m
                    .typ
                    .as_ref()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "bind".to_string()),
            })
            .collect();

        let networks = if let Some(network_settings) = &inspect.network_settings {
            if let Some(networks) = &network_settings.networks {
                networks.keys().cloned().collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let created = inspect
            .created
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let started_at = state_info
            .started_at
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let finished_at = state_info
            .finished_at
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Self {
            id: inspect.id.unwrap_or_default(),
            name,
            image: config.image.unwrap_or_default(),
            status: state_info
                .status
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            state,
            created,
            started_at,
            finished_at,
            ports,
            mounts,
            networks,
            command: config.cmd.unwrap_or_default().join(" "),
            entrypoint: config.entrypoint.unwrap_or_default(),
            environment: config.env.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Real-time statistics for a running container
///
/// Contains CPU and memory usage metrics.
pub struct ContainerStats {
    /// CPU usage as a percentage (0-100+)
    pub cpu_percent: f64,
    /// Current memory usage in bytes
    pub memory_usage: u64,
    /// Memory limit in bytes
    pub memory_limit: u64,
    /// Memory usage as a percentage (0-100)
    pub memory_percent: f64,
}

impl Default for ContainerStats {
    fn default() -> Self {
        Self {
            cpu_percent: 0.0,
            memory_usage: 0,
            memory_limit: 0,
            memory_percent: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_state_conversion() {
        assert_eq!(
            "running".parse::<ContainerState>().unwrap(),
            ContainerState::Running
        );
        assert_eq!(
            "EXITED".parse::<ContainerState>().unwrap(),
            ContainerState::Exited
        );
        assert_eq!(
            "unknown".parse::<ContainerState>().unwrap(),
            ContainerState::Unknown
        );

        assert_eq!(ContainerState::Running.as_str(), "Running");
        assert_eq!(ContainerState::Exited.as_str(), "Exited");
    }
}
