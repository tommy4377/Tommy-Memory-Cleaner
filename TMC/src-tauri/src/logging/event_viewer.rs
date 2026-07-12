// src-tauri/src/logging/event_viewer.rs

use anyhow::Result;
use once_cell::sync::Lazy;
use std::ptr::null_mut;
use std::sync::Arc;
use parking_lot::Mutex;
use windows_sys::Win32::Foundation::{GetLastError, HANDLE};
use windows_sys::Win32::System::EventLog::*;
use windows_sys::Win32::System::Registry::*;

const EVENT_SOURCE: &str = "TommyMemoryCleaner";
const REGISTRY_PATH: &str =
    r"SYSTEM\CurrentControlSet\Services\EventLog\Application\TommyMemoryCleaner";

// Event IDs for different event types
const EVENT_ID_STARTUP: u32 = 100;
const EVENT_ID_SHUTDOWN: u32 = 200;
const EVENT_ID_OPTIMIZATION: u32 = 1000;
const EVENT_ID_AUTO_OPTIMIZATION: u32 = 1100;
const EVENT_ID_ERROR: u32 = 2000;

// Thread-safe wrapper for HANDLE
struct SafeHandle {
    handle: *mut std::ffi::c_void,
}

unsafe impl Send for SafeHandle {}
unsafe impl Sync for SafeHandle {}

impl SafeHandle {
    fn new(handle: HANDLE) -> Self {
        Self {
            handle: handle as *mut std::ffi::c_void,
        }
    }

    fn as_handle(&self) -> HANDLE {
        self.handle as HANDLE
    }

    fn is_valid(&self) -> bool {
        !self.handle.is_null()
    }
}

impl Drop for SafeHandle {
    fn drop(&mut self) {
        unsafe {
            if self.is_valid() {
                DeregisterEventSource(self.as_handle());
            }
        }
    }
}

// Main logger with Arc for thread-safe sharing
struct EventLoggerInner {
    handle: SafeHandle,
}

impl EventLoggerInner {
    fn new() -> Result<Self> {
        // Auto-register if necessary
        Self::ensure_event_source_registered();

        unsafe {
            let source = to_wide(EVENT_SOURCE);
            let handle = RegisterEventSourceW(null_mut(), source.as_ptr());

            // HANDLE in windows-sys is isize, so compare with 0
            if handle == std::ptr::null_mut() {
                // Fallback: try with Application directly
                let app_source = to_wide("Application");
                let fallback_handle = RegisterEventSourceW(null_mut(), app_source.as_ptr());

                // HANDLE in windows-sys is isize, so compare with 0
                if fallback_handle == std::ptr::null_mut() {
                    anyhow::bail!(
                        "Failed to register event source (error: {})",
                        GetLastError()
                    );
                }

                Ok(Self {
                    handle: SafeHandle::new(fallback_handle),
                })
            } else {
                Ok(Self {
                    handle: SafeHandle::new(handle),
                })
            }
        }
    }

    fn ensure_event_source_registered() {
        unsafe {
            let mut hkey: HKEY = std::ptr::null_mut();
            let path = to_wide(REGISTRY_PATH);

            // Try to create/open the registry key
            let result = RegCreateKeyExW(
                HKEY_LOCAL_MACHINE,
                path.as_ptr(),
                0,
                null_mut(),
                0, // REG_OPTION_NON_VOLATILE
                KEY_WRITE,
                null_mut(),
                &mut hkey,
                null_mut(),
            );

            // HKEY in windows-sys is isize, so compare with 0
            if result != 0 || hkey == std::ptr::null_mut() {
                // Couldn't create the key, likely not running as admin
                // Not a critical error, continue anyway
                return;
            }

            // Set EventMessageFile
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_str) = exe_path.to_str() {
                    let exe_wide = to_wide(exe_str);
                    let value_name = to_wide("EventMessageFile");

                    RegSetValueExW(
                        hkey,
                        value_name.as_ptr(),
                        0,
                        REG_SZ,
                        exe_wide.as_ptr() as *const u8,
                        (exe_wide.len() * 2) as u32,
                    );
                }
            }

            // Set TypesSupported
            let types_name = to_wide("TypesSupported");
            let types_value: u32 = EVENTLOG_ERROR_TYPE as u32
                | EVENTLOG_WARNING_TYPE as u32
                | EVENTLOG_INFORMATION_TYPE as u32;

            RegSetValueExW(
                hkey,
                types_name.as_ptr(),
                0,
                REG_DWORD,
                &types_value as *const u32 as *const u8,
                4,
            );

            // Set CategoryCount
            let cat_name = to_wide("CategoryCount");
            let cat_value: u32 = 0;

            RegSetValueExW(
                hkey,
                cat_name.as_ptr(),
                0,
                REG_DWORD,
                &cat_value as *const u32 as *const u8,
                4,
            );

            RegCloseKey(hkey);
        }
    }

    fn write_event(&self, event_type: u16, event_id: u32, message: &str) -> Result<()> {
        if !self.handle.is_valid() {
            anyhow::bail!("Invalid event log handle");
        }

        unsafe {
            // FIX: Make sure the buffer stays valid during the call
            // Convert the message to a wide string and keep it in scope
            let msg_wide = to_wide(message);

            // FIX: Limit the message length to avoid overflow
            // Windows Event Log has a limit of ~32KB per message
            let max_len = 30000; // Safe limit
            let msg_wide = if msg_wide.len() > max_len {
                let mut truncated = msg_wide[..max_len].to_vec();
                truncated.push(0); // Null terminator
                truncated
            } else {
                msg_wide
            };

            let msg_ptr = msg_wide.as_ptr();

            // FIX: Build the string array safely
            // The pointer must remain valid during the call
            let strings: [*const u16; 1] = [msg_ptr];

            // FIX: Make sure the vector isn't deallocated during the call
            // Keep msg_wide in scope until the end
            let result = ReportEventW(
                self.handle.as_handle(),
                event_type,
                0, // category
                event_id,
                null_mut(), // user SID
                1,          // number of strings
                0,          // data size
                strings.as_ptr() as *const *const u16,
                null_mut(), // raw data
            );

            // msg_wide remains valid up to this point

            if result == 0 {
                let error = GetLastError();
                tracing::debug!("Failed to write event log entry: {}", error);
                // We don't propagate the error so as not to block the app
            }

            Ok(())
        }
    }
}

// Singleton globale thread-safe
static EVENT_LOGGER: Lazy<Arc<Mutex<Option<EventLoggerInner>>>> =
    Lazy::new(|| match EventLoggerInner::new() {
        Ok(logger) => {
            tracing::info!("Event Logger initialized successfully");
            Arc::new(Mutex::new(Some(logger)))
        }
        Err(e) => {
            tracing::info!("Event Logger not available (OK if not admin): {}", e);
            Arc::new(Mutex::new(None))
        }
    });

// Helper to convert strings to Windows wide strings
fn to_wide(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

// Helper function to get a formatted timestamp
fn get_timestamp() -> String {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;

    let mut st: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut st) };

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        st.wYear, st.wMonth, st.wDay,
        st.wHour, st.wMinute, st.wSecond
    )
}

// ========== PUBLIC FUNCTIONS ==========

/// Logs application startup
pub fn log_startup_event(version: &str, config_loaded: bool) {
    // FIX: Limit the message length to avoid issues
    let exe_path = std::env::current_exe()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Limit the path length if it's too long
    let exe_display = if exe_path.len() > 200 {
        format!("{}...", &exe_path[..200])
    } else {
        exe_path
    };

    let message = format!(
        "Tommy Memory Cleaner Started\nVersion: {}\nConfiguration: {}\nProcess ID: {}\nExecutable: {}\nTimestamp: {}",
        version,
        if config_loaded { "Loaded successfully" } else { "Using defaults" },
        std::process::id(),
        exe_display,
        get_timestamp()
    );

    write_log(EVENTLOG_INFORMATION_TYPE, EVENT_ID_STARTUP, &message);
}

/// Logs application shutdown
pub fn log_shutdown_event() {
    let message = format!(
        "Tommy Memory Cleaner Shutdown\n\
        =====================================\n\
        Process ID: {}\n\
        Timestamp: {}",
        std::process::id(),
        get_timestamp()
    );

    write_log(EVENTLOG_INFORMATION_TYPE, EVENT_ID_SHUTDOWN, &message);
}

/// Logs a completed optimization
pub fn log_optimization_event(
    memory_freed_mb: f64,
    profile: &str,
    mode: &str,
    areas: &str,
    duration_ms: u128,
    errors: &[String],
) {
    let success = errors.is_empty();
    let event_type = if success {
        EVENTLOG_INFORMATION_TYPE
    } else {
        EVENTLOG_WARNING_TYPE
    };

    let message = format!(
        "Memory Optimization Completed\n\
        =====================================\n\
        Profile: {}\n\
        Mode: {}\n\
        Memory Freed: {:.2} MB\n\
        Duration: {} ms\n\
        Areas Cleaned: {}\n\
        Status: {}\n\
        Timestamp: {}\n\
        {}",
        profile,
        mode,
        memory_freed_mb,
        duration_ms,
        areas,
        if success {
            "SUCCESS"
        } else {
            "COMPLETED WITH WARNINGS"
        },
        get_timestamp(),
        if !errors.is_empty() {
            format!("\nWarnings:\n{}", errors.join("\n"))
        } else {
            String::new()
        }
    );

    write_log(event_type, EVENT_ID_OPTIMIZATION, &message);
}

/// Logs an automatic optimization
pub fn log_auto_optimization_event(reason: &str, threshold: u8) {
    let message = format!(
        "Automatic Optimization Triggered\n\
        =====================================\n\
        Reason: {}\n\
        Threshold: {}%\n\
        Timestamp: {}",
        reason,
        threshold,
        get_timestamp()
    );

    write_log(
        EVENTLOG_INFORMATION_TYPE,
        EVENT_ID_AUTO_OPTIMIZATION,
        &message,
    );
}

/// Logs a generic error
pub fn log_error_event(error: &str) {
    let message = format!(
        "Tommy Memory Cleaner Error\n\
        =====================================\n\
        Error: {}\n\
        Timestamp: {}",
        error,
        get_timestamp()
    );

    write_log(EVENTLOG_ERROR_TYPE, EVENT_ID_ERROR, &message);
}

// Internal helper function to write log entries
fn write_log(event_type: u16, event_id: u32, message: &str) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = EVENT_LOGGER.lock();
        if let Some(logger) = guard.as_ref() {
            let _ = logger.write_event(event_type, event_id, message);
        }
    }));

    if result.is_err() {
        tracing::debug!("Event log write panicked (non-critical, lock recovered)");
    }
}

// ========== TEST ==========
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_wide() {
        let wide = to_wide("test");
        assert_eq!(wide.len(), 5); // "test" + null terminator
        assert_eq!(wide[4], 0); // null terminator
    }

    #[test]
    fn test_safe_handle() {
        let handle = SafeHandle::new(null_mut());
        assert!(!handle.is_valid());

        let handle = SafeHandle::new(1 as HANDLE);
        assert!(handle.is_valid());
    }

    #[test]
    fn test_timestamp() {
        let ts = get_timestamp();
        assert!(!ts.is_empty());
        assert!(ts.contains("-"));
        assert!(ts.contains(":"));
    }
}
