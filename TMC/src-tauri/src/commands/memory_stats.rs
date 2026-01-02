use serde::{Deserialize, Serialize};
use tauri::Manager;

/// Memory statistics data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_freed_gb: f64,
    pub last_updated: String,
}

/// Get memory statistics from app data directory
#[tauri::command]
pub async fn get_memory_stats(app: tauri::AppHandle) -> Result<MemoryStats, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let stats_file = app_data_dir.join("memory_stats.json");
    
    if stats_file.exists() {
        let content = std::fs::read_to_string(&stats_file).map_err(|e| e.to_string())?;
        let stats: MemoryStats = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        Ok(stats)
    } else {
        // Return default stats if file doesn't exist
        Ok(MemoryStats {
            total_freed_gb: 0.0,
            last_updated: "1970-01-01T00:00:00Z".to_string(),
        })
    }
}

/// Save memory statistics to app data directory
#[tauri::command]
pub async fn save_memory_stats(
    app: tauri::AppHandle,
    total_freed_gb: f64,
    last_updated: String,
) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    
    // Ensure directory exists
    std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;
    
    let stats_file = app_data_dir.join("memory_stats.json");
    let stats = MemoryStats {
        total_freed_gb,
        last_updated,
    };
    
    let content = serde_json::to_string_pretty(&stats).map_err(|e| e.to_string())?;
    std::fs::write(&stats_file, content).map_err(|e| e.to_string())?;
    
    Ok(())
}
