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

    // NUOVO APPROCCIO: Usa direttamente PowerShell con XML Toast template che include l'icona esplicitamente
    // Questo garantisce che l'icona venga mostrata correttamente
    #[cfg(windows)]
    {
        // Prova prima a usare un file .ico dedicato per migliori risultati
        let icon_path_opt = ensure_notification_icon_available();

        // Helper per fare URL encoding del percorso (necessario per spazi e caratteri speciali)
        let encode_uri = |path: &str| -> String {
            // Converti backslash a forward slash e poi applica percent-encoding
            let path_normalized = path.replace("\\", "/");
            // Per file:/// locali, dobbiamo fare percent-encoding solo dei caratteri speciali, non di tutto
            // Windows Toast accetta percorsi diretti, ma per sicurezza codifichiamo spazi e caratteri speciali
            let mut encoded = String::new();
            for ch in path_normalized.chars() {
                match ch {
                    ' ' => encoded.push_str("%20"),
                    '!' => encoded.push_str("%21"),
                    '#' => encoded.push_str("%23"),
                    '$' => encoded.push_str("%24"),
                    '%' => encoded.push_str("%25"),
                    '&' => encoded.push_str("%26"),
                    '\'' => encoded.push_str("%27"),
                    '(' => encoded.push_str("%28"),
                    ')' => encoded.push_str("%29"),
                    '*' => encoded.push_str("%2A"),
                    '+' => encoded.push_str("%2B"),
                    ',' => encoded.push_str("%2C"),
                    ':' => encoded.push_str("%3A"),
                    ';' => encoded.push_str("%3B"),
                    '=' => encoded.push_str("%3D"),
                    '?' => encoded.push_str("%3F"),
                    '@' => encoded.push_str("%40"),
                    '[' => encoded.push_str("%5B"),
                    ']' => encoded.push_str("%5D"),
                    _ => encoded.push(ch),
                }
            }
            format!("file:///{}", encoded)
        };

        let icon_uri = if let Some(icon_path) = icon_path_opt {
            // Usa il file .ico dedicato - converto il percorso in formato file:/// per Windows Toast
            let icon_path_str = icon_path.to_string_lossy().to_string();
            // Windows Toast richiede il formato file:/// con forward slashes e percent-encoding per spazi
            encode_uri(&icon_path_str)
        } else {
            // Fallback: usa l'exe stesso
            let exe_path = std::env::current_exe()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            encode_uri(&exe_path)
        };

        // Crea un XML Toast template personalizzato con l'icona
        let xml_template = format!(
            r#"<toast launch="app-defined-string" scenario="default">
<visual>
<binding template="ToastGeneric">
<text hint-maxLines="1">{}</text>
<text>{}</text>
<image placement="appLogoOverride" hint-crop="circle" src="{}"/>
</binding>
</visual>
<audio src="ms-winsoundevent:Notification.Default" />
</toast>"#,
            title, body, icon_uri
        );

        // Salva l'XML in un file temporaneo
        let temp_dir = std::env::temp_dir();
        let xml_path = temp_dir.join("tmc_notification.xml");
        if let Err(e) = std::fs::write(&xml_path, &xml_template) {
            tracing::warn!("Failed to write notification XML: {}", e);
        } else {
            // Esegui PowerShell per mostrare la notifica
            let app_id = "TommyMemoryCleaner";
            let ps_script = format!(
                r#"
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null

try {{
    $appId = '{}'
    $regPath = 'HKCU:\Software\Classes\AppUserModelId\' + $appId
    $displayName = 'Tommy Memory Cleaner'
    
    # Forza la registrazione del DisplayName prima di ogni notifica
    # Questo assicura che Windows usi il nome corretto anche se la cache è stata invalidata
    if (-not (Test-Path $regPath)) {{
        New-Item -Path $regPath -Force | Out-Null
    }}
    Set-ItemProperty -Path $regPath -Name DisplayName -Value $displayName -Type String -Force | Out-Null
    Write-Output "DisplayName forced to: $displayName"
    
    # Carica e mostra la notifica
    $xml = New-Object Windows.Data.Xml.Dom.XmlDocument
    $xml.LoadXml([System.IO.File]::ReadAllText('{}'))
    
    $toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
    
    # Crea il notifier - Windows dovrebbe usare automaticamente il DisplayName se registrato
    $notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier($appId)
    $notifier.Show($toast)
    
    Write-Output "Toast notification shown successfully with DisplayName: $displayName"
}} catch {{
    Write-Error "Failed to show toast: $_"
    exit 1
}}
"#,
                app_id,
                xml_path.to_string_lossy().replace("'", "''")
            );

            match std::process::Command::new("powershell")
                .arg("-NoProfile")
                .arg("-NonInteractive")
                .arg("-ExecutionPolicy")
                .arg("Bypass")
                .arg("-Command")
                .arg(&ps_script)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output()
            {
                Ok(output) => {
                    // Pulisci file temporaneo
                    let _ = std::fs::remove_file(&xml_path);
                    if output.status.success() {
                        tracing::info!(
                            "✓ Windows Toast notification shown successfully with icon: {}",
                            icon_uri
                        );
                        return Ok(());
                    } else {
                        let error = String::from_utf8_lossy(&output.stderr);
                        tracing::warn!(
                            "✗ PowerShell Toast notification failed: {}, trying fallback",
                            error
                        );
                    }
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&xml_path);
                    tracing::warn!(
                        "✗ Failed to execute PowerShell Toast notification: {}, trying fallback",
                        e
                    );
                }
            }
        }
    }

    // Fallback: Usa Tauri API notification
    tracing::debug!("Trying Tauri API notification as fallback...");
    #[cfg(windows)]
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

    #[cfg(not(windows))]
    let icon_path = String::new();

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
                tracing::info!("✓ Tauri API notification shown successfully");
                return Ok(());
            }
            Err(e) => {
                tracing::warn!("✗ Tauri API notification failed: {}", e);
            }
        }
    }

    // Ultimo fallback: PowerShell Balloon
    #[cfg(windows)]
    {
        tracing::debug!("Trying PowerShell balloon notification as last fallback...");
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
            .creation_flags(0x08000000)
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    tracing::info!("✓ PowerShell balloon notification shown successfully");
                    return Ok(());
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    tracing::error!("✗ PowerShell notification failed: {}", error);
                }
            }
            Err(e) => {
                tracing::error!("✗ Failed to execute PowerShell notification: {}", e);
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
        let mut hkey: windows_sys::Win32::Foundation::HANDLE = 0;
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
