use crate::config::Config;
use crate::hotkeys::codes::code_from_str;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut};

/// Parse a hotkey string into Modifiers and key string for Tauri v2
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
        return Err("No key specified in hotkey".to_string());
    }

    Ok((mods, key))
}

/// Register a global hotkey using Tauri v2 API
pub fn register_global_hotkey_v2(
    app: &AppHandle,
    hotkey: &str,
    _cfg: Arc<Mutex<Config>>,
) -> Result<(), String> {
    // First unregister all existing hotkeys
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;

    // Parse hotkey string
    let (modifiers, key) = parse_hotkey_for_v2(hotkey)?;
    let code = code_from_str(&key)?;

    // Create shortcut
    let shortcut = Shortcut::new(Some(modifiers), code);

    // Register the hotkey
    app.global_shortcut()
        .register(shortcut)
        .map_err(|e| e.to_string())?;

    tracing::info!("Global hotkey registered: {}", hotkey);
    Ok(())
}

/// Tauri command to register a global hotkey
#[tauri::command]
pub fn cmd_register_hotkey(
    app: AppHandle,
    hotkey: String,
    state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    register_global_hotkey_v2(&app, &hotkey, state.cfg.clone())
}
