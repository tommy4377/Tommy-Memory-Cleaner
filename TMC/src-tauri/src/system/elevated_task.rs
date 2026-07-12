use anyhow::Result;
use std::process::Command;
use crate::config::get_portable_detector;
use tracing::{info, error, warn};

/// Task name for elevated execution
const ELEVATED_TASK_NAME: &str = "TommyMemoryCleanerElevated";

/// Creates an elevated scheduled task that can run the app without UAC prompt.
///
/// The task is on-demand only (`/sc once` with a start time already in the
/// past): it never fires by itself and is triggered explicitly via
/// `schtasks /run`. Logon autostart remains solely controlled by the
/// registry Run entry (system/startup.rs); using `onlogon` here caused a
/// duplicate launch at every logon (task + registry) and an
/// "Another instance is already running" popup.
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
        "/sc", "once",
        "/st", "00:00", // Start time in the past: on-demand only, never self-fires
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

/// Checks that the elevated task exists AND its action points at the
/// currently running executable.
///
/// `schtasks /run` reports success even when the task's target exe no longer
/// exists (e.g., the app was moved or updated in place), in which case nothing
/// is launched. Trusting a stale task made the app exit at startup without
/// ever starting the elevated instance. Callers should fall back to a UAC
/// prompt when this returns false; the elevated instance then recreates the
/// task with the correct path.
pub fn elevated_task_matches_current_exe() -> bool {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let exe_str = exe.to_string_lossy().to_lowercase();

    let mut cmd = Command::new("schtasks");
    cmd.args([
        "/query",
        "/tn", ELEVATED_TASK_NAME,
        "/xml",
    ]);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    match cmd.output() {
        Ok(output) if output.status.success() => {
            // The XML contains the action as <Command>"C:\path\to\exe"</Command>.
            // A simple case-insensitive substring check is sufficient here; on a
            // false negative we merely fall back to the UAC prompt.
            String::from_utf8_lossy(&output.stdout)
                .to_lowercase()
                .contains(&exe_str)
        }
        _ => false,
    }
}
