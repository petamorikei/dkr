use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Tick,
    Resize(u16, u16),
}

pub struct EventHandler {
    receiver: mpsc::Receiver<Event>,
    _handler_thread: thread::JoinHandle<()>,
}

impl EventHandler {
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

    pub fn next(&self) -> Result<Event> {
        Ok(self.receiver.recv()?)
    }
}

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
        (KeyCode::Char('R'), KeyModifiers::SHIFT) => Some(Action::Restart),
        (KeyCode::Char('d'), KeyModifiers::NONE) | (KeyCode::Delete, _) => Some(Action::Delete),
        (KeyCode::Char('l'), KeyModifiers::NONE) => Some(Action::ViewLogs),
        (KeyCode::Char('i'), KeyModifiers::NONE) => Some(Action::Inspect),
        
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

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Quit,
    NextTab,
    PreviousTab,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Select,
    MultiSelect,
    SelectAll,
    StartStop,
    Restart,
    Delete,
    ViewLogs,
    Inspect,
    SwitchToTab(usize),
    Refresh,
    Help,
    Search,
    Escape,
}