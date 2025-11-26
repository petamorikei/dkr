use super::utils::centered_rect;
use crate::app::{App, AppTab, ModalState, StatusKind};
use crate::docker::ContainerSummary;
use anyhow::Result;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
};

use super::inspect_viewer::draw_inspect_popup;
use super::log_viewer::draw_log_popup;
use super::stats_viewer::draw_stats_popup;
use super::widgets::{draw_containers_tab, draw_images_tab, draw_networks_tab, draw_volumes_tab};

pub fn render(
    frame: &mut Frame,
    app: &mut App,
    containers: &[ContainerSummary],
    images: &[bollard::models::ImageSummary],
    volumes: &Option<bollard::models::VolumeListResponse>,
    networks: &[bollard::models::Network],
) -> Result<()> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with tabs
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Footer
        ])
        .split(frame.size());

    // Draw header with tabs
    draw_header(frame, app, chunks[0]);

    // Draw main content based on current tab
    match app.current_tab {
        AppTab::Containers => draw_containers_tab(frame, app, containers, chunks[1]),
        AppTab::Images => draw_images_tab(frame, app, images, chunks[1]),
        AppTab::Volumes => draw_volumes_tab(frame, app, volumes, chunks[1]),
        AppTab::Networks => draw_networks_tab(frame, app, networks, chunks[1]),
    }

    // Draw footer with help
    draw_footer(frame, app, chunks[2]);

    // Draw status message if present
    if let Some(status) = &app.status_message {
        draw_status_popup(frame, status.kind, &status.message);
    }

    // Draw modals based on current modal state
    match app.modal {
        ModalState::Help => {
            draw_help_popup(frame);
        }
        ModalState::Logs => {
            if let Some(ref mut viewer) = app.log_viewer {
                draw_log_popup(frame, viewer);
            }
        }
        ModalState::Inspect => {
            if let Some(ref viewer) = app.inspect_viewer {
                draw_inspect_popup(frame, viewer);
            }
        }
        ModalState::Stats => {
            if let Some(ref viewer) = app.stats_viewer {
                draw_stats_popup(frame, viewer);
            }
        }
        ModalState::ConfirmDelete => {
            draw_delete_confirmation(frame, app.current_tab, app.pending_delete_ids.len());
        }
        ModalState::Search | ModalState::None => {}
    }

    Ok(())
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = AppTab::all()
        .iter()
        .map(|t| {
            let text = t.with_number();
            if *t == app.current_tab {
                // Highlight the number for selected tab
                if let Some(idx) = text.find("] ") {
                    let (number_part, name_part) = text.split_at(idx + 1);
                    Line::from(vec![
                        Span::styled(number_part.to_string(), Style::default().fg(Color::Yellow)),
                        Span::styled(name_part.to_string(), Style::default()),
                    ])
                } else {
                    Line::from(text)
                }
            } else {
                Line::from(text)
            }
        })
        .collect();

    let selected_index = AppTab::all()
        .iter()
        .position(|&t| t == app.current_tab)
        .unwrap_or(0);

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" dkr - Docker TUI "),
        )
        .select(selected_index)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    // Show search input when in search mode
    if app.modal == ModalState::Search {
        let search_text = vec![
            Span::styled("Search: ", Style::default().fg(Color::Yellow)),
            Span::styled(&app.search_query, Style::default().fg(Color::White)),
            Span::styled(
                "_",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
            Span::raw("  "),
            Span::styled("[Enter]", Style::default().fg(Color::DarkGray)),
            Span::raw(" Apply  "),
            Span::styled("[Esc]", Style::default().fg(Color::DarkGray)),
            Span::raw(" Cancel"),
        ];

        let search_bar = Paragraph::new(Line::from(search_text))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Left);

        frame.render_widget(search_bar, area);
        return;
    }

    let mut help_text = vec![
        // Common navigation keys
        Span::styled("[1-4/Tab]", Style::default().fg(Color::Yellow)),
        Span::raw(" Switch  "),
        Span::styled("[j/k]", Style::default().fg(Color::Yellow)),
        Span::raw(" Navigate  "),
        Span::styled("[/]", Style::default().fg(Color::Yellow)),
        Span::raw(" Search  "),
        Span::styled("[Space]", Style::default().fg(Color::Yellow)),
        Span::raw(" Select  "),
        Span::styled("[a]", Style::default().fg(Color::Yellow)),
        Span::raw(" All  "),
    ];

    // Show selection count if items are selected
    if app.has_selection() {
        help_text.push(Span::styled(
            format!(" ({} selected) ", app.selected_items.len()),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Tab-specific operations
    match app.current_tab {
        AppTab::Containers => {
            help_text.extend(vec![
                Span::styled("[s]", Style::default().fg(Color::Yellow)),
                Span::raw(" Start/Stop  "),
                Span::styled("[S]", Style::default().fg(Color::Yellow)),
                Span::raw(" Stats  "),
                Span::styled("[R]", Style::default().fg(Color::Yellow)),
                Span::raw(" Restart  "),
                Span::styled("[d]", Style::default().fg(Color::Yellow)),
                Span::raw(" Delete  "),
                Span::styled("[l]", Style::default().fg(Color::Yellow)),
                Span::raw(" Logs  "),
                Span::styled("[i]", Style::default().fg(Color::Yellow)),
                Span::raw(" Inspect  "),
            ]);
        }
        AppTab::Images => {
            help_text.extend(vec![
                Span::styled("[p]", Style::default().fg(Color::Yellow)),
                Span::raw(" Pull  "),
                Span::styled("[d]", Style::default().fg(Color::Yellow)),
                Span::raw(" Delete  "),
                Span::styled("[i]", Style::default().fg(Color::Yellow)),
                Span::raw(" Inspect  "),
            ]);
        }
        AppTab::Volumes => {
            help_text.extend(vec![
                Span::styled("[d]", Style::default().fg(Color::Yellow)),
                Span::raw(" Delete  "),
                Span::styled("[i]", Style::default().fg(Color::Yellow)),
                Span::raw(" Inspect  "),
            ]);
        }
        AppTab::Networks => {
            help_text.extend(vec![
                Span::styled("[d]", Style::default().fg(Color::Yellow)),
                Span::raw(" Delete  "),
                Span::styled("[i]", Style::default().fg(Color::Yellow)),
                Span::raw(" Inspect  "),
            ]);
        }
    }

    // Common actions
    help_text.extend(vec![
        Span::styled("[?]", Style::default().fg(Color::Yellow)),
        Span::raw(" Help  "),
        Span::styled("[q]", Style::default().fg(Color::Yellow)),
        Span::raw(" Quit"),
    ]);

    let help = Paragraph::new(Line::from(help_text))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(help, area);
}

fn draw_status_popup(frame: &mut Frame, kind: StatusKind, message: &str) {
    let area = centered_rect(60, 20, frame.size());

    frame.render_widget(Clear, area);

    let (title, color) = match kind {
        StatusKind::Success => (" Success ", Color::Green),
        StatusKind::Info => (" Info ", Color::Blue),
        StatusKind::Error => (" Error ", Color::Red),
    };

    let status_widget = Paragraph::new(message)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color)),
        )
        .style(Style::default().fg(color))
        .alignment(Alignment::Center);

    frame.render_widget(status_widget, area);
}

fn draw_help_popup(frame: &mut Frame) {
    let area = centered_rect(70, 75, frame.size());

    frame.render_widget(Clear, area);

    let help_text = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Global Commands",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  1-4            - Jump to specific tab (Containers/Images/Volumes/Networks)"),
        Line::from("  Tab/Shift+Tab  - Cycle through tabs"),
        Line::from("  q, Ctrl+c      - Quit application"),
        Line::from("  r, Ctrl+r      - Refresh current view"),
        Line::from("  ?              - Show this help"),
        Line::from("  /              - Search/Filter (Coming soon)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  j, ↓           - Move down"),
        Line::from("  k, ↑           - Move up"),
        Line::from("  PageUp/Down    - Page navigation"),
        Line::from("  Home/End       - Jump to start/end"),
        Line::from("  Space          - Toggle selection (for batch operations)"),
        Line::from("  a              - Select all / Deselect all (toggle)"),
        Line::from("  Esc            - Clear selection / Close dialogs"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Container Operations (Containers tab only)",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  s              - Start/Stop container"),
        Line::from("  S (Shift+s)    - View container stats (CPU/Memory)"),
        Line::from("  R              - Restart container"),
        Line::from("  d, Delete      - Remove container"),
        Line::from("  l              - View logs"),
        Line::from("  i              - Inspect (JSON)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Image Operations",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  p              - Pull image"),
        Line::from("  d, Delete      - Remove image"),
        Line::from("  i              - Inspect (JSON)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Volume/Network Operations",
            Style::default().add_modifier(Modifier::BOLD),
        )]),
        Line::from("  d, Delete      - Remove selected item"),
        Line::from("  i              - Inspect (JSON)"),
        Line::from(""),
        Line::from("Press any key to close this help"),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Left);

    frame.render_widget(help, area);
}

fn draw_delete_confirmation(frame: &mut Frame, tab: AppTab, count: usize) {
    let area = centered_rect(50, 20, frame.size());

    frame.render_widget(Clear, area);

    let item_type = match tab {
        AppTab::Containers => "container",
        AppTab::Images => "image",
        AppTab::Volumes => "volume",
        AppTab::Networks => "network",
    };

    let item_type_plural = match tab {
        AppTab::Containers => "containers",
        AppTab::Images => "images",
        AppTab::Volumes => "volumes",
        AppTab::Networks => "networks",
    };

    let message = if count > 1 {
        format!(
            "Are you sure you want to delete {} {}?",
            count, item_type_plural
        )
    } else {
        format!("Are you sure you want to delete this {}?", item_type)
    };

    let text = vec![
        Line::from(""),
        Line::from(message),
        Line::from(""),
        Line::from(vec![
            Span::styled("[y]", Style::default().fg(Color::Green)),
            Span::raw(" Yes  "),
            Span::styled("[n]", Style::default().fg(Color::Red)),
            Span::raw(" No"),
        ]),
    ];

    let confirmation = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Confirm Delete ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(confirmation, area);
}
