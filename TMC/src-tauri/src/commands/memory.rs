use crate::memory::types::{Areas, Reason};
use std::time::Duration;
use tauri::{AppHandle, State, Manager};

#[tauri::command]
pub fn cmd_memory_info(state: State<'_, crate::AppState>) -> Result<crate::memory::types::MemoryInfo, String> {
    state.engine.memory().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cmd_list_process_names() -> Result<Vec<String>, String> {
    Ok(crate::memory::ops::list_process_names())
}

#[tauri::command]
pub fn cmd_get_critical_processes() -> Result<Vec<String>, String> {
    Ok(crate::memory::critical_processes::get_critical_processes_list())
}

#[tauri::command]
pub fn cmd_optimize_async(
    app: AppHandle, 
    state: State<'_, crate::AppState>, 
    reason: Reason, 
    areas: String
) -> Result<(), String> {
    // Rate limiting check
    {
        let mut rl = state.rate_limiter.lock()
            .map_err(|_| "Rate limiter lock poisoned".to_string())?;
        if !rl.check_rate_limit("optimize") {
            return Err("Too many optimization requests. Please wait before trying again.".to_string());
        }
    }
    
    let engine = state.engine.clone();
    let cfg = state.cfg.clone();
    
    // Parse areas using the function from main.rs for now
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
                "" => {}, // Ignora stringhe vuote
                unknown => {
                    tracing::warn!("Unknown memory area flag: '{}' in areas string: '{}'", unknown, areas);
                }
            }
        }
        result
    };
    
    // Passa le aree direttamente a perform_optimization invece di modificare la config condivisa
    // Questo evita race conditions se due ottimizzazioni vengono avviate contemporaneamente
    tauri::async_runtime::spawn(async move {
        // Esegui l'ottimizzazione (il flag viene gestito automaticamente da perform_optimization)
        crate::perform_optimization(app.clone(), engine, cfg.clone(), reason, true, Some(areas_flags)).await;
        
        // Gestisci chiusura dopo ottimizzazione se configurato
        if reason == Reason::Manual {
            // FIX: Rilascia il lock prima dell'await
            let should_close = cfg.lock()
                .map(|c| c.close_after_opt)
                .unwrap_or(false);
            
            if should_close {
                tokio::time::sleep(Duration::from_secs(2)).await;
                if let Some(window) = app.get_webview_window("main") {
                    let _: Result<(), _> = window.close();
                }
            }
        }
        // NOTA: Il flag OPTIMIZATION_RUNNING viene rilasciato automaticamente da scopeguard in perform_optimization
    });
    
    Ok(())
}
