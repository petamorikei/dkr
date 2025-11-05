//! Docker client trait and implementations

use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::container::{ListContainersOptions, RemoveContainerOptions, StopContainerOptions};
use bollard::image::ListImagesOptions;
use bollard::network::ListNetworksOptions;
use bollard::volume::ListVolumesOptions;
use bollard::Docker;

use super::container::{ContainerInfo, ContainerStats, ContainerSummary};

/// Trait defining Docker client operations
///
/// This trait abstracts Docker API operations, allowing for both real implementations
/// (via Bollard) and mock implementations for testing.
#[async_trait]
pub trait DockerClient: Send + Sync {
    /// Lists all containers
    ///
    /// # Arguments
    /// * `all` - If true, includes stopped containers. If false, only running containers.
    async fn list_containers(&self, all: bool) -> Result<Vec<ContainerSummary>>;

    /// Gets detailed information about a specific container
    async fn get_container(&self, id: &str) -> Result<ContainerInfo>;

    /// Gets real-time statistics for a container
    ///
    /// Returns CPU and memory usage metrics.
    async fn get_container_stats(&self, id: &str) -> Result<ContainerStats>;

    /// Starts a container
    async fn start_container(&self, id: &str) -> Result<()>;

    /// Stops a container
    async fn stop_container(&self, id: &str) -> Result<()>;

    /// Restarts a container
    async fn restart_container(&self, id: &str) -> Result<()>;

    /// Removes a container
    ///
    /// # Arguments
    /// * `id` - Container ID
    /// * `force` - If true, forces removal even if container is running
    async fn remove_container(&self, id: &str, force: bool) -> Result<()>;

    /// Lists all images
    async fn list_images(&self) -> Result<Vec<bollard::models::ImageSummary>>;

    /// Lists all volumes
    async fn list_volumes(&self) -> Result<bollard::models::VolumeListResponse>;

    /// Lists all networks
    async fn list_networks(&self) -> Result<Vec<bollard::models::Network>>;

    /// Gets container logs
    ///
    /// # Arguments
    /// * `id` - Container ID
    /// * `tail` - Optional number of lines to return from the end
    async fn get_container_logs(&self, id: &str, tail: Option<usize>) -> Result<Vec<String>>;

    /// Inspects an image
    async fn inspect_image(&self, id: &str) -> Result<bollard::models::ImageInspect>;

    /// Inspects a volume
    async fn inspect_volume(&self, name: &str) -> Result<bollard::models::Volume>;

    /// Inspects a network
    async fn inspect_network(&self, id: &str) -> Result<bollard::models::Network>;

    /// Removes an image
    ///
    /// # Arguments
    /// * `id` - Image ID
    /// * `force` - If true, forces removal even if image is in use
    async fn remove_image(&self, id: &str, force: bool) -> Result<()>;

    /// Removes a volume
    async fn remove_volume(&self, name: &str) -> Result<()>;

    /// Removes a network
    async fn remove_network(&self, id: &str) -> Result<()>;

    /// Pulls an image from a registry
    ///
    /// # Arguments
    /// * `image` - Image name with optional tag (e.g., "nginx:latest")
    async fn pull_image(&self, image: &str) -> Result<()>;
}

/// Bollard-based implementation of DockerClient
///
/// This is the production implementation that communicates with the Docker daemon
/// via the Bollard library.
pub struct BollardDockerClient {
    docker: Docker,
}

impl BollardDockerClient {
    /// Creates a new BollardDockerClient
    ///
    /// Connects to Docker using local defaults (Unix socket on Linux/macOS,
    /// named pipe on Windows). Tests the connection by pinging the daemon.
    ///
    /// # Errors
    ///
    /// Returns an error if Docker is not running or connection fails.
    pub async fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()
            .context("Failed to connect to Docker. Is Docker running?")?;

        // Test connection
        docker
            .ping()
            .await
            .context("Failed to ping Docker daemon")?;

        Ok(Self { docker })
    }
}

#[async_trait]
impl DockerClient for BollardDockerClient {
    async fn list_containers(&self, all: bool) -> Result<Vec<ContainerSummary>> {
        let options = ListContainersOptions::<String> {
            all,
            ..Default::default()
        };

        let containers = self
            .docker
            .list_containers(Some(options))
            .await
            .context("Failed to list containers")?;

        Ok(containers
            .into_iter()
            .map(ContainerSummary::from)
            .collect())
    }

    async fn get_container(&self, id: &str) -> Result<ContainerInfo> {
        let container = self
            .docker
            .inspect_container(id, None)
            .await
            .context(format!("Failed to inspect container {}", id))?;

        Ok(ContainerInfo::from(container))
    }

    async fn get_container_stats(&self, id: &str) -> Result<ContainerStats> {
        use bollard::container::StatsOptions;
        use futures::StreamExt;

        let options = StatsOptions {
            stream: false,
            one_shot: true,
        };

        let mut stream = self.docker.stats(id, Some(options));

        if let Some(result) = stream.next().await {
            let stats = result.context("Failed to get container stats")?;

            // Calculate CPU percentage
            let cpu_delta = stats.cpu_stats.cpu_usage.total_usage as f64
                - stats.precpu_stats.cpu_usage.total_usage as f64;
            let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0) as f64
                - stats.precpu_stats.system_cpu_usage.unwrap_or(0) as f64;
            let cpu_percent = if system_delta > 0.0 && cpu_delta > 0.0 {
                let num_cpus = stats.cpu_stats.online_cpus.unwrap_or(1) as f64;
                (cpu_delta / system_delta) * num_cpus * 100.0
            } else {
                0.0
            };

            // Get memory stats
            let memory_usage = stats.memory_stats.usage.unwrap_or(0);
            let memory_limit = stats.memory_stats.limit.unwrap_or(0);
            let memory_percent = if memory_limit > 0 {
                (memory_usage as f64 / memory_limit as f64) * 100.0
            } else {
                0.0
            };

            Ok(ContainerStats {
                cpu_percent,
                memory_usage,
                memory_limit,
                memory_percent,
            })
        } else {
            Err(anyhow::anyhow!("No stats available for container {}", id))
        }
    }

    async fn start_container(&self, id: &str) -> Result<()> {
        self.docker
            .start_container::<String>(id, None)
            .await
            .context(format!("Failed to start container {}", id))?;
        Ok(())
    }

    async fn stop_container(&self, id: &str) -> Result<()> {
        let options = StopContainerOptions { t: 10 };
        self.docker
            .stop_container(id, Some(options))
            .await
            .context(format!("Failed to stop container {}", id))?;
        Ok(())
    }

    async fn restart_container(&self, id: &str) -> Result<()> {
        self.docker
            .restart_container(id, None)
            .await
            .context(format!("Failed to restart container {}", id))?;
        Ok(())
    }

    async fn remove_container(&self, id: &str, force: bool) -> Result<()> {
        let options = RemoveContainerOptions {
            force,
            ..Default::default()
        };
        self.docker
            .remove_container(id, Some(options))
            .await
            .context(format!("Failed to remove container {}", id))?;
        Ok(())
    }

    async fn list_images(&self) -> Result<Vec<bollard::models::ImageSummary>> {
        let options = ListImagesOptions::<String> {
            all: true,
            ..Default::default()
        };

        self.docker
            .list_images(Some(options))
            .await
            .context("Failed to list images")
    }

    async fn list_volumes(&self) -> Result<bollard::models::VolumeListResponse> {
        let options = ListVolumesOptions::<String> {
            ..Default::default()
        };

        self.docker
            .list_volumes(Some(options))
            .await
            .context("Failed to list volumes")
    }

    async fn list_networks(&self) -> Result<Vec<bollard::models::Network>> {
        let options = ListNetworksOptions::<String> {
            ..Default::default()
        };

        self.docker
            .list_networks(Some(options))
            .await
            .context("Failed to list networks")
    }

    async fn get_container_logs(&self, id: &str, tail: Option<usize>) -> Result<Vec<String>> {
        use bollard::container::{LogOutput, LogsOptions};
        use futures::StreamExt;

        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: tail.map(|t| t.to_string()).unwrap_or_else(|| "all".to_string()),
            ..Default::default()
        };

        let mut stream = self.docker.logs(id, Some(options));
        let mut logs = Vec::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(output) => {
                    let line = match output {
                        LogOutput::StdOut { message } => String::from_utf8_lossy(&message).to_string(),
                        LogOutput::StdErr { message } => String::from_utf8_lossy(&message).to_string(),
                        LogOutput::Console { message } => String::from_utf8_lossy(&message).to_string(),
                        LogOutput::StdIn { .. } => continue,
                    };
                    logs.push(line);
                }
                Err(e) => return Err(anyhow::anyhow!("Failed to read logs: {}", e)),
            }
        }

        Ok(logs)
    }

    async fn inspect_image(&self, id: &str) -> Result<bollard::models::ImageInspect> {
        self.docker
            .inspect_image(id)
            .await
            .context(format!("Failed to inspect image {}", id))
    }

    async fn inspect_volume(&self, name: &str) -> Result<bollard::models::Volume> {
        self.docker
            .inspect_volume(name)
            .await
            .context(format!("Failed to inspect volume {}", name))
    }

    async fn inspect_network(&self, id: &str) -> Result<bollard::models::Network> {
        self.docker
            .inspect_network::<String>(id, None)
            .await
            .context(format!("Failed to inspect network {}", id))
    }

    async fn remove_image(&self, id: &str, force: bool) -> Result<()> {
        let options = bollard::image::RemoveImageOptions {
            force,
            ..Default::default()
        };
        self.docker
            .remove_image(id, Some(options), None)
            .await
            .context(format!("Failed to remove image {}", id))?;
        Ok(())
    }

    async fn remove_volume(&self, name: &str) -> Result<()> {
        let options = bollard::volume::RemoveVolumeOptions {
            force: false,
        };
        self.docker
            .remove_volume(name, Some(options))
            .await
            .context(format!("Failed to remove volume {}", name))?;
        Ok(())
    }

    async fn remove_network(&self, id: &str) -> Result<()> {
        self.docker
            .remove_network(id)
            .await
            .context(format!("Failed to remove network {}", id))?;
        Ok(())
    }

    async fn pull_image(&self, image: &str) -> Result<()> {
        use bollard::image::CreateImageOptions;
        use futures::StreamExt;

        let options = CreateImageOptions {
            from_image: image,
            ..Default::default()
        };

        let mut stream = self.docker.create_image(Some(options), None, None);

        while let Some(result) = stream.next().await {
            result.context(format!("Failed to pull image {}", image))?;
        }

        Ok(())
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock Docker client for testing
    pub struct MockDockerClient {
        containers: Mutex<HashMap<String, ContainerSummary>>,
        should_fail: Mutex<bool>,
    }

    impl MockDockerClient {
        pub fn new() -> Self {
            Self {
                containers: Mutex::new(HashMap::new()),
                should_fail: Mutex::new(false),
            }
        }

        pub fn add_container(&self, id: String, name: String, state: String, image: String) {
            let summary = ContainerSummary {
                id: id.clone(),
                name,
                status: state.clone(),
                state: state.parse().unwrap_or(super::super::container::ContainerState::Unknown),
                image,
                created: 0,
                ports: vec![],
            };
            self.containers.lock().unwrap().insert(id, summary);
        }

        pub fn set_should_fail(&self, should_fail: bool) {
            *self.should_fail.lock().unwrap() = should_fail;
        }
    }

    #[async_trait]
    impl DockerClient for MockDockerClient {
        async fn list_containers(&self, _all: bool) -> Result<Vec<ContainerSummary>> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: list_containers failed"));
            }
            Ok(self.containers.lock().unwrap().values().cloned().collect())
        }

        async fn get_container(&self, id: &str) -> Result<ContainerInfo> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: get_container failed"));
            }
            Err(anyhow::anyhow!("Mock: get_container not implemented for {}", id))
        }

        async fn get_container_stats(&self, _id: &str) -> Result<ContainerStats> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: get_container_stats failed"));
            }
            Ok(ContainerStats {
                cpu_percent: 25.5,
                memory_usage: 1024 * 1024 * 100, // 100 MB
                memory_limit: 1024 * 1024 * 512, // 512 MB
                memory_percent: 19.53,
            })
        }

        async fn start_container(&self, _id: &str) -> Result<()> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: start_container failed"));
            }
            Ok(())
        }

        async fn stop_container(&self, _id: &str) -> Result<()> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: stop_container failed"));
            }
            Ok(())
        }

        async fn restart_container(&self, _id: &str) -> Result<()> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: restart_container failed"));
            }
            Ok(())
        }

        async fn remove_container(&self, id: &str, _force: bool) -> Result<()> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: remove_container failed"));
            }
            self.containers.lock().unwrap().remove(id);
            Ok(())
        }

        async fn list_images(&self) -> Result<Vec<bollard::models::ImageSummary>> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: list_images failed"));
            }
            Ok(vec![])
        }

        async fn list_volumes(&self) -> Result<bollard::models::VolumeListResponse> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: list_volumes failed"));
            }
            Ok(bollard::models::VolumeListResponse {
                volumes: Some(vec![]),
                warnings: None,
            })
        }

        async fn list_networks(&self) -> Result<Vec<bollard::models::Network>> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: list_networks failed"));
            }
            Ok(vec![])
        }

        async fn get_container_logs(&self, _id: &str, _tail: Option<usize>) -> Result<Vec<String>> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: get_container_logs failed"));
            }
            Ok(vec!["Mock log line 1".to_string(), "Mock log line 2".to_string()])
        }

        async fn inspect_image(&self, _id: &str) -> Result<bollard::models::ImageInspect> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: inspect_image failed"));
            }
            Err(anyhow::anyhow!("Mock: inspect_image not implemented"))
        }

        async fn inspect_volume(&self, _name: &str) -> Result<bollard::models::Volume> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: inspect_volume failed"));
            }
            Err(anyhow::anyhow!("Mock: inspect_volume not implemented"))
        }

        async fn inspect_network(&self, _id: &str) -> Result<bollard::models::Network> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: inspect_network failed"));
            }
            Err(anyhow::anyhow!("Mock: inspect_network not implemented"))
        }

        async fn remove_image(&self, _id: &str, _force: bool) -> Result<()> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: remove_image failed"));
            }
            Ok(())
        }

        async fn remove_volume(&self, _name: &str) -> Result<()> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: remove_volume failed"));
            }
            Ok(())
        }

        async fn remove_network(&self, _id: &str) -> Result<()> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: remove_network failed"));
            }
            Ok(())
        }

        async fn pull_image(&self, _image: &str) -> Result<()> {
            if *self.should_fail.lock().unwrap() {
                return Err(anyhow::anyhow!("Mock error: pull_image failed"));
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[tokio::test]
        async fn test_mock_docker_client_list_containers() {
            let mock = MockDockerClient::new();
            mock.add_container(
                "container1".to_string(),
                "test-container".to_string(),
                "running".to_string(),
                "nginx:latest".to_string(),
            );

            let containers = mock.list_containers(true).await.unwrap();
            assert_eq!(containers.len(), 1);
            assert_eq!(containers[0].name, "test-container");
            assert_eq!(containers[0].image, "nginx:latest");
        }

        #[tokio::test]
        async fn test_mock_docker_client_remove_container() {
            let mock = MockDockerClient::new();
            mock.add_container(
                "container1".to_string(),
                "test-container".to_string(),
                "running".to_string(),
                "nginx:latest".to_string(),
            );

            assert_eq!(mock.list_containers(true).await.unwrap().len(), 1);
            mock.remove_container("container1", false).await.unwrap();
            assert_eq!(mock.list_containers(true).await.unwrap().len(), 0);
        }

        #[tokio::test]
        async fn test_mock_docker_client_failure() {
            let mock = MockDockerClient::new();
            mock.set_should_fail(true);

            let result = mock.list_containers(true).await;
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("Mock error"));
        }
    }
}
