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
use dkr::handlers;
use dkr::ui::render;
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
                if crossterm::event::poll(Duration::from_millis(0))?
                    && let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                        if app.show_help {
                            app.show_help = false;
                            continue;
                        }
                        
                        if app.show_confirm_delete {
                            // Handle delete confirmation
                            use crossterm::event::KeyCode;
                            match key.code {
                                KeyCode::Char('y') | KeyCode::Char('Y') => {
                                    // Proceed with deletion using handler
                                    handlers::confirm_delete(app).await?;
                                    // Refresh data after deletion
                                    fetch_data(app, &mut containers, &mut images, &mut volumes, &mut networks).await?;
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

                        if app.show_stats {
                            // Handle stats viewer keys
                            use crossterm::event::KeyCode;
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    app.show_stats = false;
                                    app.stats_viewer = None;
                                }
                                _ => {}
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
            if let Some(response) = volumes
                && let Some(vols) = &response.volumes
                    && app.selected_index >= vols.len() && !vols.is_empty() {
                        app.selected_index = vols.len() - 1;
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

        // Tab switching
        Action::NextTab | Action::PreviousTab | Action::SwitchToTab(_) => {
            let should_fetch = handlers::handle_tab_switch(app, action);
            return Ok(should_fetch);
        }

        // Navigation
        Action::Up | Action::Down | Action::PageUp | Action::PageDown | Action::Home | Action::End => {
            handlers::handle_navigation(app, action, containers, images, volumes, networks);
        }

        // Selection
        Action::MultiSelect | Action::SelectAll => {
            handlers::handle_selection(app, action, containers, images, volumes, networks);
        }

        // Container operations
        Action::StartStop | Action::Restart => {
            handlers::handle_container_operations(app, action, containers).await?;
        }

        // Delete
        Action::Delete => {
            handlers::handle_delete_action(app, containers, images, volumes, networks).await?;
        }

        // View logs
        Action::ViewLogs => {
            handlers::handle_view_logs(app, containers).await?;
        }

        // View stats
        Action::ViewStats => {
            handlers::handle_view_stats(app, containers).await?;
        }

        // Inspect
        Action::Inspect => {
            handlers::handle_inspect(app, containers, images, volumes, networks).await?;
        }

        // Pull image
        Action::PullImage => {
            handlers::handle_pull_image(app, images).await?;
        }

        // Other actions
        Action::Help => {
            app.show_help = true;
        }
        Action::Refresh => {
            app.clear_status();
        }
        Action::Search => {
            // Search not yet implemented
        }
        Action::Escape => {
            app.clear_status();
            app.show_help = false;
            app.clear_selection();
        }

        Action::Select => {
            // Select is handled in the main event loop for confirmation dialogs
        }
    }

    Ok(true)
}