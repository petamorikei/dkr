use anyhow::{Context, Result};
use bollard::container::{ListContainersOptions, RemoveContainerOptions, StopContainerOptions};
use bollard::image::ListImagesOptions;
use bollard::network::ListNetworksOptions;
use bollard::volume::ListVolumesOptions;
use bollard::Docker;

use super::container::{ContainerInfo, ContainerSummary};

pub struct DockerClient {
    docker: Docker,
}

impl DockerClient {
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

    pub async fn list_containers(&self, all: bool) -> Result<Vec<ContainerSummary>> {
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

    pub async fn get_container(&self, id: &str) -> Result<ContainerInfo> {
        let container = self
            .docker
            .inspect_container(id, None)
            .await
            .context(format!("Failed to inspect container {}", id))?;

        Ok(ContainerInfo::from(container))
    }

    pub async fn start_container(&self, id: &str) -> Result<()> {
        self.docker
            .start_container::<String>(id, None)
            .await
            .context(format!("Failed to start container {}", id))?;
        Ok(())
    }

    pub async fn stop_container(&self, id: &str) -> Result<()> {
        let options = StopContainerOptions { t: 10 };
        self.docker
            .stop_container(id, Some(options))
            .await
            .context(format!("Failed to stop container {}", id))?;
        Ok(())
    }

    pub async fn restart_container(&self, id: &str) -> Result<()> {
        self.docker
            .restart_container(id, None)
            .await
            .context(format!("Failed to restart container {}", id))?;
        Ok(())
    }

    pub async fn remove_container(&self, id: &str, force: bool) -> Result<()> {
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

    pub async fn list_images(&self) -> Result<Vec<bollard::models::ImageSummary>> {
        let options = ListImagesOptions::<String> {
            all: true,
            ..Default::default()
        };

        self.docker
            .list_images(Some(options))
            .await
            .context("Failed to list images")
    }

    pub async fn list_volumes(&self) -> Result<bollard::models::VolumeListResponse> {
        let options = ListVolumesOptions::<String> {
            ..Default::default()
        };

        self.docker
            .list_volumes(Some(options))
            .await
            .context("Failed to list volumes")
    }

    pub async fn list_networks(&self) -> Result<Vec<bollard::models::Network>> {
        let options = ListNetworksOptions::<String> {
            ..Default::default()
        };

        self.docker
            .list_networks(Some(options))
            .await
            .context("Failed to list networks")
    }

    pub async fn get_container_logs(&self, id: &str, tail: Option<usize>) -> Result<Vec<String>> {
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

    pub async fn inspect_image(&self, id: &str) -> Result<bollard::models::ImageInspect> {
        self.docker
            .inspect_image(id)
            .await
            .context(format!("Failed to inspect image {}", id))
    }

    pub async fn inspect_volume(&self, name: &str) -> Result<bollard::models::Volume> {
        self.docker
            .inspect_volume(name)
            .await
            .context(format!("Failed to inspect volume {}", name))
    }

    pub async fn inspect_network(&self, id: &str) -> Result<bollard::models::Network> {
        self.docker
            .inspect_network::<String>(id, None)
            .await
            .context(format!("Failed to inspect network {}", id))
    }

    pub async fn remove_image(&self, id: &str, force: bool) -> Result<()> {
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

    pub async fn remove_volume(&self, name: &str) -> Result<()> {
        let options = bollard::volume::RemoveVolumeOptions {
            force: false,
        };
        self.docker
            .remove_volume(name, Some(options))
            .await
            .context(format!("Failed to remove volume {}", name))?;
        Ok(())
    }

    pub async fn remove_network(&self, id: &str) -> Result<()> {
        self.docker
            .remove_network(id)
            .await
            .context(format!("Failed to remove network {}", id))?;
        Ok(())
    }
}