use super::utils::centered_rect;
use crate::docker::ContainerStats;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph},
    Frame,
};

#[derive(Clone)]
pub struct StatsViewer {
    pub container_name: String,
    pub stats: ContainerStats,
}

impl StatsViewer {
    pub fn new(container_name: String, stats: ContainerStats) -> Self {
        Self {
            container_name,
            stats,
        }
    }

    pub fn update_stats(&mut self, stats: ContainerStats) {
        self.stats = stats;
    }
}

pub fn draw_stats_popup(frame: &mut Frame, viewer: &StatsViewer) {
    let area = centered_rect(60, 40, frame.size());

    // Clear the area
    frame.render_widget(Clear, area);

    // Create the main block
    let block = Block::default()
        .title(format!(" Container Stats: {} ", viewer.container_name))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    frame.render_widget(block, area);

    // Create inner layout
    let inner_area = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(4),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // CPU section
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Memory section
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Memory details
            Constraint::Min(0),    // Remaining space
        ])
        .split(inner_area);

    // CPU usage
    let cpu_label = Paragraph::new(Line::from(vec![
        Span::raw("CPU Usage: "),
        Span::styled(
            format!("{:.2}%", viewer.stats.cpu_percent),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    frame.render_widget(cpu_label, chunks[0]);

    let cpu_gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(Style::default().fg(Color::Yellow).bg(Color::Black))
        .percent((viewer.stats.cpu_percent.min(100.0)) as u16);
    frame.render_widget(
        cpu_gauge,
        Rect {
            x: chunks[0].x,
            y: chunks[0].y + 1,
            width: chunks[0].width,
            height: 1,
        },
    );

    // Memory usage
    let memory_label = Paragraph::new(Line::from(vec![
        Span::raw("Memory Usage: "),
        Span::styled(
            format!("{:.2}%", viewer.stats.memory_percent),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    frame.render_widget(memory_label, chunks[2]);

    let memory_gauge = Gauge::default()
        .block(Block::default())
        .gauge_style(Style::default().fg(Color::Green).bg(Color::Black))
        .percent((viewer.stats.memory_percent.min(100.0)) as u16);
    frame.render_widget(
        memory_gauge,
        Rect {
            x: chunks[2].x,
            y: chunks[2].y + 1,
            width: chunks[2].width,
            height: 1,
        },
    );

    // Memory details
    let memory_details = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("Usage: "),
            Span::styled(
                format_bytes(viewer.stats.memory_usage),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("Limit: "),
            Span::styled(
                format_bytes(viewer.stats.memory_limit),
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ]);
    frame.render_widget(memory_details, chunks[4]);

    // Help text at the bottom
    let help_area = Rect {
        x: area.x + 2,
        y: area.y + area.height - 2,
        width: area.width.saturating_sub(4),
        height: 1,
    };
    let help = Paragraph::new("Press 'q' or 'Esc' to close")
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(help, help_area);
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}
