//! Hotkeys Submodule Entry Point
//! 
//! This module exports the global shortcut management logic, providing 
//! both the internal registration functions and the Tauri-exposed commands.
//! It serves as the public interface for the application's hotkey system.

pub mod codes;
pub mod manager;

// Re-exporting core functionality for cleaner crate-level access
pub use manager::{cmd_register_hotkey, register_global_hotkey_v2};