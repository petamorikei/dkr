use crate::config::Config;
use crate::docker::DockerClient;
use crate::ui::{LogViewer, InspectViewer};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppTab {
    Containers,
    Images,
    Volumes,
    Networks,
}

impl AppTab {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppTab::Containers => "Containers",
            AppTab::Images => "Images",
            AppTab::Volumes => "Volumes",
            AppTab::Networks => "Networks",
        }
    }
    
    pub fn with_number(&self) -> String {
        match self {
            AppTab::Containers => "[1] Containers".to_string(),
            AppTab::Images => "[2] Images".to_string(),
            AppTab::Volumes => "[3] Volumes".to_string(),
            AppTab::Networks => "[4] Networks".to_string(),
        }
    }

    pub fn all() -> Vec<AppTab> {
        vec![
            AppTab::Containers,
            AppTab::Images,
            AppTab::Volumes,
            AppTab::Networks,
        ]
    }
}

pub struct App {
    pub config: Config,
    pub docker: Arc<Mutex<DockerClient>>,
    pub current_tab: AppTab,
    pub selected_index: usize,
    pub selected_items: std::collections::HashSet<String>,
    pub should_quit: bool,
    pub show_help: bool,
    pub show_logs: bool,
    pub show_inspect: bool,
    pub show_confirm_delete: bool,
    pub pending_delete_ids: Vec<String>,
    pub log_viewer: Option<LogViewer>,
    pub inspect_viewer: Option<InspectViewer>,
    pub error_message: Option<String>,
}

impl App {
    pub async fn new() -> Result<Self> {
        let config = Config::load()?;
        let docker = DockerClient::new().await?;

        Ok(Self {
            config,
            docker: Arc::new(Mutex::new(docker)),
            current_tab: AppTab::Containers,
            selected_index: 0,
            selected_items: std::collections::HashSet::new(),
            should_quit: false,
            show_help: false,
            show_logs: false,
            show_inspect: false,
            show_confirm_delete: false,
            pending_delete_ids: Vec::new(),
            log_viewer: None,
            inspect_viewer: None,
            error_message: None,
        })
    }

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
    
    pub fn toggle_selection(&mut self, id: String) {
        if self.selected_items.contains(&id) {
            self.selected_items.remove(&id);
        } else {
            self.selected_items.insert(id);
        }
    }
    
    pub fn clear_selection(&mut self) {
        self.selected_items.clear();
    }
    
    pub fn has_selection(&self) -> bool {
        !self.selected_items.is_empty()
    }

    pub fn select_next(&mut self, max_items: usize) {
        if max_items > 0 {
            self.selected_index = (self.selected_index + 1).min(max_items - 1);
        }
    }

    pub fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn select_first(&mut self) {
        self.selected_index = 0;
    }

    pub fn select_last(&mut self, max_items: usize) {
        if max_items > 0 {
            self.selected_index = max_items - 1;
        }
    }

    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

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