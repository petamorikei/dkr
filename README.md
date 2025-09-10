# dkr - Docker TUI

A Terminal User Interface (TUI) application for managing Docker containers, images, volumes, and networks.

## Features

- **Container Management**: View, start, stop, restart, and remove containers
- **Image Management**: List and manage Docker images
- **Volume Management**: View Docker volumes
- **Network Management**: List Docker networks
- **Real-time Updates**: Auto-refresh to keep information current
- **Keyboard Navigation**: Efficient keyboard shortcuts for all operations
- **Hybrid Layout**: Tab-based navigation with optional detail panes

## Installation

### Prerequisites

- Rust 1.70+ (for building from source)
- Docker daemon running
- User must have permissions to access Docker socket

### Build from Source

```bash
git clone https://github.com/yourusername/dkr.git
cd dkr
cargo build --release
```

The binary will be available at `target/release/dkr`

## Usage

Simply run the application:

```bash
dkr
```

### Keyboard Shortcuts

#### Global Commands
- `Tab` / `Shift+Tab` - Switch between tabs
- `1-4` - Jump to specific tab (Containers/Images/Volumes/Networks)
- `q`, `Ctrl+c` - Quit application
- `r`, `Ctrl+r` - Refresh current view
- `?` - Show help
- `/` - Search/Filter (coming soon)

#### Navigation
- `j`, `↓` - Move down
- `k`, `↑` - Move up
- `PageUp`/`PageDown` - Page navigation
- `Home`/`End` - Jump to start/end
- `Enter` - View details
- `Space` - Multi-select (coming soon)

#### Container Operations
- `s` - Start/Stop container
- `R` - Restart container
- `d`, `Delete` - Remove container
- `l` - View logs (coming soon)
- `i` - Inspect (JSON) (coming soon)

## Configuration

Configuration file is located at `~/.config/dkr/config.toml`

```toml
[general]
refresh_interval = 5         # Auto-refresh interval in seconds
default_view = "containers"  # Default tab on startup
confirm_delete = true        # Confirm before deleting
auto_refresh = true          # Enable auto-refresh

[ui]
theme = "dark"              # Theme: dark/light
show_header = true          # Show header
show_footer = true          # Show footer with help
show_logs_pane = false      # Show logs pane on startup
logs_buffer_size = 1000     # Log buffer size in lines

[docker]
socket = "unix:///var/run/docker.sock"  # Docker socket path
timeout = 30                # API timeout in seconds
```

## Troubleshooting

### Permission Denied

If you get a permission error accessing the Docker socket:

```bash
# Add your user to the docker group
sudo usermod -aG docker $USER

# Log out and back in for changes to take effect
```

### Docker Not Running

Make sure the Docker daemon is running:

```bash
# On Linux with systemd
sudo systemctl start docker

# On macOS
open -a Docker
```

## Development

### Project Structure

```
dkr/
├── src/
│   ├── main.rs           # Application entry point
│   ├── app.rs            # Application state management
│   ├── config.rs         # Configuration handling
│   ├── docker/           # Docker API integration
│   │   ├── client.rs     # Docker client implementation
│   │   └── container.rs  # Container data models
│   ├── ui/               # Terminal UI components
│   │   ├── render.rs     # Main rendering logic
│   │   └── widgets.rs    # UI widgets
│   └── event.rs          # Event handling
└── Cargo.toml            # Dependencies
```

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

## License

MIT License

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.