pub mod config;
pub mod memory;
pub mod system;
pub mod theme;
pub mod ui;
pub mod i18n;

// Re-export commonly used functions
pub use ui::{show_or_create_window, position_tray_menu};
pub use i18n::{TranslationState, get_translation};

