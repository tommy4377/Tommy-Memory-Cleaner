#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use tauri::AppHandle;

// Helper per convertire ICO in PNG ad alta risoluzione
#[cfg(windows)]
fn convert_ico_to_highres_png(ico_data: &[u8]) -> Result<Vec<u8>, String> {
    // Carica l'ICO usando image::load_from_memory che gestisce automaticamente il formato
    let img =
        image::load_from_memory(ico_data).map_err(|e| format!("Failed to load ICO: {}", e))?;

    // Converti in RGBA8
    let rgba_img = img.to_rgba8();

    // Resize a 256x256 (alta risoluzione per Windows Toast)
    let resized =
        image::imageops::resize(&rgba_img, 256, 256, image::imageops::FilterType::Lanczos3);

    // Codifica come PNG usando DynamicImage::save (API image 0.25)
    // Converti RgbaImage in DynamicImage per poter usare save
    let dynamic_img = image::DynamicImage::ImageRgba8(resized);

    // Salva in un buffer in memoria usando il metodo save_with_format
    let mut png_data = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut png_data);
        dynamic_img
            .write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| format!("Failed to encode PNG: {}", e))?;
    }

    Ok(png_data)
}

// Helper per ottenere il percorso dell'icona PNG ad alta risoluzione accessibile
// Windows Toast funziona meglio con PNG ad alta risoluzione (128x128 o più grande) invece di ICO
#[cfg(windows)]
fn ensure_notification_icon_available() -> Option<std::path::PathBuf> {
    use std::fs;

    // Prova prima a leggere PNG 128x128 dalla directory runtime (se distribuito con l'app)
    // Altrimenti usa ICO embedded e convertilo in PNG usando la libreria image
    let (icon_data, icon_ext) = {
        let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();

        // Prova a leggere PNG dalla directory runtime (se l'app è distribuita con le icone)
        if let Ok(png_data) = fs::read(exe_dir.join("icons").join("128x128.png")) {
            (png_data, "png")
        } else if let Ok(png_data) = fs::read(exe_dir.join("128x128.png")) {
            (png_data, "png")
        } else if let Ok(png_data) = fs::read(exe_dir.join("icons").join("icon.png")) {
            (png_data, "png")
        } else if let Ok(png_data) = fs::read(exe_dir.join("icon.png")) {
            (png_data, "png")
        } else {
            // Fallback: converti ICO embedded in PNG 256x256 ad alta risoluzione
            // Questo risolve il problema della sgranatura
            match convert_ico_to_highres_png(include_bytes!("../../icons/icon.ico")) {
                Ok(png_data) => {
                    tracing::debug!(
                        "Converted ICO to high-res PNG (256x256) for better notification quality"
                    );
                    (png_data, "png")
                }
                Err(e) => {
                    tracing::warn!("Failed to convert ICO to PNG, using ICO: {}", e);
                    (include_bytes!("../../icons/icon.ico").to_vec(), "ico")
                }
            }
        }
    };

    // Prova a salvare l'icona nella directory dati dell'app
    let icon_path = {
        let detector = crate::config::get_portable_detector();
        detector.data_dir().join(format!("icon.{}", icon_ext))
    };

    // Crea la directory se non esiste
    if let Some(parent) = icon_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            tracing::warn!("Failed to create icon directory: {}", e);
            return None;
        }
    }

    // Copia l'icona solo se non esiste o se è stata modificata
    // Controlla se il file esiste e ha la stessa dimensione
    let needs_copy = match fs::metadata(&icon_path) {
        Ok(meta) => meta.len() != icon_data.len() as u64,
        Err(_) => true, // File non esiste, devi copiarlo
    };

    if needs_copy {
        if let Err(e) = fs::write(&icon_path, &icon_data) {
            tracing::warn!("Failed to write notification icon: {}", e);
            return None;
        }
        tracing::debug!(
            "Notification icon (format: {}) copied to: {}",
            icon_ext,
            icon_path.display()
        );
    }

    Some(icon_path)
}

/// Show Windows notification with proper icon and theme
///
/// Attempt chain (ordered by efficiency):
/// 1. Tauri Plugin Notification (native Rust, zero overhead)
/// 2. winrt-notification crate (native Rust WinRT toast)
/// 3. PowerShell Balloon (last resort, spawns powershell.exe)
#[cfg(windows)]
pub fn show_windows_notification(
    app: &AppHandle,
    title: &str,
    body: &str,
    theme: &str,
) -> Result<(), String> {
    tracing::info!(
        "Attempting to show notification - Title: '{}', Body: '{}', Theme: {}",
        title,
        body,
        theme
    );

    // ── Attempt 1: Tauri plugin notification (native Rust, zero overhead) ──
    tracing::debug!("Attempt 1: Tauri plugin notification...");
    {
        let icon_path = ensure_notification_icon_available()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .or_else(|| {
                std::env::current_exe().ok().and_then(|exe_path| {
                    tracing::debug!("Using embedded icon from exe: {}", exe_path.display());
                    exe_path.to_str().map(|s| s.to_string())
                })
            })
            .unwrap_or_else(|| {
                tracing::warn!("Cannot get icon path, notification may fail");
                String::new()
            });

        if !icon_path.is_empty() {
            use tauri_plugin_notification::NotificationExt;
            match app
                .notification()
                .builder()
                .title(title)
                .body(body)
                .icon(icon_path)
                .show()
            {
                Ok(_) => {
                    tracing::info!("Notification sent via Tauri plugin (native, zero overhead)");
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("Tauri plugin notification failed: {}, trying fallback", e);
                }
            }
        } else {
            tracing::warn!("Icon path empty, skipping Tauri plugin notification");
        }
    }

    // ── Attempt 2: winrt-notification crate (native Rust WinRT toast) ──
    tracing::debug!("Attempt 2: winrt-notification crate...");
    {
        use winrt_notification::{IconCrop, Toast};

        let icon_path = ensure_notification_icon_available();

        let mut toast = Toast::new("TommyMemoryCleaner")
            .title(title)
            .text1(body);

        if let Some(ref path) = icon_path {
            toast = toast.icon(path, IconCrop::Circular, "Tommy Memory Cleaner");
        }

        match toast.show() {
            Ok(_) => {
                tracing::info!("Notification sent via winrt-notification (native WinRT)");
                return Ok(());
            }
            Err(e) => {
                tracing::warn!(
                    "winrt-notification failed: {}, trying fallback",
                    e
                );
            }
        }
    }

    // ── Attempt 3: PowerShell Balloon (last resort) ──
    tracing::debug!("Attempt 3: PowerShell balloon notification (last resort)...");
    {
        let title_clone = title.to_string();
        let body_clone = body.to_string();
        let ps_script = format!(
            r#"
try {{
    Add-Type -AssemblyName System.Windows.Forms -ErrorAction Stop
    $notification = New-Object System.Windows.Forms.NotifyIcon
    $notification.Icon = [System.Drawing.SystemIcons]::Information
    $notification.BalloonTipTitle = '{}'
    $notification.BalloonTipText = '{}'
    $notification.Visible = $true
    $notification.ShowBalloonTip(5000)
    Start-Sleep -Seconds 6
    $notification.Dispose()
    Write-Output "Notification shown successfully"
}} catch {{
    Write-Error "Failed to show notification: $_"
    exit 1
}}
"#,
            title_clone
                .replace("'", "''")
                .replace("\n", " ")
                .replace("\r", " "),
            body_clone
                .replace("'", "''")
                .replace("\n", " ")
                .replace("\r", " ")
        );

        match std::process::Command::new("powershell")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(&ps_script)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    tracing::info!("Notification sent via PowerShell balloon (last resort)");
                    return Ok(());
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    tracing::error!("PowerShell balloon notification failed: {}", error);
                }
            }
            Err(e) => {
                tracing::error!("Failed to execute PowerShell balloon notification: {}", e);
            }
        }
    }

    Err("All notification methods failed".to_string())
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
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::Registry::{RegSetValueExW, HKEY_CURRENT_USER, REG_SZ};

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
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegDeleteKeyW, RegOpenKeyExW, KEY_ALL_ACCESS,
        };
        // Prova prima ad aprire la chiave per verificare se esiste
        let key_path_wide: Vec<u16> = OsStr::new(key_path).encode_wide().chain(Some(0)).collect();
        let mut hkey_test: windows_sys::Win32::Foundation::HANDLE = std::ptr::null_mut();
        let open_result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            key_path_wide.as_ptr(),
            0,
            KEY_ALL_ACCESS,
            &mut hkey_test,
        );
        if open_result == 0 && hkey_test != std::ptr::null_mut() {
            RegCloseKey(hkey_test);
            // Elimina la chiave - potrebbe richiedere più tentativi
            let delete_result = RegDeleteKeyW(HKEY_CURRENT_USER, key_path_wide.as_ptr());
            if delete_result != 0 {
                tracing::debug!(
                    "Note: Could not delete existing registry key (may have subkeys): {}",
                    delete_result
                );
            } else {
                tracing::debug!("Deleted existing registry key for re-creation");
            }
        }
    }

    // Prova a usare un file .ico dedicato per migliori risultati con Windows Toast
    // Fallback all'exe se non riesce
    let icon_path = ensure_notification_icon_available()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| exe_path.clone());

    // Converti stringhe a wide strings
    let key_path_wide: Vec<u16> = OsStr::new(key_path).encode_wide().chain(Some(0)).collect();
    let display_name_wide: Vec<u16> = OsStr::new(display_name)
        .encode_wide()
        .chain(Some(0))
        .collect();

    unsafe {
        // Crea la chiave se non esiste e imposta i valori
        let mut hkey: windows_sys::Win32::Foundation::HANDLE = std::ptr::null_mut();
        let result = windows_sys::Win32::System::Registry::RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_path_wide.as_ptr(),
            0,
            std::ptr::null(),
            0,
            0x20006, // KEY_WRITE
            std::ptr::null(),
            &mut hkey,
            0 as *mut u32,
        );

        if result == 0 {
            // Imposta DisplayName
            let display_name_value: Vec<u16> = OsStr::new("DisplayName")
                .encode_wide()
                .chain(Some(0))
                .collect();
            RegSetValueExW(
                hkey,
                display_name_value.as_ptr(),
                0,
                REG_SZ,
                display_name_wide.as_ptr() as *const u8,
                (display_name_wide.len() * 2) as u32,
            );

            // Imposta IconUri
            let icon_uri_value: Vec<u16> =
                OsStr::new("IconUri").encode_wide().chain(Some(0)).collect();
            let icon_path_wide: Vec<u16> = OsStr::new(&icon_path)
                .encode_wide()
                .chain(Some(0))
                .collect();
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
