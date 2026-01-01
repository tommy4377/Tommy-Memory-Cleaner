use crate::config::Priority;
use tauri::{AppHandle, State};


#[tauri::command]
pub fn cmd_set_priority(
    state: State<'_, crate::AppState>, 
    priority: Priority
) -> Result<(), String> {
    crate::system::priority::set_priority(priority.clone())
        .map_err(|e| e.to_string())?;
    
    let mut cfg = state.cfg.lock()
        .map_err(|_| "Config lock poisoned".to_string())?;
    cfg.run_priority = priority;
    cfg.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cmd_run_on_startup(enable: bool, state: State<'_, crate::AppState>) -> Result<(), String> {
    crate::system::startup::set_run_on_startup(enable)
        .map_err(|e| format!("Failed to set startup: {}. Try running as administrator.", e))?;
    
    let is_enabled = crate::system::startup::is_startup_enabled();
    if enable && !is_enabled {
        return Err("Failed to enable startup. Please add the app manually to Windows startup.".to_string());
    }
    
    let mut cfg = state.cfg.lock()
        .map_err(|_| "Config lock poisoned".to_string())?;
    cfg.run_on_startup = is_enabled;
    cfg.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cmd_set_always_on_top(
    app: AppHandle, 
    on: bool, 
    state: State<'_, crate::AppState>
) -> Result<(), String> {
    crate::system::window::set_always_on_top(&app, on)?;
    
    let mut cfg = state.cfg.lock()
        .map_err(|_| "Config lock poisoned".to_string())?;
    cfg.always_on_top = on;
    cfg.save().map_err(|e| e.to_string())
}
