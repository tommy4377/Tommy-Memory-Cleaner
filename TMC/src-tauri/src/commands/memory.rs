/// Memory management commands for Tauri backend
use crate::memory::types::{Areas, Reason};
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

/// Get current memory usage information
#[tauri::command]
pub fn cmd_memory_info(
    state: State<'_, crate::AppState>,
) -> Result<crate::memory::types::MemoryInfo, String> {
    state.engine.memory().map_err(|e| e.to_string())
}

/// Get list of all running process names
#[tauri::command]
pub fn cmd_list_process_names() -> Result<Vec<String>, String> {
    Ok(crate::memory::ops::list_process_names())
}

/// Get list of critical system processes
#[tauri::command]
pub fn cmd_get_critical_processes() -> Result<Vec<String>, String> {
    Ok(crate::memory::critical_processes::get_critical_processes_list())
}

/// Execute memory optimization asynchronously
#[tauri::command]
pub fn cmd_optimize_async(
    app: AppHandle,
    state: State<'_, crate::AppState>,
    reason: Reason,
    areas: String,
) -> Result<(), String> {
    // Rate limiting check
    {
        let mut rl = state
            .rate_limiter
            .lock()
            .map_err(|_| "Rate limiter lock poisoned".to_string())?;
        if !rl.check_rate_limit("optimize") {
            return Err(
                "Too many optimization requests. Please wait before trying again.".to_string(),
            );
        }
    }

    let engine = state.engine.clone();
    let cfg = state.cfg.clone();

    // Parse areas string to bitflags
    let areas_flags = {
        let mut result = Areas::empty();
        for flag in areas.split('|') {
            match flag.trim() {
                "COMBINED_PAGE_LIST" => result |= Areas::COMBINED_PAGE_LIST,
                "MODIFIED_FILE_CACHE" => result |= Areas::MODIFIED_FILE_CACHE,
                "MODIFIED_PAGE_LIST" => result |= Areas::MODIFIED_PAGE_LIST,
                "REGISTRY_CACHE" => result |= Areas::REGISTRY_CACHE,
                "STANDBY_LIST" => result |= Areas::STANDBY_LIST,
                "STANDBY_LIST_LOW" => result |= Areas::STANDBY_LIST_LOW,
                "SYSTEM_FILE_CACHE" => result |= Areas::SYSTEM_FILE_CACHE,
                "WORKING_SET" => result |= Areas::WORKING_SET,
                "" => {}
                unknown => {
                    tracing::warn!(
                        "Unknown memory area flag: '{}' in areas string: '{}'",
                        unknown,
                        areas
                    );
                }
            }
        }
        result
    };

    // Run optimization in background
    tauri::async_runtime::spawn(async move {
        crate::perform_optimization(
            app.clone(),
            engine,
            cfg.clone(),
            reason,
            true,
            Some(areas_flags),
        )
        .await;

        // Handle auto-close after optimization
        if reason == Reason::Manual {
            let should_close = cfg.lock().map(|c| c.close_after_opt).unwrap_or(false);

            if should_close {
                tokio::time::sleep(Duration::from_secs(2)).await;
                if let Some(window) = app.get_webview_window("main") {
                    let _: Result<(), _> = window.close();
                }
            }
        }
    });

    Ok(())
}
