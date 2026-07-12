use crate::config::Priority;
use tauri::{AppHandle, State};

/// System configuration commands for the Tommy Memory Cleaner application.
///
/// This module provides Tauri commands to manage system-level settings
/// including process priority, startup behavior, and window properties.

/// Restarts the application with elevated privileges.
///
/// Persists the elevation preference in config BEFORE launching, because the
/// new elevated instance reads it at startup to create the silent-elevation
/// scheduled task. If the launch fails (e.g., the user declines the UAC
/// prompt), the previous preference is restored so the next startup does not
/// prompt unexpectedly, and the current process keeps running.
#[tauri::command]
pub fn cmd_restart_with_elevation(
    app: AppHandle,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        // Persist the elevation preference so future launches auto-elevate
        let previous_preference = {
            let mut cfg = state
                .cfg
                .lock()
                .map_err(|_| "Config lock poisoned".to_string())?;
            let previous = cfg.request_elevation_on_startup;
            cfg.request_elevation_on_startup = true;
            if let Err(e) = cfg.save() {
                tracing::warn!("Failed to save elevation preference: {}", e);
            }
            previous
        };

        if let Err(e) = crate::restart_with_elevation(&app) {
            // The elevated instance did not start: roll back the preference
            // if we were the ones who flipped it on.
            if !previous_preference {
                if let Ok(mut cfg) = state.cfg.lock() {
                    cfg.request_elevation_on_startup = false;
                    if let Err(save_err) = cfg.save() {
                        tracing::warn!("Failed to roll back elevation preference: {}", save_err);
                    }
                }
            }
            return Err(e.to_string());
        }
        Ok(())
    }

    #[cfg(not(windows))]
    {
        let _ = app;
        let _ = state;
        Err("Elevation is only supported on Windows".to_string())
    }
}

/// Manages the elevated task for silent admin startup.
#[tauri::command]
pub fn cmd_manage_elevated_task(create: bool) -> Result<(), String> {
    if create {
        #[cfg(windows)]
        {
            use crate::system::elevated_task::create_elevated_task;
            create_elevated_task().map_err(|e| e.to_string())?
        }
        #[cfg(not(windows))]
        {
            return Err("Elevated task is only supported on Windows".to_string());
        }
    } else {
        #[cfg(windows)]
        {
            use crate::system::elevated_task::delete_elevated_task;
            delete_elevated_task().map_err(|e| e.to_string())?
        }
        #[cfg(not(windows))]
        {
            return Err("Elevated task is only supported on Windows".to_string());
        }
    }
    Ok(())
}

/// Sets the application process priority.
///
/// Updates both the current process priority and persists the setting
/// in the application configuration for future sessions.
#[tauri::command]
pub fn cmd_set_priority(
    state: State<'_, crate::AppState>,
    priority: Priority,
) -> Result<(), String> {
    crate::system::priority::set_priority(priority.clone()).map_err(|e| e.to_string())?;

    let mut cfg = state
        .cfg
        .lock()
        .map_err(|_| "Config lock poisoned".to_string())?;
    cfg.run_priority = priority;
    cfg.save().map_err(|e| e.to_string())
}

/// Configures the application to run automatically on system startup.
///
/// Attempts to set the startup preference in Windows registry and verifies
/// the operation was successful. Persists the setting in the application
/// configuration for consistency.
#[tauri::command]
pub fn cmd_run_on_startup(enable: bool, state: State<'_, crate::AppState>) -> Result<(), String> {
    crate::system::startup::set_run_on_startup(enable).map_err(|e| {
        format!(
            "Failed to set startup: {}. Try running as administrator.",
            e
        )
    })?;

    let is_enabled = crate::system::startup::is_startup_enabled();
    if enable && !is_enabled {
        return Err(
            "Failed to enable startup. Please add the app manually to Windows startup.".to_string(),
        );
    }

    let mut cfg = state
        .cfg
        .lock()
        .map_err(|_| "Config lock poisoned".to_string())?;
    cfg.run_on_startup = is_enabled;
    cfg.save().map_err(|e| e.to_string())
}

/// Controls the window's "always on top" behavior.
///
/// Sets or removes the always-on-top property for the application window
/// and persists this preference in the configuration for future sessions.
#[tauri::command]
pub fn cmd_set_always_on_top(
    app: AppHandle,
    on: bool,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    crate::system::window::set_always_on_top(&app, on)?;

    let mut cfg = state
        .cfg
        .lock()
        .map_err(|_| "Config lock poisoned".to_string())?;
    cfg.always_on_top = on;
    cfg.save().map_err(|e| e.to_string())
}
