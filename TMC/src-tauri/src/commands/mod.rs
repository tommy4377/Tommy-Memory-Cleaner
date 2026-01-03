/// Tauri command modules for backend functionality.
///
/// This module organizes all Tauri commands that expose backend functionality
/// to the frontend, including configuration management, memory operations,
/// system integration, theme handling, and UI management.
pub mod app_info;
pub mod config;
pub mod i18n;
pub mod memory;
pub mod memory_stats;
pub mod system;
pub mod theme;
pub mod ui;

// Re-export commonly used functions for convenient access
pub use i18n::{get_translation, TranslationState};
pub use ui::{position_tray_menu, show_or_create_window};
