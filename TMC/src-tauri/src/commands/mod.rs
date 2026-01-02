pub mod config;
pub mod i18n;
pub mod memory;
pub mod system;
pub mod theme;
pub mod ui;

// Re-export commonly used functions
pub use i18n::{get_translation, TranslationState};
pub use ui::{position_tray_menu, show_or_create_window};
