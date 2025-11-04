use crate::app::App;
use crate::docker::{ContainerSummary, ContainerState};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Row, Table, TableState, Paragraph},
    Frame,
};

pub fn draw_containers_tab(
    frame: &mut Frame,
    app: &App,
    containers: &[ContainerSummary],
    area: Rect,
) {
    let headers = Row::new(vec![
        Cell::from("Name"),
        Cell::from("Status"),
        Cell::from("Image"),
        Cell::from("Ports"),
        Cell::from("Created"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD))
    .height(1);

    let rows: Vec<Row> = containers
        .iter()
        .map(|container| {
            let status_color = match container.state {
                ContainerState::Running => Color::Green,
                ContainerState::Paused => Color::Yellow,
                ContainerState::Exited | ContainerState::Dead => Color::Red,
                _ => Color::Gray,
            };

            let ports_str = container
                .ports
                .iter()
                .filter_map(|p| {
                    p.public_port.map(|pub_port| {
                        format!("{}:{}", pub_port, p.private_port)
                    })
                })
                .collect::<Vec<_>>()
                .join(", ");

            let created = format_timestamp(container.created);
            
            // Add checkbox indicator for multi-selection
            let selected_mark = if app.selected_items.contains(&container.id) {
                "[✓] "
            } else {
                "[ ] "
            };
            
            let name_with_mark = format!("{}{}", selected_mark, container.name);

            Row::new(vec![
                Cell::from(name_with_mark),
                Cell::from(Span::styled(
                    container.state.as_str(),
                    Style::default().fg(status_color),
                )),
                Cell::from(container.image.clone()),
                Cell::from(ports_str),
                Cell::from(created),
            ])
            .height(1)
        })
        .collect();

    let selected_style = Style::default()
        .bg(Color::Blue)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let widths = [
        Constraint::Percentage(25),
        Constraint::Percentage(15),
        Constraint::Percentage(25),
        Constraint::Percentage(20),
        Constraint::Percentage(15),
    ];
    
    let table = Table::new(rows, widths)
        .header(headers)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Containers ({}) ", containers.len()))
        )
        .highlight_style(selected_style);

    let mut state = TableState::default();
    state.select(Some(app.selected_index));

    frame.render_stateful_widget(table, area, &mut state);
}

pub fn draw_images_tab(
    frame: &mut Frame,
    app: &App,
    images: &[bollard::models::ImageSummary],
    area: Rect,
) {
    // Show loading indicator if images are empty
    if images.is_empty() {
        let loading = Paragraph::new("Loading...")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Images ")
            )
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(loading, area);
        return;
    }

    let headers = Row::new(vec![
        Cell::from("Repository"),
        Cell::from("Tag"),
        Cell::from("Image ID"),
        Cell::from("Created"),
        Cell::from("Size"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD))
    .height(1);

    let rows: Vec<Row> = images
        .iter()
        .map(|image| {
            // Add checkbox indicator for multi-selection
            let selected_mark = if app.selected_items.contains(&image.id) {
                "[✓] "
            } else {
                "[ ] "
            };
            let repo_tags = image.repo_tags
                .first()
                .cloned()
                .unwrap_or_else(|| "<none>".to_string());
            
            let parts: Vec<&str> = repo_tags.split(':').collect();
            let repo = format!("{}{}", selected_mark, parts.first().unwrap_or(&"<none>"));
            let tag = parts.get(1).unwrap_or(&"<none>").to_string();
            
            let id = image.id
                .chars()
                .skip(7)
                .take(12)
                .collect::<String>();
            
            let created = format_timestamp(image.created);
            let size = format_size(image.size);

            Row::new(vec![
                Cell::from(repo),
                Cell::from(tag),
                Cell::from(id),
                Cell::from(created),
                Cell::from(size),
            ])
            .height(1)
        })
        .collect();

    let selected_style = Style::default()
        .bg(Color::Blue)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let widths = [
        Constraint::Percentage(30),
        Constraint::Percentage(15),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(15),
    ];
    
    let table = Table::new(rows, widths)
        .header(headers)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Images ({}) ", images.len()))
        )
        .highlight_style(selected_style);

    let mut state = TableState::default();
    state.select(Some(app.selected_index));

    frame.render_stateful_widget(table, area, &mut state);
}

pub fn draw_volumes_tab(
    frame: &mut Frame,
    app: &App,
    volumes_response: &Option<bollard::models::VolumeListResponse>,
    area: Rect,
) {
    // Show loading indicator if volumes haven't been fetched yet
    if volumes_response.is_none() {
        let loading = Paragraph::new("Loading...")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Volumes ")
            )
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(loading, area);
        return;
    }

    if let Some(response) = volumes_response {
        if let Some(volumes) = &response.volumes {
            let headers = Row::new(vec![
                Cell::from("Name"),
                Cell::from("Driver"),
                Cell::from("Mountpoint"),
                Cell::from("Created"),
            ])
            .style(Style::default().add_modifier(Modifier::BOLD))
            .height(1);

            let rows: Vec<Row> = volumes
                .iter()
                .map(|volume| {
                    // Add checkbox indicator for multi-selection
                    let selected_mark = if app.selected_items.contains(&volume.name) {
                        "[✓] "
                    } else {
                        "[ ] "
                    };
                    let name = format!("{}{}", selected_mark, volume.name);
                    let driver = volume.driver.clone();
                    let mountpoint = volume.mountpoint.clone();
                    let created = volume.created_at.as_ref().unwrap_or(&"<unknown>".to_string()).clone();

                    Row::new(vec![
                        Cell::from(name),
                        Cell::from(driver),
                        Cell::from(mountpoint),
                        Cell::from(created),
                    ])
                    .height(1)
                })
                .collect();

            let selected_style = Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD);

            let widths = [
                Constraint::Percentage(25),
                Constraint::Percentage(15),
                Constraint::Percentage(40),
                Constraint::Percentage(20),
            ];
            
            let table = Table::new(rows, widths)
                .header(headers)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" Volumes ({}) ", volumes.len()))
                )
                .highlight_style(selected_style);

            let mut state = TableState::default();
            state.select(Some(app.selected_index));

            frame.render_stateful_widget(table, area, &mut state);
        } else {
            draw_empty_message(frame, "No volumes found", area);
        }
    } else {
        draw_empty_message(frame, "Loading volumes...", area);
    }
}

pub fn draw_networks_tab(
    frame: &mut Frame,
    app: &App,
    networks: &[bollard::models::Network],
    area: Rect,
) {
    // Show loading indicator if networks are empty
    if networks.is_empty() {
        let loading = Paragraph::new("Loading...")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Networks ")
            )
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(loading, area);
        return;
    }

    let headers = Row::new(vec![
        Cell::from("Name"),
        Cell::from("ID"),
        Cell::from("Driver"),
        Cell::from("Scope"),
        Cell::from("Created"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD))
    .height(1);

    let rows: Vec<Row> = networks
        .iter()
        .map(|network| {
            // Add checkbox indicator for multi-selection
            let network_id = network.id.as_ref().unwrap_or(&"<none>".to_string()).clone();
            let selected_mark = if app.selected_items.contains(&network_id) {
                "[✓] "
            } else {
                "[ ] "
            };
            
            let name = network.name.as_ref().unwrap_or(&"<none>".to_string()).clone();
            let name_with_mark = format!("{}{}", selected_mark, name);
            
            let id = network_id
                .chars()
                .take(12)
                .collect::<String>();
            let driver = network.driver.as_ref().unwrap_or(&"<none>".to_string()).clone();
            let scope = network.scope.as_ref().unwrap_or(&"<none>".to_string()).clone();
            let created = network.created.as_ref().unwrap_or(&"<unknown>".to_string()).clone();

            Row::new(vec![
                Cell::from(name_with_mark),
                Cell::from(id),
                Cell::from(driver),
                Cell::from(scope),
                Cell::from(created),
            ])
            .height(1)
        })
        .collect();

    let selected_style = Style::default()
        .bg(Color::Blue)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let widths = [
        Constraint::Percentage(25),
        Constraint::Percentage(20),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
        Constraint::Percentage(25),
    ];
    
    let table = Table::new(rows, widths)
        .header(headers)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Networks ({}) ", networks.len()))
        )
        .highlight_style(selected_style);

    let mut state = TableState::default();
    state.select(Some(app.selected_index));

    frame.render_stateful_widget(table, area, &mut state);
}

fn draw_empty_message(frame: &mut Frame, message: &str, area: Rect) {
    let paragraph = Paragraph::new(message)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray))
        .alignment(ratatui::layout::Alignment::Center);
    
    frame.render_widget(paragraph, area);
}

fn format_timestamp(timestamp: i64) -> String {
    use chrono::{DateTime, Local, Utc};
    
    if timestamp == 0 {
        return "<unknown>".to_string();
    }
    
    let dt = DateTime::<Utc>::from_timestamp(timestamp, 0)
        .unwrap_or_else(Utc::now);
    let local_dt: DateTime<Local> = dt.into();
    let now = Local::now();
    let duration = now.signed_duration_since(local_dt);
    
    if duration.num_days() > 0 {
        format!("{} days ago", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{} hours ago", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{} minutes ago", duration.num_minutes())
    } else {
        "Just now".to_string()
    }
}

fn format_size(size: i64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.1} {}", size, UNITS[unit_index])
}