//! Event handling and keyboard input processing

use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Terminal events
#[derive(Debug, Clone)]
pub enum Event {
    /// Keyboard input event
    Key(KeyEvent),
    /// Periodic tick event for UI updates
    Tick,
    /// Terminal resize event
    Resize(u16, u16),
}

/// Event handler that polls for terminal events
///
/// Runs in a background thread and sends events through a channel.
pub struct EventHandler {
    receiver: mpsc::Receiver<Event>,
    _handler_thread: thread::JoinHandle<()>,
}

impl EventHandler {
    /// Creates a new event handler
    ///
    /// # Arguments
    /// * `tick_rate` - Duration between tick events
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::channel();
        let handler_thread = {
            let sender = sender.clone();
            thread::spawn(move || {
                let mut last_tick = std::time::Instant::now();
                loop {
                    let timeout = tick_rate
                        .checked_sub(last_tick.elapsed())
                        .unwrap_or_else(|| Duration::from_secs(0));

                    if event::poll(timeout).unwrap_or(false) {
                        match event::read() {
                            Ok(CrosstermEvent::Key(key)) => {
                                if sender.send(Event::Key(key)).is_err() {
                                    break;
                                }
                            }
                            Ok(CrosstermEvent::Resize(width, height)) => {
                                if sender.send(Event::Resize(width, height)).is_err() {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }

                    if last_tick.elapsed() >= tick_rate {
                        if sender.send(Event::Tick).is_err() {
                            break;
                        }
                        last_tick = std::time::Instant::now();
                    }
                }
            })
        };

        Self {
            receiver,
            _handler_thread: handler_thread,
        }
    }

    /// Blocks until the next event is available
    ///
    /// # Errors
    ///
    /// Returns an error if the event handler thread has panicked.
    pub fn next(&self) -> Result<Event> {
        Ok(self.receiver.recv()?)
    }
}

/// Converts a keyboard event into an application action
///
/// Returns `None` if the key combination is not bound to any action.
pub fn handle_key_event(key: KeyEvent) -> Option<Action> {
    match (key.code, key.modifiers) {
        // Quit
        (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            Some(Action::Quit)
        }
        
        // Navigation
        (KeyCode::Tab, KeyModifiers::NONE) => Some(Action::NextTab),
        (KeyCode::BackTab, KeyModifiers::SHIFT) => Some(Action::PreviousTab),
        
        // List navigation
        (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => Some(Action::Up),
        (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => Some(Action::Down),
        (KeyCode::PageUp, _) => Some(Action::PageUp),
        (KeyCode::PageDown, _) => Some(Action::PageDown),
        (KeyCode::Home, _) => Some(Action::Home),
        (KeyCode::End, _) => Some(Action::End),
        
        // Selection
        (KeyCode::Enter, KeyModifiers::NONE) => Some(Action::Select),
        (KeyCode::Char(' '), KeyModifiers::NONE) => Some(Action::MultiSelect),
        
        // Container operations
        (KeyCode::Char('s'), KeyModifiers::NONE) => Some(Action::StartStop),
        (KeyCode::Char('S'), KeyModifiers::SHIFT) => Some(Action::ViewStats),
        (KeyCode::Char('R'), KeyModifiers::SHIFT) => Some(Action::Restart),
        (KeyCode::Char('d'), KeyModifiers::NONE) | (KeyCode::Delete, _) => Some(Action::Delete),
        (KeyCode::Char('l'), KeyModifiers::NONE) => Some(Action::ViewLogs),
        (KeyCode::Char('i'), KeyModifiers::NONE) => Some(Action::Inspect),
        (KeyCode::Char('p'), KeyModifiers::NONE) => Some(Action::PullImage),
        
        // View switching
        (KeyCode::Char('1'), KeyModifiers::NONE) => Some(Action::SwitchToTab(0)),
        (KeyCode::Char('2'), KeyModifiers::NONE) => Some(Action::SwitchToTab(1)),
        (KeyCode::Char('3'), KeyModifiers::NONE) => Some(Action::SwitchToTab(2)),
        (KeyCode::Char('4'), KeyModifiers::NONE) => Some(Action::SwitchToTab(3)),
        
        // Other
        (KeyCode::Char('r'), KeyModifiers::NONE) | (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
            Some(Action::Refresh)
        }
        (KeyCode::Char('?'), KeyModifiers::NONE) => Some(Action::Help),
        (KeyCode::Char('/'), KeyModifiers::NONE) => Some(Action::Search),
        (KeyCode::Char('a'), KeyModifiers::CONTROL) => Some(Action::SelectAll),
        (KeyCode::Esc, KeyModifiers::NONE) => Some(Action::Escape),
        
        _ => None,
    }
}

/// User actions that can be triggered by keyboard input
#[derive(Debug, Clone, Copy)]
pub enum Action {
    /// Quit the application
    Quit,
    /// Switch to next tab
    NextTab,
    /// Switch to previous tab
    PreviousTab,
    /// Move selection up
    Up,
    /// Move selection down
    Down,
    /// Page up
    PageUp,
    /// Page down
    PageDown,
    /// Jump to first item
    Home,
    /// Jump to last item
    End,
    /// Select current item
    Select,
    /// Toggle multi-select for current item
    MultiSelect,
    /// Select all items
    SelectAll,
    /// Start or stop container (toggles state)
    StartStop,
    /// Restart container
    Restart,
    /// Delete selected item(s)
    Delete,
    /// View container logs
    ViewLogs,
    /// View container statistics
    ViewStats,
    /// Inspect resource (JSON view)
    Inspect,
    /// Pull Docker image
    PullImage,
    /// Switch to specific tab by index
    SwitchToTab(usize),
    /// Refresh current view
    Refresh,
    /// Show help
    Help,
    /// Open search/filter (not yet implemented)
    Search,
    /// Escape/cancel current operation
    Escape,
}