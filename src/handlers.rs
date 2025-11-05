//! Action handlers for user interactions
//!
//! Contains functions that handle various user actions like navigation,
//! container operations, deletion, log viewing, etc.

use crate::app::{App, AppTab};
use crate::docker::ContainerSummary;
use crate::event::Action;
use crate::ui::{InspectViewer, LogViewer, StatsViewer};
use anyhow::{Context, Result};
use bollard::models::{ImageSummary, Network, VolumeListResponse};

/// Handle navigation actions (Up, Down, PageUp, PageDown, Home, End)
pub fn handle_navigation(
    app: &mut App,
    action: Action,
    containers: &[ContainerSummary],
    images: &[ImageSummary],
    volumes: &Option<VolumeListResponse>,
    networks: &[Network],
) {
    let max_items = match app.current_tab {
        AppTab::Containers => containers.len(),
        AppTab::Images => images.len(),
        AppTab::Volumes => volumes
            .as_ref()
            .and_then(|v| v.volumes.as_ref())
            .map(|v| v.len())
            .unwrap_or(0),
        AppTab::Networks => networks.len(),
    };

    match action {
        Action::Up => {
            app.select_previous();
        }
        Action::Down => {
            app.select_next(max_items);
        }
        Action::PageUp => {
            app.selected_index = app.selected_index.saturating_sub(10);
        }
        Action::PageDown => {
            app.selected_index = (app.selected_index + 10).min(max_items.saturating_sub(1));
        }
        Action::Home => app.select_first(),
        Action::End => app.select_last(max_items),
        _ => {}
    }
}

/// Handle tab switching actions (NextTab, PreviousTab, SwitchToTab)
pub fn handle_tab_switch(app: &mut App, action: Action) -> bool {
    match action {
        Action::NextTab => {
            app.next_tab();
            true // Trigger data fetch
        }
        Action::PreviousTab => {
            app.previous_tab();
            true // Trigger data fetch
        }
        Action::SwitchToTab(index) => {
            let tabs = AppTab::all();
            if index < tabs.len() {
                app.current_tab = tabs[index];
                app.selected_index = 0;
                app.clear_selection();
                true // Trigger data fetch
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Handle selection actions (MultiSelect, SelectAll)
pub fn handle_selection(
    app: &mut App,
    action: Action,
    containers: &[ContainerSummary],
    images: &[ImageSummary],
    volumes: &Option<VolumeListResponse>,
    networks: &[Network],
) {
    match action {
        Action::MultiSelect => {
            match app.current_tab {
                AppTab::Containers => {
                    if let Some(container) = containers.get(app.selected_index) {
                        app.toggle_selection(container.id.clone());
                    }
                }
                AppTab::Images => {
                    if let Some(image) = images.get(app.selected_index) {
                        app.toggle_selection(image.id.clone());
                    }
                }
                AppTab::Volumes => {
                    if let Some(response) = volumes
                        && let Some(vols) = &response.volumes
                            && let Some(volume) = vols.get(app.selected_index) {
                                app.toggle_selection(volume.name.clone());
                            }
                }
                AppTab::Networks => {
                    if let Some(network) = networks.get(app.selected_index)
                        && let Some(id) = &network.id {
                            app.toggle_selection(id.clone());
                        }
                }
            }
        }
        Action::SelectAll => {
            app.clear_selection();
            match app.current_tab {
                AppTab::Containers => {
                    for container in containers {
                        app.selected_items.insert(container.id.clone());
                    }
                }
                AppTab::Images => {
                    for image in images {
                        app.selected_items.insert(image.id.clone());
                    }
                }
                AppTab::Volumes => {
                    if let Some(response) = volumes
                        && let Some(vols) = &response.volumes {
                            for volume in vols {
                                app.selected_items.insert(volume.name.clone());
                            }
                        }
                }
                AppTab::Networks => {
                    for network in networks {
                        if let Some(id) = &network.id {
                            app.selected_items.insert(id.clone());
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

/// Handle container operations (Start/Stop, Restart)
pub async fn handle_container_operations(
    app: &mut App,
    action: Action,
    containers: &[ContainerSummary],
) -> Result<()> {
    match action {
        Action::StartStop => {
            if let Some(container) = containers.get(app.selected_index) {
                let docker = app.docker.lock().await;
                match container.state.as_str() {
                    "Running" => {
                        docker.stop_container(&container.id).await.context("Failed to stop container")?;
                    }
                    _ => {
                        docker.start_container(&container.id).await.context("Failed to start container")?;
                    }
                }
            }
        }
        Action::Restart => {
            if let Some(container) = containers.get(app.selected_index) {
                let docker = app.docker.lock().await;
                docker.restart_container(&container.id).await.context("Failed to restart container")?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Handle delete confirmation and execution
pub async fn handle_delete_action(
    app: &mut App,
    containers: &[ContainerSummary],
    images: &[ImageSummary],
    volumes: &Option<VolumeListResponse>,
    networks: &[Network],
) -> Result<()> {
    // Prepare list of items to delete
    let mut ids_to_delete = Vec::new();

    match app.current_tab {
        AppTab::Containers => {
            if app.has_selection() {
                ids_to_delete = app.selected_items.iter().cloned().collect();
            } else if let Some(container) = containers.get(app.selected_index) {
                ids_to_delete.push(container.id.clone());
            }

            if !ids_to_delete.is_empty() {
                if app.config.general.confirm_delete {
                    app.show_confirm_delete = true;
                    app.pending_delete_ids = ids_to_delete;
                } else {
                    for id in &ids_to_delete {
                        let result = {
                            let docker = app.docker.lock().await;
                            docker.remove_container(id, false).await
                        };
                        if let Err(e) = result {
                            app.set_error(format!("Failed to remove container: {}", e));
                        }
                    }
                    app.clear_selection();
                }
            }
        }
        AppTab::Images => {
            if app.has_selection() {
                ids_to_delete = app.selected_items.iter().cloned().collect();
            } else if let Some(image) = images.get(app.selected_index) {
                ids_to_delete.push(image.id.clone());
            }

            if !ids_to_delete.is_empty() {
                if app.config.general.confirm_delete {
                    app.show_confirm_delete = true;
                    app.pending_delete_ids = ids_to_delete;
                } else {
                    for id in &ids_to_delete {
                        let result = {
                            let docker = app.docker.lock().await;
                            docker.remove_image(id, false).await
                        };
                        if let Err(e) = result {
                            app.set_error(format!("Failed to remove image: {}", e));
                        }
                    }
                    app.clear_selection();
                }
            }
        }
        AppTab::Volumes => {
            if app.has_selection() {
                ids_to_delete = app.selected_items.iter().cloned().collect();
            } else if let Some(response) = volumes
                && let Some(vols) = &response.volumes {
                    if app.selected_index >= vols.len() {
                        app.selected_index = vols.len().saturating_sub(1);
                    }

                    if let Some(volume) = vols.get(app.selected_index) {
                        ids_to_delete.push(volume.name.clone());
                    }
                }

            if !ids_to_delete.is_empty() {
                if app.config.general.confirm_delete {
                    app.show_confirm_delete = true;
                    app.pending_delete_ids = ids_to_delete;
                } else {
                    for name in &ids_to_delete {
                        let result = {
                            let docker = app.docker.lock().await;
                            docker.remove_volume(name).await
                        };
                        if let Err(e) = result {
                            app.set_error(format!("Failed to remove volume: {}", e));
                        }
                    }
                    app.clear_selection();
                }
            }
        }
        AppTab::Networks => {
            if app.has_selection() {
                ids_to_delete = app.selected_items.iter().cloned().collect();
            } else if let Some(network) = networks.get(app.selected_index)
                && let Some(id) = &network.id {
                    ids_to_delete.push(id.clone());
                }

            if !ids_to_delete.is_empty() {
                if app.config.general.confirm_delete {
                    app.show_confirm_delete = true;
                    app.pending_delete_ids = ids_to_delete;
                } else {
                    for id in &ids_to_delete {
                        let result = {
                            let docker = app.docker.lock().await;
                            docker.remove_network(id).await
                        };
                        if let Err(e) = result {
                            app.set_error(format!("Failed to remove network: {}", e));
                        }
                    }
                    app.clear_selection();
                }
            }
        }
    }

    Ok(())
}

/// Handle view logs action
pub async fn handle_view_logs(
    app: &mut App,
    containers: &[ContainerSummary],
) -> Result<()> {
    if app.current_tab == AppTab::Containers
        && let Some(container) = containers.get(app.selected_index) {
        let logs_result = {
            let docker = app.docker.lock().await;
            docker.get_container_logs(&container.id, Some(100)).await
        };

        match logs_result {
            Ok(logs) => {
                let mut viewer = LogViewer::new(container.name.clone());
                viewer.set_logs(logs);
                app.log_viewer = Some(viewer);
                app.show_logs = true;
            }
            Err(e) => {
                app.set_error(format!("Failed to fetch logs: {}", e));
            }
        }
    }
    Ok(())
}


/// Handle inspect action
pub async fn handle_inspect(
    app: &mut App,
    containers: &[ContainerSummary],
    images: &[ImageSummary],
    volumes: &Option<VolumeListResponse>,
    networks: &[Network],
) -> Result<()> {
    match app.current_tab {
        AppTab::Containers => {
            if let Some(container) = containers.get(app.selected_index) {
                let result = {
                    let docker = app.docker.lock().await;
                    docker.get_container(&container.id).await
                };

                match result {
                    Ok(inspect_data) => {
                        let json_value = serde_json::to_value(&inspect_data)
                            .unwrap_or(serde_json::Value::Null);
                        app.inspect_viewer = Some(InspectViewer::new(
                            format!("Container: {}", container.name),
                            json_value,
                        ));
                        app.show_inspect = true;
                    }
                    Err(e) => {
                        app.set_error(format!("Failed to inspect container: {}", e));
                    }
                }
            }
        }
        AppTab::Images => {
            if let Some(image) = images.get(app.selected_index) {
                let image_id = image.id.strip_prefix("sha256:").unwrap_or(&image.id);
                let image_id = image_id.to_string();

                let result = {
                    let docker = app.docker.lock().await;
                    docker.inspect_image(&image_id).await
                };

                match result {
                    Ok(inspect_data) => {
                        let json_value = serde_json::to_value(&inspect_data)
                            .unwrap_or(serde_json::Value::Null);
                        let title = image
                            .repo_tags
                            .first()
                            .map(|tag| format!("Image: {}", tag))
                            .unwrap_or_else(|| format!("Image: {}", image_id.chars().take(12).collect::<String>()));
                        app.inspect_viewer = Some(InspectViewer::new(title, json_value));
                        app.show_inspect = true;
                    }
                    Err(e) => {
                        app.set_error(format!("Failed to inspect image: {}", e));
                    }
                }
            }
        }
        AppTab::Volumes => {
            if let Some(response) = volumes
                && let Some(vols) = &response.volumes {
                    if app.selected_index >= vols.len() {
                        app.selected_index = vols.len().saturating_sub(1);
                    }

                    if let Some(volume) = vols.get(app.selected_index) {
                        let result = {
                            let docker = app.docker.lock().await;
                            docker.inspect_volume(&volume.name).await
                        };

                        match result {
                            Ok(inspect_data) => {
                                let json_value = serde_json::to_value(&inspect_data)
                                    .unwrap_or(serde_json::Value::Null);
                                app.inspect_viewer = Some(InspectViewer::new(
                                    format!("Volume: {}", volume.name),
                                    json_value,
                                ));
                                app.show_inspect = true;
                            }
                            Err(e) => {
                                app.set_error(format!("Failed to inspect volume: {}", e));
                            }
                        }
                    }
                }
        }
        AppTab::Networks => {
            if app.selected_index >= networks.len() {
                app.selected_index = networks.len().saturating_sub(1);
            }

            if let Some(network) = networks.get(app.selected_index) {
                let network_id = network.id.clone()
                    .unwrap_or_else(String::new);

                let result = {
                    let docker = app.docker.lock().await;
                    docker.inspect_network(&network_id).await
                };

                match result {
                    Ok(inspect_data) => {
                        let json_value = serde_json::to_value(&inspect_data)
                            .unwrap_or(serde_json::Value::Null);
                        let title = network
                            .name
                            .as_ref()
                            .map(|name| format!("Network: {}", name))
                            .unwrap_or_else(|| "Network".to_string());
                        app.inspect_viewer = Some(InspectViewer::new(title, json_value));
                        app.show_inspect = true;
                    }
                    Err(e) => {
                        app.set_error(format!("Failed to inspect network: {}", e));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Confirm and execute delete operation
pub async fn confirm_delete(app: &mut App) -> Result<()> {
    if app.show_confirm_delete {
        let ids = app.pending_delete_ids.clone();
        let current_tab = app.current_tab;
        app.show_confirm_delete = false;
        app.pending_delete_ids.clear();

        for id in &ids {
            let result = {
                let docker = app.docker.lock().await;
                match current_tab {
                    AppTab::Containers => docker.remove_container(id, false).await,
                    AppTab::Images => docker.remove_image(id, false).await,
                    AppTab::Volumes => docker.remove_volume(id).await,
                    AppTab::Networks => docker.remove_network(id).await,
                }
            };
            if let Err(e) = result {
                let item_type = match current_tab {
                    AppTab::Containers => "container",
                    AppTab::Images => "image",
                    AppTab::Volumes => "volume",
                    AppTab::Networks => "network",
                };
                app.set_error(format!("Failed to remove {} {}: {}", item_type, id, e));
            }
        }
        app.clear_selection();
    }
    Ok(())
}

/// Handle viewing container statistics
pub async fn handle_view_stats(
    app: &mut App,
    containers: &[ContainerSummary],
) -> Result<()> {
    if app.current_tab != AppTab::Containers {
        return Ok(());
    }

    if let Some(container) = containers.get(app.selected_index) {
        let container_id = container.id.clone();
        let container_name = container.name.clone();

        let result = {
            let docker = app.docker.lock().await;
            docker.get_container_stats(&container_id).await
        };

        match result {
            Ok(stats) => {
                app.stats_viewer = Some(StatsViewer::new(container_name, stats));
                app.show_stats = true;
            }
            Err(e) => {
                app.set_error(format!("Failed to get container stats: {}", e));
            }
        }
    }

    Ok(())
}

/// Handle pulling a Docker image
pub async fn handle_pull_image(
    app: &mut App,
    images: &[ImageSummary],
) -> Result<()> {
    if app.current_tab != AppTab::Images {
        return Ok(());
    }

    if let Some(image) = images.get(app.selected_index) {
        if let Some(tag) = image.repo_tags.first() {
            let image_name = tag.clone();

            let result = {
                let docker = app.docker.lock().await;
                docker.pull_image(&image_name).await
            };

            match result {
                Ok(()) => {
                    // Success - could add a success message if needed
                }
                Err(e) => {
                    app.set_error(format!("Failed to pull image {}: {}", image_name, e));
                }
            }
        } else {
            app.set_error("No image tag available".to_string());
        }
    }

    Ok(())
}
