/// Memory management commands for the Tauri backend.
/// 
/// This module provides Tauri commands for memory optimization operations,
/// including memory information retrieval, process listing, and both synchronous
/// and asynchronous memory optimization functionality.

use crate::memory::types::{Areas, Reason};
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

/// Retrieves current memory usage information.
/// 
/// # Returns
/// 
/// Returns a `MemoryInfo` struct containing detailed memory statistics
/// for physical and virtual memory, or an error string if the operation fails.
#[tauri::command]
pub fn cmd_memory_info(
    state: State<'_, crate::AppState>,
) -> Result<crate::memory::types::MemoryInfo, String> {
    state.engine.memory().map_err(|e| e.to_string())
}

/// Retrieves a list of all running process names.
/// 
/// # Returns
/// 
/// Returns a vector containing the names of all currently running processes
/// on the system.
#[tauri::command]
pub fn cmd_list_process_names() -> Result<Vec<String>, String> {
    Ok(crate::memory::ops::list_process_names())
}

/// Retrieves a list of critical system processes.
/// 
/// These processes should not be terminated during memory optimization
/// to maintain system stability.
/// 
/// # Returns
/// 
/// Returns a vector containing the names of critical system processes
/// that are protected from optimization.
#[tauri::command]
pub fn cmd_get_critical_processes() -> Result<Vec<String>, String> {
    Ok(crate::memory::critical_processes::get_critical_processes_list())
}

/// Executes memory optimization asynchronously.
/// 
/// This command initiates memory optimization in a background task,
/// allowing the UI to remain responsive during the operation.
/// 
/// # Arguments
/// 
/// * `app` - The application handle for window management
/// * `state` - The application state containing the engine and configuration
/// * `reason` - The reason for optimization (manual, scheduled, low memory)
/// * `areas` - String representation of memory areas to optimize
/// 
/// # Returns
/// 
/// Returns `Ok(())` if the optimization task is started successfully,
/// or an error string if rate limiting is exceeded.
#[tauri::command]
pub fn cmd_optimize_async(
    app: AppHandle,
    state: State<'_, crate::AppState>,
    reason: Reason,
    areas: String,
) -> Result<(), String> {
    // Rate limiting check to prevent excessive optimization requests
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

    // Parse areas string to bitflags for memory optimization
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

    // Run optimization in background task to avoid blocking UI
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

        // Handle automatic window closing after optimization if configured
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
