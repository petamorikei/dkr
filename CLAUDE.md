# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`dkr` is a Docker TUI (Terminal User Interface) application built with Rust edition 2024. It provides an interactive terminal interface for managing Docker containers, images, volumes, and networks using Ratatui for the UI and bollard for Docker API interaction.

## Commands

### Build
```bash
cargo build
```

### Run
```bash
cargo run
```

### Test
```bash
cargo test
```

### Run a single test
```bash
cargo test test_name
```

### Check for compilation errors
```bash
cargo check
```

### Format code
```bash
cargo fmt
```

### Lint
```bash
cargo clippy
```

## Project Structure

- `src/main.rs` - Entry point containing the main function
- `src/app.rs` - Main application state and logic
- `src/ui/` - UI components and rendering
- `src/docker/` - Docker API integration using bollard
- `src/config.rs` - Configuration management
- `Cargo.toml` - Rust package manifest defining project metadata and dependencies

## Key Dependencies

- **ratatui** - Terminal UI framework
- **bollard** - Docker API client
- **tokio** - Async runtime
- **crossterm** - Terminal manipulation
- **serde/toml** - Configuration file handling

## Implementation Phases

### Phase 1 (MVP)
- Container list view (status, name, image)
- Container operations (start/stop/restart/remove)
- Basic keyboard navigation
- Tab views for images/volumes/networks
- Simple error display

### Phase 2
- Log viewing (read-only, no follow mode initially)
- Configuration file support
- Filter/search (simple string matching)
- Detail view (inspect)

### Phase 3
- Log follow mode
- Statistics display (CPU/Memory)
- Image pull/remove operations

### Future Considerations
- Docker exec (shell access) - complex PTY handling
- Virtual scrolling - for large lists
- Docker Compose support
- Clipboard integration

## UI Layout

Hybrid layout with tabs and panes:
- Main view with tabs (Containers/Images/Volumes/Networks)
- Optional right pane for logs/details
- Status bar showing connection state and help

## Keyboard Shortcuts

Global:
- `1-4` or `Tab/Shift+Tab` - Switch views
- `q` or `Ctrl+c` - Quit
- `/` - Search/filter
- `?` - Help
- `r` or `Ctrl+r` - Refresh

List Navigation:
- `j/k` or `↑/↓` - Move up/down
- `PageUp/PageDown` - Page navigation
- `Home/End` - Jump to start/end
- `Enter` - Toggle details
- `Space` - Multi-select

Container Operations:
- `s` - Start/Stop toggle
- `R` - Restart
- `l` - View logs
- `e` - Exec (future)
- `d` or `Delete` - Remove
- `i` - Inspect

## Configuration

Config file location: `~/.config/dkr/config.toml`

## Error Handling

- Clear error messages in English
- Auto-reconnect for Docker daemon connection issues
- Non-blocking error notifications
- Error logging to `~/.local/share/dkr/errors.log`

## Development Notes

- All code, comments, and messages should be in English
- Follow existing Rust conventions and idioms
- Use async/await with tokio for Docker API calls
- Keep initial implementation simple, avoid over-engineering