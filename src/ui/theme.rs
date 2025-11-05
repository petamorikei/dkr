use ratatui::style::{Color, Modifier, Style};
use crate::docker::ContainerState;

/// Theme using only ANSI colors for maximum compatibility
pub struct Theme {
    pub selected_style: Style,
    pub header_style: Style,
    pub running_style: Style,
    pub stopped_style: Style,
    pub paused_style: Style,
    pub error_style: Style,
    pub warning_style: Style,
    pub info_style: Style,
    pub help_key_style: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self::ansi()
    }
}

impl Theme {
    /// Create theme from config string
    pub fn from_config(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "ansi" => Self::ansi(),
            "reversed" => Self::reversed(),
            "minimal" => Self::minimal(),
            _ => Self::default(),
        }
    }

    /// Get color for container state
    pub fn get_state_color(&self, state: &ContainerState) -> Color {
        match state {
            ContainerState::Running => {
                self.running_style.fg.unwrap_or(Color::Green)
            }
            ContainerState::Paused => {
                self.paused_style.fg.unwrap_or(Color::Yellow)
            }
            ContainerState::Exited | ContainerState::Dead => {
                self.stopped_style.fg.unwrap_or(Color::Red)
            }
            _ => Color::Gray,
        }
    }

    /// Standard ANSI color theme
    pub fn ansi() -> Self {
        Self {
            // Blue background with white text for good contrast
            selected_style: Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            
            header_style: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            
            running_style: Style::default()
                .fg(Color::Green),
            
            stopped_style: Style::default()
                .fg(Color::Red),
            
            paused_style: Style::default()
                .fg(Color::Yellow),
            
            error_style: Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
            
            warning_style: Style::default()
                .fg(Color::Yellow),
            
            info_style: Style::default()
                .fg(Color::Cyan),
            
            help_key_style: Style::default()
                .fg(Color::Yellow),
        }
    }
    
    /// Alternative theme using reversed colors
    pub fn reversed() -> Self {
        Self {
            // Reversed for selection
            selected_style: Style::default()
                .add_modifier(Modifier::REVERSED | Modifier::BOLD),
            
            header_style: Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            
            running_style: Style::default()
                .fg(Color::Green),
            
            stopped_style: Style::default()
                .fg(Color::Red),
            
            paused_style: Style::default()
                .fg(Color::Yellow),
            
            error_style: Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
            
            warning_style: Style::default()
                .fg(Color::Yellow),
            
            info_style: Style::default()
                .fg(Color::Cyan),
            
            help_key_style: Style::default()
                .fg(Color::Yellow),
        }
    }
    
    /// Minimal theme with underline for selection
    pub fn minimal() -> Self {
        Self {
            // Just underline and bold for selection
            selected_style: Style::default()
                .add_modifier(Modifier::UNDERLINED | Modifier::BOLD),
            
            header_style: Style::default()
                .add_modifier(Modifier::BOLD),
            
            running_style: Style::default()
                .fg(Color::Green),
            
            stopped_style: Style::default()
                .fg(Color::Red),
            
            paused_style: Style::default()
                .fg(Color::Yellow),
            
            error_style: Style::default()
                .fg(Color::Red),
            
            warning_style: Style::default()
                .fg(Color::Yellow),
            
            info_style: Style::default()
                .fg(Color::Cyan),
            
            help_key_style: Style::default()
                .add_modifier(Modifier::BOLD),
        }
    }
}