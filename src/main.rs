use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dkr::App;
use dkr::app::AppTab;
use dkr::docker::ContainerSummary;
use dkr::event::{handle_key_event, Action};
use dkr::ui::{render, LogViewer, InspectViewer};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = match App::new().await {
        Ok(app) => app,
        Err(e) => {
            // Cleanup terminal before printing error
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.show_cursor()?;
            eprintln!("Failed to initialize application: {}", e);
            std::process::exit(1);
        }
    };

    // Run the app
    let res = run_app(&mut terminal, &mut app).await;

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Application error: {}", err);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    
    // Initial data fetch
    let mut containers = Vec::new();
    let mut images = Vec::new();
    let mut volumes = None;
    let mut networks = Vec::new();
    
    // Fetch initial data
    fetch_data(app, &mut containers, &mut images, &mut volumes, &mut networks).await?;

    // Start refresh timer
    let mut refresh_interval = time::interval(Duration::from_secs(app.config.general.refresh_interval));

    loop {
        // Draw UI
        terminal.draw(|f| {
            if let Err(e) = render(f, app, &containers, &images, &volumes, &networks) {
                log::error!("Failed to render: {}", e);
                // Show error to user instead of just logging
                app.set_error(format!("Render error: {}", e));
            }
        })?;

        // Handle events
        tokio::select! {
            _ = refresh_interval.tick() => {
                if app.config.general.auto_refresh && !app.show_help {
                    fetch_data(app, &mut containers, &mut images, &mut volumes, &mut networks).await?;
                }
            }
            _ = tokio::time::timeout(Duration::from_millis(250), tokio::task::yield_now()) => {
                // Check for keyboard events
                if crossterm::event::poll(Duration::from_millis(0))? {
                    if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                        if app.show_help {
                            app.show_help = false;
                            continue;
                        }
                        
                        if app.show_confirm_delete {
                            // Handle delete confirmation
                            use crossterm::event::KeyCode;
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') => {
                                    // Proceed with deletion based on current tab
                                    let ids = app.pending_delete_ids.clone();
                                    for id in ids {
                                        let result = {
                                            let docker = app.docker.lock().await;
                                            match app.current_tab {
                                                AppTab::Containers => docker.remove_container(&id, false).await,
                                                AppTab::Images => docker.remove_image(&id, false).await,
                                                AppTab::Volumes => docker.remove_volume(&id).await,
                                                AppTab::Networks => docker.remove_network(&id).await,
                                            }
                                        };
                                        if let Err(e) = result {
                                            let item_type = match app.current_tab {
                                                AppTab::Containers => "container",
                                                AppTab::Images => "image",
                                                AppTab::Volumes => "volume",
                                                AppTab::Networks => "network",
                                            };
                                            app.set_error(format!("Failed to remove {} {}: {}", item_type, id, e));
                                        }
                                    }
                                    app.pending_delete_ids.clear();
                                    app.clear_selection();
                                    // Refresh data after deletion
                                    fetch_data(app, &mut containers, &mut images, &mut volumes, &mut networks).await?;
                                    app.show_confirm_delete = false;
                                }
                                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                                    // Cancel deletion
                                    app.show_confirm_delete = false;
                                    app.pending_delete_ids.clear();
                                }
                                _ => {}
                            }
                            continue;
                        }
                        
                        if app.show_inspect {
                            // Handle inspect viewer keys
                            if let Some(ref mut viewer) = app.inspect_viewer {
                                use crossterm::event::KeyCode;
                                match key.code {
                                    KeyCode::Char('q') | KeyCode::Esc => {
                                        app.show_inspect = false;
                                        app.inspect_viewer = None;
                                    }
                                    KeyCode::Char('j') | KeyCode::Down => viewer.scroll_down(1),
                                    KeyCode::Char('k') | KeyCode::Up => viewer.scroll_up(1),
                                    KeyCode::PageDown => viewer.page_down(10),
                                    KeyCode::PageUp => viewer.page_up(10),
                                    KeyCode::Home => viewer.scroll_to_top(),
                                    KeyCode::End => viewer.scroll_to_bottom(),
                                    _ => {}
                                }
                            }
                            continue;
                        }
                        
                        if app.show_logs {
                            // Handle log viewer keys
                            if let Some(ref mut viewer) = app.log_viewer {
                                use crossterm::event::KeyCode;
                                match key.code {
                                    KeyCode::Char('q') | KeyCode::Esc => {
                                        app.show_logs = false;
                                        app.log_viewer = None;
                                    }
                                    KeyCode::Char('j') | KeyCode::Down => viewer.scroll_down(1),
                                    KeyCode::Char('k') | KeyCode::Up => viewer.scroll_up(1),
                                    KeyCode::PageDown => viewer.page_down(10),
                                    KeyCode::PageUp => viewer.page_up(10),
                                    KeyCode::Home => viewer.scroll_to_top(),
                                    KeyCode::End => viewer.scroll_to_bottom(),
                                    KeyCode::Char('f') => viewer.toggle_follow(),
                                    KeyCode::Char('n') => viewer.find_next(),
                                    KeyCode::Char('N') => viewer.find_previous(),
                                    _ => {}
                                }
                            }
                            continue;
                        }
                        
                        if let Some(action) = handle_key_event(key) {
                            if !handle_action(app, action, &containers, &images, &volumes, &networks).await? {
                                break;
                            }
                            
                            // Refresh data after container operations or tab switches
                            if matches!(action, Action::StartStop | Action::Restart | Action::Delete 
                                | Action::NextTab | Action::PreviousTab | Action::SwitchToTab(_)) {
                                fetch_data(app, &mut containers, &mut images, &mut volumes, &mut networks).await?;
                            }
                        }
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

async fn fetch_data(
    app: &mut App,
    containers: &mut Vec<ContainerSummary>,
    images: &mut Vec<bollard::models::ImageSummary>,
    volumes: &mut Option<bollard::models::VolumeListResponse>,
    networks: &mut Vec<bollard::models::Network>,
) -> Result<()> {
    let result = {
        let docker = app.docker.lock().await;
        
        match app.current_tab {
            AppTab::Containers => {
                docker.list_containers(true).await.map(|data| {
                    *containers = data;
                })
            }
            AppTab::Images => {
                docker.list_images().await.map(|data| {
                    *images = data;
                })
            }
            AppTab::Volumes => {
                docker.list_volumes().await.map(|data| {
                    *volumes = Some(data);
                })
            }
            AppTab::Networks => {
                docker.list_networks().await.map(|data| {
                    *networks = data;
                })
            }
        }
    };
    
    // Validate selected_index after data refresh
    match app.current_tab {
        AppTab::Containers => {
            if app.selected_index >= containers.len() && !containers.is_empty() {
                app.selected_index = containers.len() - 1;
            }
        }
        AppTab::Images => {
            if app.selected_index >= images.len() && !images.is_empty() {
                app.selected_index = images.len() - 1;
            }
        }
        AppTab::Volumes => {
            if let Some(response) = volumes {
                if let Some(vols) = &response.volumes {
                    if app.selected_index >= vols.len() && !vols.is_empty() {
                        app.selected_index = vols.len() - 1;
                    }
                }
            }
        }
        AppTab::Networks => {
            if app.selected_index >= networks.len() && !networks.is_empty() {
                app.selected_index = networks.len() - 1;
            }
        }
    }
    
    if let Err(e) = result {
        let msg = match app.current_tab {
            AppTab::Containers => format!("Failed to fetch containers: {}", e),
            AppTab::Images => format!("Failed to fetch images: {}", e),
            AppTab::Volumes => format!("Failed to fetch volumes: {}", e),
            AppTab::Networks => format!("Failed to fetch networks: {}", e),
        };
        app.set_error(msg);
    }
    
    Ok(())
}

async fn handle_action(
    app: &mut App,
    action: Action,
    containers: &[ContainerSummary],
    images: &[bollard::models::ImageSummary],
    volumes: &Option<bollard::models::VolumeListResponse>,
    networks: &[bollard::models::Network],
) -> Result<bool> {
    match action {
        Action::Quit => {
            app.quit();
            return Ok(false);
        }
        Action::NextTab => {
            app.next_tab();
            return Ok(true); // Return early to trigger data fetch
        }
        Action::PreviousTab => {
            app.previous_tab();
            return Ok(true); // Return early to trigger data fetch
        }
        Action::SwitchToTab(index) => {
            let tabs = AppTab::all();
            if index < tabs.len() {
                app.current_tab = tabs[index];
                app.selected_index = 0;
                app.clear_selection(); // Clear selection when switching tabs
                return Ok(true); // Return early to trigger data fetch
            }
        }
        Action::Up => {
            if app.selected_index > 0 {
                app.selected_index -= 1;
            }
        }
        Action::Down => {
            let max_items = match app.current_tab {
                AppTab::Containers => containers.len(),
                _ => 10, // Default for other tabs
            };
            app.select_next(max_items);
        }
        Action::PageUp => {
            app.selected_index = app.selected_index.saturating_sub(10);
        }
        Action::PageDown => {
            let max_items = match app.current_tab {
                AppTab::Containers => containers.len(),
                _ => 10,
            };
            app.selected_index = (app.selected_index + 10).min(max_items.saturating_sub(1));
        }
        Action::Home => app.select_first(),
        Action::End => {
            let max_items = match app.current_tab {
                AppTab::Containers => containers.len(),
                _ => 10,
            };
            app.select_last(max_items);
        }
        Action::MultiSelect => {
            // Toggle selection for current item
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
                    if let Some(response) = volumes {
                        if let Some(vols) = &response.volumes {
                            if let Some(volume) = vols.get(app.selected_index) {
                                app.toggle_selection(volume.name.clone());
                            }
                        }
                    }
                }
                AppTab::Networks => {
                    if let Some(network) = networks.get(app.selected_index) {
                        if let Some(id) = &network.id {
                            app.toggle_selection(id.clone());
                        }
                    }
                }
            }
        }
        Action::StartStop => {
            if app.current_tab == AppTab::Containers {
                // Ensure selected_index is within bounds
                if app.selected_index >= containers.len() {
                    app.selected_index = containers.len().saturating_sub(1);
                }
                
                if let Some(container) = containers.get(app.selected_index) {
                    let result = {
                        let docker = app.docker.lock().await;
                        if container.state == dkr::docker::ContainerState::Running {
                            docker.stop_container(&container.id).await
                        } else {
                            docker.start_container(&container.id).await
                        }
                    };
                    
                    if let Err(e) = result {
                        app.set_error(format!("Operation failed: {}", e));
                    }
                }
            }
        }
        Action::Restart => {
            if app.current_tab == AppTab::Containers {
                // Ensure selected_index is within bounds
                if app.selected_index >= containers.len() {
                    app.selected_index = containers.len().saturating_sub(1);
                }
                
                if let Some(container) = containers.get(app.selected_index) {
                    let result = {
                        let docker = app.docker.lock().await;
                        docker.restart_container(&container.id).await
                    };
                    if let Err(e) = result {
                        app.set_error(format!("Failed to restart container: {}", e));
                    }
                }
            }
        }
        Action::Delete => {
            match app.current_tab {
                AppTab::Containers => {
                    // Prepare list of items to delete
                    let mut ids_to_delete = Vec::new();
                    
                    if app.has_selection() {
                        // Use multi-selection
                        ids_to_delete = app.selected_items.iter().cloned().collect();
                    } else {
                        // Use single selection
                        if app.selected_index >= containers.len() {
                            app.selected_index = containers.len().saturating_sub(1);
                        }
                        
                        if let Some(container) = containers.get(app.selected_index) {
                            ids_to_delete.push(container.id.clone());
                        }
                    }
                    
                    if !ids_to_delete.is_empty() {
                        // Check if confirmation is enabled in config
                        if app.config.general.confirm_delete {
                            // Show confirmation dialog
                            app.show_confirm_delete = true;
                            app.pending_delete_ids = ids_to_delete;
                        } else {
                            // Delete without confirmation
                            for id in ids_to_delete {
                                let result = {
                                    let docker = app.docker.lock().await;
                                    docker.remove_container(&id, false).await
                                };
                                if let Err(e) = result {
                                    app.set_error(format!("Failed to remove container {}: {}", id, e));
                                }
                            }
                            app.clear_selection();
                        }
                    }
                }
                AppTab::Images => {
                    // Prepare list of items to delete
                    let mut ids_to_delete = Vec::new();
                    
                    if app.has_selection() {
                        // Use multi-selection
                        ids_to_delete = app.selected_items.iter().cloned().collect();
                    } else {
                        // Use single selection
                        if app.selected_index >= images.len() {
                            app.selected_index = images.len().saturating_sub(1);
                        }
                        
                        if let Some(image) = images.get(app.selected_index) {
                            ids_to_delete.push(image.id.clone());
                        }
                    }
                    
                    if !ids_to_delete.is_empty() {
                        // Check if confirmation is enabled in config
                        if app.config.general.confirm_delete {
                            // Show confirmation dialog
                            app.show_confirm_delete = true;
                            app.pending_delete_ids = ids_to_delete;
                        } else {
                            // Delete without confirmation
                            for id in ids_to_delete {
                                let result = {
                                    let docker = app.docker.lock().await;
                                    docker.remove_image(&id, false).await
                                };
                                if let Err(e) = result {
                                    app.set_error(format!("Failed to remove image {}: {}", id, e));
                                }
                            }
                            app.clear_selection();
                        }
                    }
                }
                AppTab::Volumes => {
                    // Prepare list of items to delete
                    let mut ids_to_delete = Vec::new();
                    
                    if app.has_selection() {
                        // Use multi-selection
                        ids_to_delete = app.selected_items.iter().cloned().collect();
                    } else {
                        if let Some(response) = volumes {
                            if let Some(vols) = &response.volumes {
                                if app.selected_index >= vols.len() {
                                    app.selected_index = vols.len().saturating_sub(1);
                                }
                                
                                if let Some(volume) = vols.get(app.selected_index) {
                                    ids_to_delete.push(volume.name.clone());
                                }
                            }
                        }
                    }
                    
                    if !ids_to_delete.is_empty() {
                        // Check if confirmation is enabled in config
                        if app.config.general.confirm_delete {
                            // Show confirmation dialog
                            app.show_confirm_delete = true;
                            app.pending_delete_ids = ids_to_delete;
                        } else {
                            // Delete without confirmation
                            for name in ids_to_delete {
                                let result = {
                                    let docker = app.docker.lock().await;
                                    docker.remove_volume(&name).await
                                };
                                if let Err(e) = result {
                                    app.set_error(format!("Failed to remove volume {}: {}", name, e));
                                }
                            }
                            app.clear_selection();
                        }
                    }
                }
                AppTab::Networks => {
                    // Prepare list of items to delete
                    let mut ids_to_delete = Vec::new();
                    
                    if app.has_selection() {
                        // Use multi-selection
                        ids_to_delete = app.selected_items.iter().cloned().collect();
                    } else {
                        // Use single selection
                        if app.selected_index >= networks.len() {
                            app.selected_index = networks.len().saturating_sub(1);
                        }
                        
                        if let Some(network) = networks.get(app.selected_index) {
                            if let Some(id) = &network.id {
                                ids_to_delete.push(id.clone());
                            }
                        }
                    }
                    
                    if !ids_to_delete.is_empty() {
                        // Check if confirmation is enabled in config
                        if app.config.general.confirm_delete {
                            // Show confirmation dialog
                            app.show_confirm_delete = true;
                            app.pending_delete_ids = ids_to_delete;
                        } else {
                            // Delete without confirmation
                            for id in ids_to_delete {
                                let result = {
                                    let docker = app.docker.lock().await;
                                    docker.remove_network(&id).await
                                };
                                if let Err(e) = result {
                                    app.set_error(format!("Failed to remove network {}: {}", id, e));
                                }
                            }
                            app.clear_selection();
                        }
                    }
                }
            }
        }
        Action::ViewLogs => {
            if app.current_tab == AppTab::Containers {
                // Ensure selected_index is within bounds
                if app.selected_index >= containers.len() {
                    app.selected_index = containers.len().saturating_sub(1);
                }
                
                if let Some(container) = containers.get(app.selected_index) {
                    // Fetch logs
                    let logs = {
                        let docker = app.docker.lock().await;
                        docker.get_container_logs(&container.id, Some(100)).await
                    };
                    
                    match logs {
                        Ok(log_lines) => {
                            let mut viewer = LogViewer::new(container.name.clone());
                            viewer.set_logs(log_lines);
                            app.log_viewer = Some(viewer);
                            app.show_logs = true;
                        }
                        Err(e) => {
                            app.set_error(format!("Failed to fetch logs: {}", e));
                        }
                    }
                }
            }
        }
        Action::Inspect => {
            match app.current_tab {
                AppTab::Containers => {
                    // Ensure selected_index is within bounds
                    if app.selected_index >= containers.len() {
                        app.selected_index = containers.len().saturating_sub(1);
                    }
                    
                    if let Some(container) = containers.get(app.selected_index) {
                        // Fetch container details
                        let details = {
                            let docker = app.docker.lock().await;
                            docker.get_container(&container.id).await
                        };
                        
                        match details {
                            Ok(info) => {
                                let json_value = serde_json::to_value(&info).unwrap_or(serde_json::Value::Null);
                                let viewer = InspectViewer::new(
                                    format!("Container: {}", container.name),
                                    json_value
                                );
                                app.inspect_viewer = Some(viewer);
                                app.show_inspect = true;
                            }
                            Err(e) => {
                                app.set_error(format!("Failed to inspect container: {}", e));
                            }
                        }
                    }
                }
                AppTab::Images => {
                    // Ensure selected_index is within bounds
                    if app.selected_index >= images.len() {
                        app.selected_index = images.len().saturating_sub(1);
                    }
                    
                    if let Some(image) = images.get(app.selected_index) {
                        // Get the image ID (remove sha256: prefix if present)
                        let image_id = image.id.strip_prefix("sha256:").unwrap_or(&image.id);
                        
                        // Fetch image details
                        let details = {
                            let docker = app.docker.lock().await;
                            docker.inspect_image(image_id).await
                        };
                        
                        match details {
                            Ok(info) => {
                                let json_value = serde_json::to_value(&info).unwrap_or(serde_json::Value::Null);
                                let name = image.repo_tags.first()
                                    .cloned()
                                    .unwrap_or_else(|| image_id.chars().take(12).collect());
                                let viewer = InspectViewer::new(
                                    format!("Image: {}", name),
                                    json_value
                                );
                                app.inspect_viewer = Some(viewer);
                                app.show_inspect = true;
                            }
                            Err(e) => {
                                app.set_error(format!("Failed to inspect image: {}", e));
                            }
                        }
                    }
                }
                AppTab::Volumes => {
                    if let Some(response) = volumes {
                        if let Some(vols) = &response.volumes {
                            // Ensure selected_index is within bounds
                            if app.selected_index >= vols.len() {
                                app.selected_index = vols.len().saturating_sub(1);
                            }
                            
                            if let Some(volume) = vols.get(app.selected_index) {
                                // Fetch volume details
                                let details = {
                                    let docker = app.docker.lock().await;
                                    docker.inspect_volume(&volume.name).await
                                };
                                
                                match details {
                                    Ok(info) => {
                                        let json_value = serde_json::to_value(&info).unwrap_or(serde_json::Value::Null);
                                        let viewer = InspectViewer::new(
                                            format!("Volume: {}", volume.name),
                                            json_value
                                        );
                                        app.inspect_viewer = Some(viewer);
                                        app.show_inspect = true;
                                    }
                                    Err(e) => {
                                        app.set_error(format!("Failed to inspect volume: {}", e));
                                    }
                                }
                            }
                        }
                    }
                }
                AppTab::Networks => {
                    // Ensure selected_index is within bounds
                    if app.selected_index >= networks.len() {
                        app.selected_index = networks.len().saturating_sub(1);
                    }
                    
                    if let Some(network) = networks.get(app.selected_index) {
                        let network_id = network.id.as_ref()
                            .map(|s| s.clone())
                            .unwrap_or_else(|| String::new());
                        
                        // Fetch network details
                        let details = {
                            let docker = app.docker.lock().await;
                            docker.inspect_network(&network_id).await
                        };
                        
                        match details {
                            Ok(info) => {
                                let json_value = serde_json::to_value(&info).unwrap_or(serde_json::Value::Null);
                                let name = network.name.as_ref()
                                    .cloned()
                                    .unwrap_or_else(|| "<unknown>".to_string());
                                let viewer = InspectViewer::new(
                                    format!("Network: {}", name),
                                    json_value
                                );
                                app.inspect_viewer = Some(viewer);
                                app.show_inspect = true;
                            }
                            Err(e) => {
                                app.set_error(format!("Failed to inspect network: {}", e));
                            }
                        }
                    }
                }
            }
        }
        Action::Help => app.show_help = true,
        Action::Refresh => {
            app.clear_error();
        }
        Action::SelectAll => {
            // Select all items in current tab
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
                    if let Some(response) = volumes {
                        if let Some(vols) = &response.volumes {
                            for volume in vols {
                                app.selected_items.insert(volume.name.clone());
                            }
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
        Action::Escape => {
            app.clear_error();
            app.show_help = false;
            app.clear_selection(); // Clear selection on Escape
        }
        _ => {}
    }
    
    Ok(true)
}