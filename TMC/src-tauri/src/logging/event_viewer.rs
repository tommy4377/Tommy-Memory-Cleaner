// src-tauri/src/logging/event_viewer.rs

use anyhow::Result;
use std::ptr::null_mut;
use std::sync::Arc;
use windows_sys::Win32::System::EventLog::*;
use windows_sys::Win32::Foundation::{HANDLE, GetLastError};
use windows_sys::Win32::System::Registry::*;
use once_cell::sync::Lazy;
use std::sync::Mutex;

const EVENT_SOURCE: &str = "TommyMemoryCleaner";
const REGISTRY_PATH: &str = r"SYSTEM\CurrentControlSet\Services\EventLog\Application\TommyMemoryCleaner";

// Event IDs per diversi tipi di eventi
const EVENT_ID_STARTUP: u32 = 100;
const EVENT_ID_SHUTDOWN: u32 = 200;
const EVENT_ID_OPTIMIZATION: u32 = 1000;
const EVENT_ID_AUTO_OPTIMIZATION: u32 = 1100;
const EVENT_ID_ERROR: u32 = 2000;

// Wrapper thread-safe per HANDLE
struct SafeHandle {
    handle: *mut std::ffi::c_void,
}

unsafe impl Send for SafeHandle {}
unsafe impl Sync for SafeHandle {}

impl SafeHandle {
    fn new(handle: HANDLE) -> Self {
        Self { handle: handle as *mut std::ffi::c_void }
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

// Logger principale con Arc per condivisione thread-safe
struct EventLoggerInner {
    handle: SafeHandle,
}

impl EventLoggerInner {
    fn new() -> Result<Self> {
        // Auto-registra se necessario
        Self::ensure_event_source_registered();
        
        unsafe {
            let source = to_wide(EVENT_SOURCE);
            let handle = RegisterEventSourceW(null_mut(), source.as_ptr());
            
            // HANDLE in windows-sys is isize, so compare with 0
            if handle == 0 {
                // Fallback: prova con Application direttamente
                let app_source = to_wide("Application");
                let fallback_handle = RegisterEventSourceW(null_mut(), app_source.as_ptr());
                
                // HANDLE in windows-sys is isize, so compare with 0
                if fallback_handle == 0 {
                    anyhow::bail!("Failed to register event source (error: {})", GetLastError());
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
            let mut hkey: HKEY = 0;
            let path = to_wide(REGISTRY_PATH);
            
            // Prova a creare/aprire la chiave del registro
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
            if result != 0 || hkey == 0 {
                // Non riusciamo a creare la chiave, probabilmente non siamo admin
                // Non è un errore critico, continua comunque
                return;
            }
            
            // Imposta EventMessageFile
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
            
            // Imposta TypesSupported
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
            
            // Imposta CategoryCount
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
            // FIX: Assicurati che il buffer rimanga valido durante la chiamata
            // Converti il messaggio in wide string e mantienilo in scope
            let msg_wide = to_wide(message);
            
            // FIX: Limita la lunghezza del messaggio per evitare overflow
            // Windows Event Log ha un limite di ~32KB per messaggio
            let max_len = 30000; // Limite sicuro
            let msg_wide = if msg_wide.len() > max_len {
                let mut truncated = msg_wide[..max_len].to_vec();
                truncated.push(0); // Null terminator
                truncated
            } else {
                msg_wide
            };
            
            let msg_ptr = msg_wide.as_ptr();
            
            // FIX: Crea l'array di stringhe in modo sicuro
            // Il puntatore deve rimanere valido durante la chiamata
            let strings: [*const u16; 1] = [msg_ptr];
            
            // FIX: Assicurati che il vettore non venga deallocato durante la chiamata
            // Manteniamo msg_wide in scope fino alla fine
            let result = ReportEventW(
                self.handle.as_handle(),
                event_type,
                0, // category
                event_id,
                null_mut(), // user SID
                1, // number of strings
                0, // data size
                strings.as_ptr() as *const *const u16,
                null_mut(), // raw data
            );
            
            // msg_wide rimane valido fino a qui
            
            if result == 0 {
                let error = GetLastError();
                tracing::debug!("Failed to write event log entry: {}", error);
                // Non propaghiamo l'errore per non bloccare l'app
            }
            
            Ok(())
        }
    }
}

// Singleton globale thread-safe
static EVENT_LOGGER: Lazy<Arc<Mutex<Option<EventLoggerInner>>>> = Lazy::new(|| {
    match EventLoggerInner::new() {
        Ok(logger) => {
            tracing::info!("Event Logger initialized successfully");
            Arc::new(Mutex::new(Some(logger)))
        },
        Err(e) => {
            tracing::info!("Event Logger not available (OK if not admin): {}", e);
            Arc::new(Mutex::new(None))
        }
    }
});

// Helper per convertire stringhe in wide strings Windows
fn to_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsStr;
    
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

// Funzione helper per ottenere timestamp formattato
fn get_timestamp() -> String {
    use std::time::SystemTime;
    
    // Usa SystemTime invece di chrono per evitare una dipendenza extra
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => {
            let total_secs = duration.as_secs();
            let secs_per_day = 86400;
            let days_since_epoch = total_secs / secs_per_day;
            
            // Calcolo approssimativo (per logging è sufficiente)
            let year = 1970 + (days_since_epoch / 365);
            let month = ((days_since_epoch % 365) / 30) + 1;
            let day = ((days_since_epoch % 365) % 30) + 1;
            let hour = (total_secs % secs_per_day) / 3600;
            let minute = ((total_secs % secs_per_day) % 3600) / 60;
            let second = total_secs % 60;
            
            format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", 
                    year, month, day, hour, minute, second)
        }
        Err(_) => "Unknown time".to_string()
    }
}

// ========== FUNZIONI PUBBLICHE ==========

/// Log dell'avvio dell'applicazione
pub fn log_startup_event(version: &str, config_loaded: bool) {
    // FIX: Limita la lunghezza del messaggio per evitare problemi
    let exe_path = std::env::current_exe()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    
    // Limita la lunghezza del path se troppo lungo
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

/// Log dello shutdown dell'applicazione
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

/// Log di un'ottimizzazione completata
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
        if success { "SUCCESS" } else { "COMPLETED WITH WARNINGS" },
        get_timestamp(),
        if !errors.is_empty() {
            format!("\nWarnings:\n{}", errors.join("\n"))
        } else {
            String::new()
        }
    );
    
    write_log(event_type, EVENT_ID_OPTIMIZATION, &message);
}

/// Log di un'ottimizzazione automatica
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
    
    write_log(EVENTLOG_INFORMATION_TYPE, EVENT_ID_AUTO_OPTIMIZATION, &message);
}

/// Log di un errore generico
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

// Funzione helper interna per scrivere i log
fn write_log(event_type: u16, event_id: u32, message: &str) {
    // FIX: Non crashare se il logging fallisce - usa catch_unwind
    let result = std::panic::catch_unwind(|| {
        if let Ok(guard) = EVENT_LOGGER.lock() {
            if let Some(logger) = guard.as_ref() {
                let _ = logger.write_event(event_type, event_id, message);
            }
        }
    });
    
    if result.is_err() {
        tracing::debug!("Event log write panicked (non-critical)");
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