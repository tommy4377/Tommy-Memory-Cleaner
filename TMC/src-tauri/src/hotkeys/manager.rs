//! Global Shortcut Management Module
//!
//! This module handles the registration and parsing of system-wide hotkeys
//! using the Tauri v2 Global Shortcut plugin. It provides utilities to
//! convert string representations of shortcuts into hardware-level key codes
//! and modifier bitflags.

use crate::config::Config;
use crate::hotkeys::codes::code_from_str;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut};

/// Parses a human-readable hotkey string into Tauri Modifiers and a key identifier.
///
/// Supported modifiers: CTRL, ALT, SHIFT, SUPER/WIN.
/// The last element in the plus-separated string is treated as the primary key.
pub fn parse_hotkey_for_v2(hotkey: &str) -> Result<(Modifiers, String), String> {
    let parts: Vec<&str> = hotkey.split('+').map(|s| s.trim()).collect();
    let mut mods = Modifiers::empty();
    let mut key = String::new();

    for part in parts {
        match part.to_uppercase().as_str() {
            "CTRL" | "CONTROL" => mods |= Modifiers::CONTROL,
            "ALT" => mods |= Modifiers::ALT,
            "SHIFT" => mods |= Modifiers::SHIFT,
            "SUPER" | "WIN" | "META" => mods |= Modifiers::SUPER,
            _ => key = part.to_uppercase(),
        }
    }

    if key.is_empty() {
        return Err("No primary key found in hotkey string".to_string());
    }

    Ok((mods, key))
}

/// Configures and registers a global hotkey within the Tauri application context.
///
/// This function ensures that any previously registered shortcuts are cleared
/// before attempting to register the new hotkey to prevent conflicts.
pub fn register_global_hotkey_v2(
    app: &AppHandle,
    hotkey: &str,
    _cfg: Arc<Mutex<Config>>,
) -> Result<(), String> {
    // Clear previous registrations to ensure a clean state
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;

    // Deconstruct hotkey string and resolve hardware key code
    let (modifiers, key) = parse_hotkey_for_v2(hotkey)?;
    let code = code_from_str(&key)?;

    // Initialize the shortcut structure
    let shortcut = Shortcut::new(Some(modifiers), code);

    // Final registration with the operating system via Tauri plugin
    app.global_shortcut()
        .register(shortcut)
        .map_err(|e| e.to_string())?;

    tracing::info!("Global hotkey successfully registered: {}", hotkey);
    Ok(())
}

/// Tauri IPC command to dynamically update the global hotkey from the frontend.
///
/// Accesses the application state to retrieve configuration before triggering
/// the underlying registration logic.
#[tauri::command]
pub fn cmd_register_hotkey(
    app: AppHandle,
    hotkey: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    register_global_hotkey_v2(&app, &hotkey, state.cfg.clone())
}
