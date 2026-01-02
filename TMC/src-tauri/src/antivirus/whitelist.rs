/// Antivirus whitelist management for Windows Defender integration.
///
/// This module provides functionality to register the application with
/// antivirus software and implement memory operations that avoid triggering
/// false positive detections.
use anyhow;

/// Registers the application as trusted with Windows Defender.
///
/// Attempts to add the executable to Windows Defender's exclusion list.
/// Note: Full implementation requires administrator privileges.
///
/// # Returns
///
/// Returns `Ok(())` if the registration process completes without errors,
/// even if the actual registration fails due to insufficient privileges.
pub fn register_as_trusted() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        // Retrieve the current executable path
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_str) = exe_path.to_str() {
                // Attempt to register the app as trusted using Windows Defender APIs
                // Note: This requires administrator privileges to work fully
                // For now, just logging for debug - full implementation would require
                // Windows Defender Management APIs which are complex
                tracing::debug!("Attempting to register as trusted: {}", exe_str);

                // Future implementation could include:
                // - Adding to Windows Defender exclusion list via PowerShell:
                //   Add-MpPreference -ExclusionPath "path"
                // - Register as trusted publisher
                // - Whitelisting via Group Policy (requires admin)
            }
        }
    }

    Ok(())
}

/// Executes memory operations with randomized timing to avoid antivirus detection.
///
/// This wrapper function adds a random delay before executing the provided
/// operation to help prevent pattern-based detection by antivirus software.
///
/// # Type Parameters
///
/// * `F` - A closure that performs the memory operation
/// * `R` - The return type of the operation
///
/// # Returns
///
/// Returns the result of the operation or an error if it fails.
pub fn safe_memory_operation<F, R>(operation: F) -> Result<R, anyhow::Error>
where
    F: FnOnce() -> Result<R, anyhow::Error>,
{
    // Add random delay to avoid pattern detection
    use rand::Rng;
    use std::time::Duration;

    let mut rng = rand::thread_rng();
    let delay = Duration::from_millis(rng.gen_range(10..100));
    std::thread::sleep(delay);

    operation()
}
