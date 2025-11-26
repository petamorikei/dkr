mod inspect_viewer;
mod log_viewer;
mod render;
mod stats_viewer;
mod theme;
mod utils;
mod widgets;

pub use inspect_viewer::{InspectViewer, draw_inspect_popup};
pub use log_viewer::{LogViewer, draw_log_popup};
pub use render::render;
pub use stats_viewer::{StatsViewer, draw_stats_popup};
pub use theme::Theme;
pub use utils::centered_rect;
pub use widgets::{draw_containers_tab, draw_images_tab, draw_networks_tab, draw_volumes_tab};
