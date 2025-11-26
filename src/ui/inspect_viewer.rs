use super::utils::centered_rect;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};
use serde_json;

#[derive(Clone)]
pub struct InspectViewer {
    pub title: String,
    pub content: String,
    pub scroll_position: u16,
    pub max_scroll: u16,
}

impl InspectViewer {
    pub fn new(title: String, data: serde_json::Value) -> Self {
        let content = serde_json::to_string_pretty(&data)
            .unwrap_or_else(|e| format!("Failed to format JSON: {}", e));
        
        let line_count = content.lines().count() as u16;
        
        Self {
            title,
            content,
            scroll_position: 0,
            max_scroll: line_count.saturating_sub(10), // Assuming ~10 lines visible
        }
    }

    pub fn scroll_up(&mut self, amount: u16) {
        self.scroll_position = self.scroll_position.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: u16) {
        self.scroll_position = (self.scroll_position + amount).min(self.max_scroll);
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_position = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_position = self.max_scroll;
    }

    pub fn page_up(&mut self, page_size: u16) {
        self.scroll_up(page_size);
    }

    pub fn page_down(&mut self, page_size: u16) {
        self.scroll_down(page_size);
    }
}

pub fn draw_inspect_viewer(frame: &mut Frame, viewer: &InspectViewer, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),     // Content
            Constraint::Length(2),  // Footer
        ])
        .split(area);

    // Content with JSON
    let lines: Vec<Line> = viewer.content
        .lines()
        .skip(viewer.scroll_position as usize)
        .take(chunks[0].height as usize - 2) // Account for borders
        .map(|line| {
            // Basic JSON syntax highlighting
            let trimmed = line.trim_start();
            if trimmed.starts_with('"') {
                // Check if this is a key-value pair (has '": ' pattern)
                if let Some(colon_pos) = line.find("\": ") {
                    // It's a key-value pair
                    let (key_part, rest) = line.split_at(colon_pos + 1);
                    Line::from(vec![
                        Span::styled(key_part, Style::default().fg(Color::Cyan)),
                        Span::styled(rest, Style::default().fg(Color::White)),
                    ])
                } else {
                    // String value (in array or standalone)
                    Line::styled(line, Style::default().fg(Color::Green))
                }
            } else if trimmed.starts_with('[') || trimmed.starts_with(']') ||
                      trimmed.starts_with('{') || trimmed.starts_with('}') {
                // Array or object brackets
                Line::styled(line, Style::default().fg(Color::Yellow))
            } else if trimmed == "true," || trimmed == "false," || trimmed == "null," ||
                      trimmed == "true" || trimmed == "false" || trimmed == "null" {
                // Boolean or null values (exact match)
                Line::styled(line, Style::default().fg(Color::Magenta))
            } else if trimmed.chars().next().is_some_and(|c| c.is_ascii_digit() || c == '-') {
                // Number values
                Line::styled(line, Style::default().fg(Color::Magenta))
            } else {
                Line::from(line)
            }
        })
        .collect();

    let content = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", viewer.title))
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(content, chunks[0]);

    // Scrollbar
    let scrollbar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    
    let mut scrollbar_state = ScrollbarState::default()
        .content_length(viewer.content.lines().count())
        .position(viewer.scroll_position as usize);

    frame.render_stateful_widget(
        scrollbar,
        chunks[0].inner(&ratatui::layout::Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );

    // Footer
    let footer_text = vec![
        Span::styled("[j/k]", Style::default().fg(Color::Yellow)),
        Span::raw(" Scroll  "),
        Span::styled("[PgUp/PgDn]", Style::default().fg(Color::Yellow)),
        Span::raw(" Page  "),
        Span::styled("[Home/End]", Style::default().fg(Color::Yellow)),
        Span::raw(" Top/Bottom  "),
        Span::styled("[/]", Style::default().fg(Color::Yellow)),
        Span::raw(" Search  "),
        Span::styled("[q]", Style::default().fg(Color::Yellow)),
        Span::raw(" Close"),
    ];

    let footer = Paragraph::new(Line::from(footer_text))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(footer, chunks[1]);
}

pub fn draw_inspect_popup(frame: &mut Frame, viewer: &InspectViewer) {
    let area = centered_rect(80, 80, frame.size());
    frame.render_widget(Clear, area);
    draw_inspect_viewer(frame, viewer, area);
}