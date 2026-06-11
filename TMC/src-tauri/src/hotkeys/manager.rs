//! Global Shortcut Management Module
//!
//! This module handles the registration and parsing of system-wide hotkeys
//! using the Tauri v2 Global Shortcut plugin. It provides utilities to
//! convert string representations of shortcuts into hardware-level key codes
//! and modifier bitflags.

use crate::config::Config;
use crate::hotkeys::codes::code_from_str;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut};

/// Canonical hotkey strings that are reserved by the operating system and must
/// never be intercepted by the application. Covers system security shortcuts,
/// OS navigation shortcuts, and universal application shortcuts.
const SYSTEM_HOTKEY_BLACKLIST: &[&str] = &[
    // System Security
    "CTRL+ALT+DEL", "CTRL+SHIFT+ESC", "WIN+L", "ALT+F4", "CTRL+ESC",
    "ALT+TAB", "ALT+SHIFT+TAB", "WIN+TAB", "CTRL+ALT+TAB",
    // System Navigation
    "WIN+E", "WIN+R", "WIN+S", "WIN+Q", "WIN+I", "WIN+A", "WIN+K",
    "WIN+P", "WIN+U", "WIN+V", "SHIFT+WIN+S", "WIN+SPACE",
    "CTRL+WIN+D", "CTRL+WIN+LEFT", "CTRL+WIN+RIGHT", "ALT+SPACE",
    // Universal Application Shortcuts
    "CTRL+C", "CTRL+V", "CTRL+X", "CTRL+Z", "CTRL+Y", "CTRL+A",
    "CTRL+S", "CTRL+O", "CTRL+N", "CTRL+P", "CTRL+W", "CTRL+Q",
    "CTRL+F", "CTRL+H", "CTRL+SHIFT+DEL", "CTRL+SHIFT+N", "CTRL+SHIFT+T",
    "CTRL+R", "CTRL+SHIFT+R", "ALT+LEFT", "ALT+RIGHT",
];

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

    // Build canonical form for blacklist matching
    let mut canonical_mods = Vec::new();
    if mods.contains(Modifiers::CONTROL) { canonical_mods.push("CTRL"); }
    if mods.contains(Modifiers::ALT)     { canonical_mods.push("ALT"); }
    if mods.contains(Modifiers::SHIFT)   { canonical_mods.push("SHIFT"); }
    if mods.contains(Modifiers::SUPER)   { canonical_mods.push("WIN"); }

    let canonical = if canonical_mods.is_empty() {
        key.clone()
    } else {
        format!("{}+{}", canonical_mods.join("+"), key)
    };

    if SYSTEM_HOTKEY_BLACKLIST.contains(&canonical.as_str()) {
        return Err(format!(
            "Hotkey '{}' is reserved by the operating system and cannot be used",
            hotkey
        ));
    }

    Ok((mods, key))
}

/// Configures and registers a global hotkey within the Tauri application context.
///
/// This function tracks the currently registered hotkey and only unregisters
/// the specific previous hotkey (if different) before registering the new one,
/// avoiding destruction of other shortcuts registered by plugins or the app.
pub fn register_global_hotkey_v2(
    app: &AppHandle,
    hotkey: &str,
    _cfg: Arc<Mutex<Config>>,
) -> Result<(), String> {
    let (modifiers, key) = parse_hotkey_for_v2(hotkey)?;
    let code = code_from_str(&key)?;
    let new_shortcut = Shortcut::new(Some(modifiers), code);

    // Unregister only the previously tracked hotkey, not ALL shortcuts
    if let Some(state) = app.try_state::<crate::AppState>() {
        if let Ok(tracked) = state.registered_hotkey.lock() {
            if let Some(old_hotkey) = tracked.clone() {
                if old_hotkey != hotkey {
                    // Parse and unregister the old hotkey specifically
                    if let Ok((old_mods, old_key)) = parse_hotkey_for_v2(&old_hotkey) {
                        if let Ok(old_code) = code_from_str(&old_key) {
                            let old_shortcut = Shortcut::new(Some(old_mods), old_code);
                            let _ = app.global_shortcut().unregister(old_shortcut);
                        }
                    }
                }
            }
        }
    }

    // Register the new hotkey
    app.global_shortcut()
        .register(new_shortcut)
        .map_err(|e| e.to_string())?;

    // Update tracking
    if let Some(state) = app.try_state::<crate::AppState>() {
        if let Ok(mut tracked) = state.registered_hotkey.lock() {
            *tracked = Some(hotkey.to_string());
        }
    }

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
