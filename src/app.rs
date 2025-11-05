//! Application state and core logic

use crate::config::Config;
use crate::docker::{BollardDockerClient, DockerClient};
use crate::ui::{InspectViewer, LogViewer, StatsViewer, Theme};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Represents the different tabs/views in the TUI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppTab {
    /// Container management view
    Containers,
    /// Image management view
    Images,
    /// Volume management view
    Volumes,
    /// Network management view
    Networks,
}

impl AppTab {
    /// Returns the string representation of the tab
    pub fn as_str(&self) -> &'static str {
        match self {
            AppTab::Containers => "Containers",
            AppTab::Images => "Images",
            AppTab::Volumes => "Volumes",
            AppTab::Networks => "Networks",
        }
    }

    /// Returns the tab name with its keyboard shortcut number
    pub fn with_number(&self) -> String {
        match self {
            AppTab::Containers => "[1] Containers".to_string(),
            AppTab::Images => "[2] Images".to_string(),
            AppTab::Volumes => "[3] Volumes".to_string(),
            AppTab::Networks => "[4] Networks".to_string(),
        }
    }

    /// Returns all available tabs in order
    pub fn all() -> Vec<AppTab> {
        vec![
            AppTab::Containers,
            AppTab::Images,
            AppTab::Volumes,
            AppTab::Networks,
        ]
    }
}

/// Main application state
///
/// Contains all the state needed to run the TUI application, including
/// configuration, Docker client, UI state, and active viewers.
pub struct App {
    /// Application configuration
    pub config: Config,
    /// UI theme configuration
    pub theme: Theme,
    /// Docker client for API operations
    pub docker: Arc<Mutex<dyn DockerClient>>,
    /// Currently active tab
    pub current_tab: AppTab,
    /// Index of the currently selected item in the list
    pub selected_index: usize,
    /// Set of selected item IDs (for multi-select operations)
    pub selected_items: std::collections::HashSet<String>,
    /// Whether the application should quit
    pub should_quit: bool,
    /// Whether the help popup is shown
    pub show_help: bool,
    /// Whether the log viewer is shown
    pub show_logs: bool,
    /// Whether the inspect viewer is shown
    pub show_inspect: bool,
    /// Whether the stats viewer is shown
    pub show_stats: bool,
    /// Whether the delete confirmation dialog is shown
    pub show_confirm_delete: bool,
    /// IDs pending deletion (shown in confirmation dialog)
    pub pending_delete_ids: Vec<String>,
    /// Active log viewer instance
    pub log_viewer: Option<LogViewer>,
    /// Active inspect viewer instance
    pub inspect_viewer: Option<InspectViewer>,
    /// Active stats viewer instance
    pub stats_viewer: Option<StatsViewer>,
    /// Current error message to display
    pub error_message: Option<String>,
}

impl App {
    /// Creates a new App instance
    ///
    /// Loads configuration from file and connects to Docker daemon.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading fails or Docker connection fails.
    pub async fn new() -> Result<Self> {
        let config = Config::load()?;
        let theme = Theme::from_config(&config.ui.theme);
        let docker = BollardDockerClient::new().await?;

        Ok(Self {
            config,
            theme,
            docker: Arc::new(Mutex::new(docker)),
            current_tab: AppTab::Containers,
            selected_index: 0,
            selected_items: std::collections::HashSet::new(),
            should_quit: false,
            show_help: false,
            show_logs: false,
            show_inspect: false,
            show_stats: false,
            show_confirm_delete: false,
            pending_delete_ids: Vec::new(),
            log_viewer: None,
            inspect_viewer: None,
            stats_viewer: None,
            error_message: None,
        })
    }

    /// Switches to the next tab
    ///
    /// Wraps around to the first tab after the last. Resets selection state.
    pub fn next_tab(&mut self) {
        let tabs = AppTab::all();
        let current_index = tabs
            .iter()
            .position(|&t| t == self.current_tab)
            .unwrap_or(0);
        let next_index = (current_index + 1) % tabs.len();
        self.current_tab = tabs[next_index];
        self.selected_index = 0;
        self.selected_items.clear(); // Clear selection when switching tabs
    }

    /// Switches to the previous tab
    ///
    /// Wraps around to the last tab before the first. Resets selection state.
    pub fn previous_tab(&mut self) {
        let tabs = AppTab::all();
        let current_index = tabs
            .iter()
            .position(|&t| t == self.current_tab)
            .unwrap_or(0);
        let prev_index = if current_index == 0 {
            tabs.len() - 1
        } else {
            current_index - 1
        };
        self.current_tab = tabs[prev_index];
        self.selected_index = 0;
        self.selected_items.clear(); // Clear selection when switching tabs
    }

    /// Toggles selection of an item by ID
    ///
    /// If the item is already selected, it is deselected. Otherwise, it is selected.
    pub fn toggle_selection(&mut self, id: String) {
        if self.selected_items.contains(&id) {
            self.selected_items.remove(&id);
        } else {
            self.selected_items.insert(id);
        }
    }

    /// Clears all selected items
    pub fn clear_selection(&mut self) {
        self.selected_items.clear();
    }

    /// Returns true if any items are selected
    pub fn has_selection(&self) -> bool {
        !self.selected_items.is_empty()
    }

    /// Moves the selection cursor to the next item
    ///
    /// # Arguments
    /// * `max_items` - Total number of items in the list
    pub fn select_next(&mut self, max_items: usize) {
        if max_items > 0 {
            self.selected_index = (self.selected_index + 1).min(max_items - 1);
        }
    }

    /// Moves the selection cursor to the previous item
    pub fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Moves the selection cursor to the first item
    pub fn select_first(&mut self) {
        self.selected_index = 0;
    }

    /// Moves the selection cursor to the last item
    ///
    /// # Arguments
    /// * `max_items` - Total number of items in the list
    pub fn select_last(&mut self, max_items: usize) {
        if max_items > 0 {
            self.selected_index = max_items - 1;
        }
    }

    /// Sets an error message to be displayed
    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
    }

    /// Clears the current error message
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Signals that the application should quit
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_navigation() {
        let tabs = AppTab::all();
        assert_eq!(tabs.len(), 4);
        assert_eq!(tabs[0], AppTab::Containers);
        assert_eq!(tabs[3], AppTab::Networks);
    }

    #[test]
    fn test_tab_display() {
        assert_eq!(AppTab::Containers.as_str(), "Containers");
        assert_eq!(AppTab::Containers.with_number(), "[1] Containers");
        assert_eq!(AppTab::Images.with_number(), "[2] Images");
    }
}