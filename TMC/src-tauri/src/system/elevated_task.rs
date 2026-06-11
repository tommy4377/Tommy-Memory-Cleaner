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
    let mut cmd = Command::new("schtasks");
    cmd.args([
        "/create",
        "/tn", ELEVATED_TASK_NAME,
        "/tr", &format!("\"{}\"", exe_path.display()),
        "/sc", "onlogon",
        "/rl", "highest",
        "/f",
        "/it",  // Run only when user is logged on
    ]);
    
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    
    let output = cmd.output()?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        error!("Failed to create elevated task: {}", error);
        return Err(anyhow::anyhow!("Failed to create elevated task: {}", error));
    }
    
    info!("✓ Created elevated scheduled task");
    Ok(())
}

/// Runs the application through the elevated task (no UAC prompt)
///
/// Returns `Ok(true)` if the elevated task was successfully triggered and the
/// current process should exit. Returns `Ok(false)` if no exit is needed.
/// The caller is responsible for performing a graceful shutdown rather than
/// calling `std::process::exit()` directly, so that Rust destructors run.
pub fn run_via_elevated_task() -> Result<bool> {
    info!("Running application via elevated task");
    
    let mut cmd = Command::new("schtasks");
    cmd.args([
        "/run",
        "/tn", ELEVATED_TASK_NAME,
    ]);
    
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    
    let output = cmd.output()?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        error!("Failed to run elevated task: {}", error);
        return Err(anyhow::anyhow!("Failed to run elevated task: {}", error));
    }
    
    // Signal the caller to exit gracefully — the elevated task will launch a new instance.
    // The caller should return from main() normally so Rust destructors run.
    Ok(true)
}

/// Deletes the elevated task
pub fn delete_elevated_task() -> Result<()> {
    let mut cmd = Command::new("schtasks");
    cmd.args([
        "/delete",
        "/tn", ELEVATED_TASK_NAME,
        "/f",
    ]);
    
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    
    let output = cmd.output()?;
    
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
    let mut cmd = Command::new("schtasks");
    cmd.args([
        "/query",
        "/tn", ELEVATED_TASK_NAME,
    ]);
    
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    
    let output = cmd.output();
    
    match output {
        Ok(result) => result.status.success(),
        Err(_) => false,
    }
}
