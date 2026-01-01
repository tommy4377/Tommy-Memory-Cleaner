#[cfg(windows)]
use tauri::AppHandle;
#[cfg(windows)]

/// Show Windows notification with proper icon and theme
#[cfg(windows)]
pub fn show_windows_notification(
    app: &AppHandle,
    title: &str,
    body: &str,
    theme: &str,
) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;
    
    // Get icon path
    let icon_path = match crate::notifications::helpers::ensure_notification_icon_available() {
        Some(path) => path,
        None => {
            tracing::warn!("No notification icon available");
            return Ok(());
        }
    };
    
    // Determine icon based on theme
    let final_icon_path = if theme == "dark" {
        // For dark theme, we might want a different icon
        icon_path
    } else {
        icon_path
    };
    
    // Send notification
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .icon(final_icon_path.to_string_lossy().to_string())
        .show()
        .map_err(|e| format!("Failed to show notification: {}", e))?;
    
    Ok(())
}

#[cfg(not(windows))]
pub fn show_windows_notification(
    _app: &AppHandle,
    _title: &str,
    _body: &str,
    _theme: &str,
) -> Result<(), String> {
    Ok(())
}

/// Register the app for Windows Toast notifications
#[cfg(windows)]
pub fn register_app_for_notifications() {
    use windows_sys::Win32::System::Registry::{RegSetValueExW, HKEY_CURRENT_USER, REG_SZ};
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    
    let _app_id = "TommyMemoryCleaner";
    // Usa to_string_lossy() per gestire correttamente i percorsi con caratteri Unicode
    let exe_path = std::env::current_exe()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    
    if exe_path.is_empty() {
        tracing::warn!("Cannot register app for notifications: exe path not found");
        return;
    }
    
    // Registra AppUserModelID nel registro con DisplayName e IconUri
    // IMPORTANTE: Windows richiede che questa registrazione avvenga PRIMA di qualsiasi notifica
    // USIAMO "TommyMemoryCleaner" come AppUserModelID per mostrare un nome user-friendly nelle notifiche
    let key_path = r"Software\Classes\AppUserModelId\TommyMemoryCleaner";
    let display_name = "Tommy Memory Cleaner";
    
    // Elimina ricorsivamente la chiave esistente per forzare la ricreazione (utile se è stata modificata)
    // Usa SHDeleteKey per eliminare anche le sottocartelle
    unsafe {
        use windows_sys::Win32::System::Registry::{RegDeleteKeyW, RegOpenKeyExW, RegCloseKey, KEY_ALL_ACCESS};
        // Prova prima ad aprire la chiave per verificare se esiste
        let key_path_wide: Vec<u16> = OsStr::new(key_path).encode_wide().chain(Some(0)).collect();
        let mut hkey_test: windows_sys::Win32::Foundation::HANDLE = 0;
        let open_result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            key_path_wide.as_ptr(),
            0,
            KEY_ALL_ACCESS,
            &mut hkey_test,
        );
        if open_result == 0 && hkey_test != 0 {
            RegCloseKey(hkey_test);
            // Elimina la chiave - potrebbe richiedere più tentativi
            let delete_result = RegDeleteKeyW(HKEY_CURRENT_USER, key_path_wide.as_ptr());
            if delete_result != 0 {
                tracing::debug!("Note: Could not delete existing registry key (may have subkeys): {}", delete_result);
            } else {
                tracing::debug!("Deleted existing registry key for re-creation");
            }
        }
    }
    
    // Prova a usare un file .ico dedicato per migliori risultati con Windows Toast
    // Fallback all'exe se non riesce
    let icon_path = crate::notifications::helpers::ensure_notification_icon_available()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| exe_path.clone());
    
    // Converti stringhe a wide strings
    let key_path_wide: Vec<u16> = OsStr::new(key_path).encode_wide().chain(Some(0)).collect();
    let display_name_wide: Vec<u16> = OsStr::new(display_name).encode_wide().chain(Some(0)).collect();
    
    unsafe {
        // Crea la chiave se non esiste e imposta i valori
        let mut hkey: windows_sys::Win32::Foundation::HANDLE = 0;
        let result = windows_sys::Win32::System::Registry::RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_path_wide.as_ptr(),
            0,
            std::ptr::null(),
            0,
            0x20006, // KEY_WRITE
            std::ptr::null_mut(),
            &mut hkey,
            std::ptr::null_mut(),
        );
        
        if result == 0 {
            // Imposta DisplayName
            let display_name_value: Vec<u16> = OsStr::new("DisplayName").encode_wide().chain(Some(0)).collect();
            RegSetValueExW(
                hkey,
                display_name_value.as_ptr(),
                0,
                REG_SZ,
                display_name_wide.as_ptr() as *const u8,
                (display_name_wide.len() * 2) as u32,
            );
            
            // Imposta IconUri
            let icon_uri_value: Vec<u16> = OsStr::new("IconUri").encode_wide().chain(Some(0)).collect();
            let icon_path_wide: Vec<u16> = OsStr::new(&icon_path).encode_wide().chain(Some(0)).collect();
            RegSetValueExW(
                hkey,
                icon_uri_value.as_ptr(),
                0,
                REG_SZ,
                icon_path_wide.as_ptr() as *const u8,
                (icon_path_wide.len() * 2) as u32,
            );
            
            windows_sys::Win32::System::Registry::RegCloseKey(hkey);
            tracing::info!("App registered for Windows notifications: {}", display_name);
        } else {
            tracing::error!("Failed to register app for notifications: 0x{:08X}", result);
        }
    }
}

#[cfg(not(windows))]
pub fn register_app_for_notifications() {
    // No-op on non-Windows platforms
}