use super::utils::centered_rect;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

const MAX_LOG_LINES: usize = 10000; // Maximum number of log lines to keep in memory

#[derive(Clone)]
pub struct LogViewer {
    pub logs: Vec<String>,
    pub container_name: String,
    pub scroll_position: usize,
    pub is_following: bool,
    pub search_term: Option<String>,
    list_state: ListState,
}

impl LogViewer {
    pub fn new(container_name: String) -> Self {
        Self {
            logs: Vec::new(),
            container_name,
            scroll_position: 0,
            is_following: true,
            search_term: None,
            list_state: ListState::default(),
        }
    }

    pub fn set_logs(&mut self, logs: Vec<String>) {
        // Limit the number of logs to prevent memory issues
        if logs.len() > MAX_LOG_LINES {
            let skip_count = logs.len() - MAX_LOG_LINES;
            self.logs = logs.into_iter().skip(skip_count).collect();
        } else {
            self.logs = logs;
        }

        if self.is_following && !self.logs.is_empty() {
            self.scroll_to_bottom();
        }
    }

    pub fn append_logs(&mut self, new_logs: Vec<String>) {
        self.logs.extend(new_logs);

        // Trim logs if they exceed the maximum limit
        if self.logs.len() > MAX_LOG_LINES {
            let excess = self.logs.len() - MAX_LOG_LINES;
            self.logs.drain(0..excess);

            // Adjust scroll position if logs were removed
            if self.scroll_position >= excess {
                self.scroll_position -= excess;
            } else {
                self.scroll_position = 0;
            }
        }

        if self.is_following {
            self.scroll_to_bottom();
        }
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_position = self.scroll_position.saturating_sub(amount);
        self.is_following = false;
        self.list_state.select(Some(self.scroll_position));
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let max_scroll = self.logs.len().saturating_sub(1);
        self.scroll_position = (self.scroll_position + amount).min(max_scroll);

        if self.scroll_position >= max_scroll {
            self.is_following = true;
        }
        self.list_state.select(Some(self.scroll_position));
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_position = 0;
        self.is_following = false;
        self.list_state.select(Some(0));
    }

    pub fn scroll_to_bottom(&mut self) {
        if !self.logs.is_empty() {
            self.scroll_position = self.logs.len() - 1;
            self.is_following = true;
            self.list_state.select(Some(self.scroll_position));
        }
    }

    pub fn toggle_follow(&mut self) {
        self.is_following = !self.is_following;
        if self.is_following {
            self.scroll_to_bottom();
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.scroll_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.scroll_down(page_size);
    }

    pub fn search(&mut self, term: String) {
        self.search_term = Some(term);
        // Find next occurrence from current position
        if let Some(ref term) = self.search_term {
            for (i, log) in self.logs.iter().enumerate().skip(self.scroll_position + 1) {
                if log.to_lowercase().contains(&term.to_lowercase()) {
                    self.scroll_position = i;
                    self.is_following = false;
                    self.list_state.select(Some(i));
                    break;
                }
            }
        }
    }

    pub fn clear_search(&mut self) {
        self.search_term = None;
    }

    pub fn find_next(&mut self) {
        if let Some(ref term) = self.search_term {
            let start = (self.scroll_position + 1).min(self.logs.len());
            for (i, log) in self.logs.iter().enumerate().skip(start) {
                if log.to_lowercase().contains(&term.to_lowercase()) {
                    self.scroll_position = i;
                    self.is_following = false;
                    self.list_state.select(Some(i));
                    break;
                }
            }
        }
    }

    pub fn find_previous(&mut self) {
        if let Some(ref term) = self.search_term {
            for (i, log) in self.logs[..self.scroll_position].iter().enumerate().rev() {
                if log.to_lowercase().contains(&term.to_lowercase()) {
                    self.scroll_position = i;
                    self.is_following = false;
                    self.list_state.select(Some(i));
                    break;
                }
            }
        }
    }
}

pub fn draw_log_viewer(frame: &mut Frame, viewer: &mut LogViewer, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Logs
            Constraint::Length(2), // Footer
        ])
        .split(area);

    // Header
    let header_text = format!(
        " Logs: {} {} ",
        viewer.container_name,
        if viewer.is_following {
            "[FOLLOWING]"
        } else {
            "[PAUSED]"
        }
    );

    let header = Paragraph::new(header_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(if viewer.is_following {
            Color::Green
        } else {
            Color::Yellow
        }));

    frame.render_widget(header, chunks[0]);

    // Logs
    let log_items: Vec<ListItem> = viewer
        .logs
        .iter()
        .map(|log| {
            let content = if let Some(ref term) = viewer.search_term {
                if log.to_lowercase().contains(&term.to_lowercase()) {
                    // Highlight search term
                    let parts: Vec<&str> = log.split(term.as_str()).collect();
                    let mut spans = Vec::new();
                    for (j, part) in parts.iter().enumerate() {
                        spans.push(Span::raw(*part));
                        if j < parts.len() - 1 {
                            spans.push(Span::styled(
                                term.clone(),
                                Style::default().bg(Color::Yellow).fg(Color::Black),
                            ));
                        }
                    }
                    Line::from(spans)
                } else {
                    Line::from(log.as_str())
                }
            } else {
                Line::from(log.as_str())
            };

            ListItem::new(content)
        })
        .collect();

    let logs_list = List::new(log_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Log Output "),
        )
        .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
        .highlight_symbol("> ");

    frame.render_stateful_widget(logs_list, chunks[1], &mut viewer.list_state);

    // Footer
    let footer_text = if viewer.search_term.is_some() {
        vec![
            Span::styled("[n]", Style::default().fg(Color::Yellow)),
            Span::raw(" Next  "),
            Span::styled("[N]", Style::default().fg(Color::Yellow)),
            Span::raw(" Previous  "),
            Span::styled("[ESC]", Style::default().fg(Color::Yellow)),
            Span::raw(" Clear Search  "),
            Span::styled("[q]", Style::default().fg(Color::Yellow)),
            Span::raw(" Close"),
        ]
    } else {
        vec![
            Span::styled("[f]", Style::default().fg(Color::Yellow)),
            Span::raw(" Follow  "),
            Span::styled("[/]", Style::default().fg(Color::Yellow)),
            Span::raw(" Search  "),
            Span::styled("[j/k]", Style::default().fg(Color::Yellow)),
            Span::raw(" Scroll  "),
            Span::styled("[PgUp/PgDn]", Style::default().fg(Color::Yellow)),
            Span::raw(" Page  "),
            Span::styled("[q]", Style::default().fg(Color::Yellow)),
            Span::raw(" Close"),
        ]
    };

    let footer = Paragraph::new(Line::from(footer_text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(footer, chunks[2]);
}

pub fn draw_log_popup(frame: &mut Frame, viewer: &mut LogViewer) {
    let area = centered_rect(90, 80, frame.size());
    frame.render_widget(Clear, area);
    draw_log_viewer(frame, viewer, area);
}
