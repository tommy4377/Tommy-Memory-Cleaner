use anyhow::Result;
use std::process::Command;
use crate::config::get_portable_detector;
use tracing::{info, error, warn};

/// Task name for elevated execution
const ELEVATED_TASK_NAME: &str = "TommyMemoryCleanerElevated";

/// Creates an elevated scheduled task that can run the app without UAC prompt
pub fn create_elevated_task() -> Result<()> {
    let detector = get_portable_detector();
    let exe_path = detector.exe_path();
    
    // Delete existing task if it exists
    delete_elevated_task()?;
    
    // Create new task with highest privileges
    let output = Command::new("schtasks")
        .args([
            "/create",
            "/tn", ELEVATED_TASK_NAME,
            "/tr", &format!("\"{}\"", exe_path.display()),
            "/sc", "onlogon",
            "/rl", "highest",
            "/f",
            "/it",  // Run only when user is logged on
        ])
        .output()?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        error!("Failed to create elevated task: {}", error);
        return Err(anyhow::anyhow!("Failed to create elevated task: {}", error));
    }
    
    info!("✓ Created elevated scheduled task");
    Ok(())
}

/// Runs the application through the elevated task (no UAC prompt)
pub fn run_via_elevated_task() -> Result<()> {
    info!("Running application via elevated task");
    
    let output = Command::new("schtasks")
        .args([
            "/run",
            "/tn", ELEVATED_TASK_NAME,
        ])
        .output()?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        error!("Failed to run elevated task: {}", error);
        return Err(anyhow::anyhow!("Failed to run elevated task: {}", error));
    }
    
    // Exit current process since the elevated task will launch a new instance
    std::process::exit(0);
}

/// Deletes the elevated task
pub fn delete_elevated_task() -> Result<()> {
    let output = Command::new("schtasks")
        .args([
            "/delete",
            "/tn", ELEVATED_TASK_NAME,
            "/f",
        ])
        .output()?;
    
    // Don't treat "task not found" as an error
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("ERROR: The system cannot find the file specified") {
            warn!("Task may not exist or other error: {}", stderr);
        }
    }
    
    Ok(())
}

/// Checks if the elevated task exists
pub fn elevated_task_exists() -> bool {
    let output = Command::new("schtasks")
        .args([
            "/query",
            "/tn", ELEVATED_TASK_NAME,
        ])
        .output();
    
    match output {
        Ok(result) => result.status.success(),
        Err(_) => false,
    }
}
