//! dkr - Docker TUI (Terminal User Interface)
//!
//! A terminal-based user interface for managing Docker containers, images, volumes, and networks.
//! Built with Ratatui for the UI and Bollard for Docker API interaction.
//!
//! # Features
//!
//! - Interactive container management (start/stop/restart/remove)
//! - Real-time container statistics (CPU/Memory)
//! - Log viewing with follow mode
//! - Image management and pull functionality
//! - Volume and network management
//! - JSON inspection of resources
//!
//! # Example
//!
//! ```no_run
//! use dkr::App;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let app = App::new().await?;
//!     // Run the TUI application
//!     Ok(())
//! }
//! ```

pub mod app;
pub mod config;
pub mod docker;
pub mod event;
pub mod handlers;
pub mod ui;

pub use app::App;
pub use config::Config;