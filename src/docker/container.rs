use bollard::models::{ContainerInspectResponse, ContainerSummary as BollardContainer};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSummary {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub state: ContainerState,
    pub created: i64,
    pub ports: Vec<PortMapping>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub state: ContainerState,
    pub created: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub ports: Vec<PortMapping>,
    pub mounts: Vec<MountInfo>,
    pub networks: Vec<String>,
    pub command: String,
    pub entrypoint: Vec<String>,
    pub environment: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ContainerState {
    Running,
    Paused,
    Restarting,
    Exited,
    Dead,
    Created,
    Unknown,
}

impl ContainerState {
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

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "running" => ContainerState::Running,
            "paused" => ContainerState::Paused,
            "restarting" => ContainerState::Restarting,
            "exited" => ContainerState::Exited,
            "dead" => ContainerState::Dead,
            "created" => ContainerState::Created,
            _ => ContainerState::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub private_port: u16,
    pub public_port: Option<u16>,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountInfo {
    pub source: String,
    pub destination: String,
    pub mode: String,
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

        let state = ContainerState::from_str(container.state.as_deref().unwrap_or("unknown"));

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
            Some(status) => ContainerState::from_str(&status.to_string()),
            None => ContainerState::Unknown,
        };

        let config = inspect.config.unwrap_or_default();

        let ports = if let Some(ref network_settings) = inspect.network_settings {
            if let Some(ports) = &network_settings.ports {
                ports
                    .into_iter()
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
            .unwrap_or_else(|| Utc::now());

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_state_conversion() {
        assert_eq!(ContainerState::from_str("running"), ContainerState::Running);
        assert_eq!(ContainerState::from_str("EXITED"), ContainerState::Exited);
        assert_eq!(ContainerState::from_str("unknown"), ContainerState::Unknown);
        
        assert_eq!(ContainerState::Running.as_str(), "Running");
        assert_eq!(ContainerState::Exited.as_str(), "Exited");
    }
}