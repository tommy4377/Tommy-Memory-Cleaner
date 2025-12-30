#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod config;
mod engine;
mod logging;
mod memory;
mod utils;
mod os;
mod ui;
mod system;
mod antivirus {
    pub mod whitelist;
}

use crate::config::{Config, Priority, Profile};
use crate::engine::Engine;
use crate::memory::types::{Areas, Reason};
use crate::ui::bridge::{emit_progress, EV_DONE};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{Manager, AppHandle, Emitter};
use tauri::webview::WebviewWindowBuilder;
use tauri::WebviewUrl;
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, GlobalShortcutExt};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_positioner::{WindowExt, Position};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use scopeguard;
use std::sync::Mutex as StdMutex;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

// ============= GLOBAL STATE =============
static HOTKEY_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static OPTIMIZATION_RUNNING: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
static PRIVILEGES_INITIALIZED: Lazy<RwLock<bool>> = Lazy::new(|| RwLock::new(false));
static FIRST_OPTIMIZATION_DONE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
// Salva l'ID del tray icon per usarlo in seguito
pub(crate) static TRAY_ICON_ID: Lazy<StdMutex<Option<String>>> = Lazy::new(|| StdMutex::new(None));

#[derive(Clone)]
struct AppState { 
    cfg: Arc<Mutex<Config>>, 
    engine: Engine 
}

// ============= TRANSLATIONS =============
fn t(lang: &str, key: &str) -> String {
    match (lang, key) {
        // Italiano
        ("it", "Open TMC") => "Apri TMC",
        ("it", "Optimize Memory") => "Ottimizza Memoria",
        ("it", "Exit") => "Esci",
        ("it", "TMC • Optimization completed") => "TMC • Ottimizzazione completata",
        ("it", "TMC • Scheduled optimization") => "TMC • Ottimizzazione programmata",
        ("it", "TMC • Low memory optimization") => "TMC • Ottimizzazione per memoria bassa",
        ("it", "Normal") => "Normale",
        ("it", "Balanced") => "Bilanciato",
        ("it", "Gaming") => "Gaming",
        
        // Spagnolo
        ("es", "Open TMC") => "Abrir TMC",
        ("es", "Optimize Memory") => "Optimizar Memoria",
        ("es", "Exit") => "Salir",
        ("es", "TMC • Optimization completed") => "TMC • Optimización completada",
        ("es", "TMC • Scheduled optimization") => "TMC • Optimización programada",
        ("es", "TMC • Low memory optimization") => "TMC • Optimización por memoria baja",
        ("es", "Normal") => "Normal",
        ("es", "Balanced") => "Equilibrado",
        ("es", "Gaming") => "Gaming",
        
        // Francese
        ("fr", "Open TMC") => "Ouvrir TMC",
        ("fr", "Optimize Memory") => "Optimiser la Mémoire",
        ("fr", "Exit") => "Quitter",
        ("fr", "TMC • Optimization completed") => "TMC • Optimisation terminée",
        ("fr", "TMC • Scheduled optimization") => "TMC • Optimisation programmée",
        ("fr", "TMC • Low memory optimization") => "TMC • Optimisation mémoire faible",
        ("fr", "Normal") => "Normal",
        ("fr", "Balanced") => "Équilibré",
        ("fr", "Gaming") => "Gaming",
        
        // Portoghese
        ("pt", "Open TMC") => "Abrir TMC",
        ("pt", "Optimize Memory") => "Otimizar Memória",
        ("pt", "Exit") => "Sair",
        ("pt", "TMC • Optimization completed") => "TMC • Otimização concluída",
        ("pt", "TMC • Scheduled optimization") => "TMC • Otimização agendada",
        ("pt", "TMC • Low memory optimization") => "TMC • Otimização por memória baixa",
        ("pt", "Normal") => "Normal",
        ("pt", "Balanced") => "Balanceado",
        ("pt", "Gaming") => "Jogos",
        
        // Tedesco
        ("de", "Open TMC") => "TMC Öffnen",
        ("de", "Optimize Memory") => "Speicher Optimieren",
        ("de", "Exit") => "Beenden",
        ("de", "TMC • Optimization completed") => "TMC • Optimierung abgeschlossen",
        ("de", "TMC • Scheduled optimization") => "TMC • Geplante Optimierung",
        ("de", "TMC • Low memory optimization") => "TMC • Optimierung bei wenig Speicher",
        ("de", "Normal") => "Normal",
        ("de", "Balanced") => "Ausgeglichen",
        ("de", "Gaming") => "Spielen",
        
        // Arabo
        ("ar", "Open TMC") => "فتح TMC",
        ("ar", "Optimize Memory") => "تحسين الذاكرة",
        ("ar", "Exit") => "خروج",
        ("ar", "TMC • Optimization completed") => "TMC • اكتمل التحسين",
        ("ar", "TMC • Scheduled optimization") => "TMC • تحسين مجدول",
        ("ar", "TMC • Low memory optimization") => "TMC • تحسين الذاكرة المنخفضة",
        ("ar", "Normal") => "عادي",
        ("ar", "Balanced") => "متوازن",
        ("ar", "Gaming") => "الألعاب",
        
        // Giapponese
        ("ja", "Open TMC") => "TMCを開く",
        ("ja", "Optimize Memory") => "メモリを最適化",
        ("ja", "Exit") => "終了",
        ("ja", "TMC • Optimization completed") => "TMC • 最適化完了",
        ("ja", "TMC • Scheduled optimization") => "TMC • スケジュール最適化",
        ("ja", "TMC • Low memory optimization") => "TMC • メモリ不足最適化",
        ("ja", "Normal") => "ノーマル",
        ("ja", "Balanced") => "バランス",
        ("ja", "Gaming") => "ゲーミング",
        
        // Cinese
        ("zh", "Open TMC") => "打开TMC",
        ("zh", "Optimize Memory") => "优化内存",
        ("zh", "Exit") => "退出",
        ("zh", "TMC • Optimization completed") => "TMC • 优化完成",
        ("zh", "TMC • Scheduled optimization") => "TMC • 计划优化",
        ("zh", "TMC • Low memory optimization") => "TMC • 低内存优化",
        ("zh", "Normal") => "普通",
        ("zh", "Balanced") => "平衡",
        ("zh", "Gaming") => "游戏",
        
        // Default inglese
        (_, "Open TMC") => "Open TMC",
        (_, "Optimize Memory") => "Optimize Memory",
        (_, "Exit") => "Exit",
        (_, "TMC • Optimization completed") => "TMC • Optimization completed",
        (_, "TMC • Scheduled optimization") => "TMC • Scheduled optimization",
        (_, "TMC • Low memory optimization") => "TMC • Low memory optimization",
        (_, "Normal") => "Normal",
        (_, "Balanced") => "Balanced",
        (_, "Gaming") => "Gaming",
        _ => key,
    }.to_string()
}

// ============= WINDOWS HELPERS =============
#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

// ============= PRIVILEGE MANAGEMENT =============
fn ensure_privileges_initialized() -> Result<(), String> {
    // Check se già inizializzato
    if *PRIVILEGES_INITIALIZED.read() {
        return Ok(());
    }
    
    // Lock per scrittura e ri-controlla
    let mut guard = PRIVILEGES_INITIALIZED.write();
    if *guard {
        return Ok(());
    }
    
    tracing::info!("Initializing Windows privileges...");
    
    // Lista di tutti i privilegi necessari
    let privileges = [
        "SeDebugPrivilege",              // Per ottimizzare working set di altri processi
        "SeIncreaseQuotaPrivilege",      // Per modificare cache di sistema
        "SeProfileSingleProcessPrivilege", // Per operazioni avanzate di memoria
    ];
    
    let mut success_count = 0;
    for priv_name in &privileges {
        match crate::memory::privileges::ensure_privilege(priv_name) {
            Ok(_) => {
                tracing::info!("✓ Acquired privilege: {}", priv_name);
                success_count += 1;
            }
            Err(e) => {
                tracing::warn!("✗ Failed to acquire {}: {}", priv_name, e);
            }
        }
    }
    
    tracing::info!("Privileges initialized: {}/{} acquired", success_count, privileges.len());
    *guard = true;
    Ok(())
}

// ============= NOTIFICATIONS =============
// Helper per convertire ICO in PNG ad alta risoluzione
#[cfg(windows)]
fn convert_ico_to_highres_png(ico_data: &[u8]) -> Result<Vec<u8>, String> {
    // Carica l'ICO usando image::load_from_memory che gestisce automaticamente il formato
    let img = image::load_from_memory(ico_data)
        .map_err(|e| format!("Failed to load ICO: {}", e))?;
    
    // Converti in RGBA8
    let rgba_img = img.to_rgba8();
    
    // Resize a 256x256 (alta risoluzione per Windows Toast)
    let resized = image::imageops::resize(
        &rgba_img,
        256,
        256,
        image::imageops::FilterType::Lanczos3,
    );
    
    // Codifica come PNG usando DynamicImage::save (API image 0.25)
    // Converti RgbaImage in DynamicImage per poter usare save
    let dynamic_img = image::DynamicImage::ImageRgba8(resized);
    
    // Salva in un buffer in memoria usando il metodo save_with_format
    let mut png_data = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut png_data);
        dynamic_img.write_to(&mut cursor, image::ImageFormat::Png)
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
            match convert_ico_to_highres_png(include_bytes!("../icons/icon.ico")) {
                Ok(png_data) => {
                    tracing::debug!("Converted ICO to high-res PNG (256x256) for better notification quality");
                    (png_data, "png")
                },
                Err(e) => {
                    tracing::warn!("Failed to convert ICO to PNG, using ICO: {}", e);
                    (include_bytes!("../icons/icon.ico").to_vec(), "ico")
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
        tracing::debug!("Notification icon (format: {}) copied to: {}", icon_ext, icon_path.display());
    }
    
    Some(icon_path)
}

// Registra l'app per Windows Toast notifications (richiesto per applicazioni non confezionate)
#[cfg(windows)]
fn register_app_for_notifications() {
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
        let mut hkey_test: *mut std::ffi::c_void = std::ptr::null_mut();
        let open_result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            key_path_wide.as_ptr(),
            0,
            KEY_ALL_ACCESS,
            &mut hkey_test,
        );
        if open_result == 0 && !hkey_test.is_null() {
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
    let icon_path = ensure_notification_icon_available()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| exe_path.clone());
    
    // Converti stringhe a wide strings
    let key_path_wide: Vec<u16> = OsStr::new(key_path).encode_wide().chain(Some(0)).collect();
    let display_name_wide: Vec<u16> = OsStr::new(display_name).encode_wide().chain(Some(0)).collect();
    
    unsafe {
        // Crea la chiave se non esiste e imposta i valori
        let mut hkey: *mut std::ffi::c_void = std::ptr::null_mut();
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
            
            // Imposta IconUri se disponibile - Windows richiede il percorso completo dell'exe
            // L'icona embedded nell'exe verrà usata automaticamente
            if !icon_path.is_empty() {
                let icon_uri_value: Vec<u16> = OsStr::new("IconUri").encode_wide().chain(Some(0)).collect();
                // Windows Toast usa il percorso diretto dell'exe per l'icona embedded
                // Non serve file://, solo il percorso nativo Windows
                let icon_uri_wide: Vec<u16> = OsStr::new(&icon_path).encode_wide().chain(Some(0)).collect();
                RegSetValueExW(
                    hkey,
                    icon_uri_value.as_ptr(),
                    0,
                    REG_SZ,
                    icon_uri_wide.as_ptr() as *const u8,
                    (icon_uri_wide.len() * 2) as u32, // REG_SZ include il null terminator
                );
                tracing::debug!("Set IconUri to: {}", icon_path);
            }
            
            windows_sys::Win32::System::Registry::RegCloseKey(hkey);
            tracing::info!("App registered for Windows Toast notifications - DisplayName: '{}', IconUri: '{}'", display_name, icon_path);
            eprintln!("[TMC] App registered - DisplayName: '{}', IconUri: '{}'", display_name, icon_path);
            
            // Forza un refresh della cache di Windows Toast usando PowerShell
            // Questo aiuta a far riconoscere il DisplayName anche se era già cachato
            let refresh_script = format!(
                r#"
                try {{
                    $regPath = 'HKCU:\Software\Classes\AppUserModelId\{}'
                    $displayName = '{}'
                    
                    # Forza la scrittura del DisplayName
                    Set-ItemProperty -Path $regPath -Name DisplayName -Value $displayName -Type String -Force | Out-Null
                    
                    # Prova a forzare un refresh della cache Toast
                    # Riavvia il servizio WpnUserService se possibile (richiede admin)
                    try {{
                        Restart-Service -Name 'WpnUserService' -Force -ErrorAction SilentlyContinue
                    }} catch {{
                        # Non critico se non riusciamo a riavviare il servizio
                    }}
                    
                    Write-Output "DisplayName refreshed: $displayName"
                }} catch {{
                    Write-Warning "Could not refresh DisplayName: $_"
                }}
                "#,
                "TommyMemoryCleaner",
                display_name
            );
            
            // Esegui lo script di refresh in background (non bloccare se fallisce)
            let _ = std::process::Command::new("powershell")
                .arg("-NoProfile")
                .arg("-NonInteractive")
                .arg("-ExecutionPolicy")
                .arg("Bypass")
                .arg("-Command")
                .arg(&refresh_script)
                .creation_flags(0x08000000)
                .spawn();
        } else {
            tracing::warn!("Failed to register app for notifications: error {}", result);
        }
    }
}

fn show_windows_notification(app: &tauri::AppHandle, title: &str, body: &str, theme: &str) -> Result<(), String> {
    tracing::info!("Attempting to show notification - Title: '{}', Body: '{}', Theme: {}", title, body, theme);
    
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
            
            if !exe_path.is_empty() {
                encode_uri(&exe_path)
            } else {
                tracing::warn!("Cannot get icon path for notification");
                String::new()
            }
        };
        
        // Escape XML special characters per testo
        let title_escaped = title
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("\"", "&quot;")
            .replace("'", "&apos;");
        let body_escaped = body
            .replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("\"", "&quot;")
            .replace("'", "&apos;");
        
        // Escape XML per attributo src (escape solo & e ")
        let icon_uri_escaped = icon_uri
            .replace("&", "&amp;")
            .replace("\"", "&quot;");
        
        // XML Toast template con icona esplicita
        // Usa hint-align="left" per allineare il testo a sinistra
        // Aggiungi anche hint-style="title" per il primo testo per migliorare l'allineamento
        let toast_xml = if !icon_uri.is_empty() {
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<toast>
    <visual>
        <binding template="ToastGeneric">
            <image placement="appLogoOverride" hint-crop="circle" src="{}" />
            <text hint-align="left" hint-style="title">{}</text>
            <text hint-align="left">{}</text>
        </binding>
    </visual>
</toast>"#,
                icon_uri_escaped, title_escaped, body_escaped
            )
        } else {
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<toast>
    <visual>
        <binding template="ToastGeneric">
            <text>{}</text>
            <text>{}</text>
        </binding>
    </visual>
</toast>"#,
                title_escaped, body_escaped
            )
        };
        
        // Salva XML in file temporaneo
        let temp_xml = std::env::temp_dir().join(format!("tmc_toast_{}.xml", std::process::id()));
        if let Err(e) = std::fs::write(&temp_xml, toast_xml) {
            tracing::warn!("Failed to write toast XML: {}, trying fallback", e);
        } else {
            // Usa PowerShell per inviare la notifica Toast con XML
            let xml_path = temp_xml.to_string_lossy().replace("\\", "\\\\");
            let app_id = "TommyMemoryCleaner";
            
            // PowerShell script che forza l'uso del DisplayName dal registro
            // IMPORTANTE: Windows Toast usa l'AppUserModelID per identificare l'app,
            // ma il DisplayName viene mostrato solo se registrato correttamente PRIMA della prima notifica
            // Forziamo la registrazione prima di ogni notifica per assicurarci che sia aggiornata
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
                xml_path.replace("'", "''")
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
                    let _ = std::fs::remove_file(&temp_xml);
                    
                    if output.status.success() {
                        tracing::info!("✓ Windows Toast notification shown successfully with icon: {}", icon_uri);
                        return Ok(());
                    } else {
                        let error = String::from_utf8_lossy(&output.stderr);
                        tracing::warn!("✗ PowerShell Toast notification failed: {}, trying fallback", error);
                    }
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&temp_xml);
                    tracing::warn!("✗ Failed to execute PowerShell Toast notification: {}, trying fallback", e);
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
            std::env::current_exe()
                .ok()
                .and_then(|exe_path| {
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
        match app.notification()
            .builder()
            .title(title)
            .body(body)
            .icon(&icon_path)
            .show()
        {
            Ok(_) => {
                tracing::info!("✓ Notification shown via Tauri API with icon '{}'", icon_path);
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
            title_clone.replace("'", "''").replace("`n", " ").replace("`r", " "),
            body_clone.replace("'", "''").replace("`n", " ").replace("`r", " ")
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

// ============= NOTIFICATION HELPERS =============
fn get_notification_title(language: &str, reason: Reason) -> String {
    match reason {
        Reason::Manual => t(language, "TMC • Optimization completed"),
        Reason::Schedule => t(language, "TMC • Scheduled optimization"),
        Reason::LowMemory => t(language, "TMC • Low memory optimization"),
    }
}

fn get_profile_display_name(profile: &Profile, language: &str) -> String {
    match profile {
        Profile::Normal => t(language, "Normal"),
        Profile::Balanced => t(language, "Balanced"),
        Profile::Gaming => t(language, "Gaming"),
    }
}

fn get_notification_body(language: &str, _reason: Reason, freed_mb: f64, free_gb: f64, profile: &Profile) -> String {
    let profile_name = get_profile_display_name(profile, language);
    
    // Formatta in base alla lingua
    match language {
        "it" => format!(
            "✅ Liberati: {:.1} MB\n🧠 RAM libera: {:.2} GB\n🎯 Profilo: {}",
            freed_mb.abs(), free_gb, profile_name
        ),
        "es" => format!(
            "✅ Liberado: {:.1} MB\n🧠 RAM libre: {:.2} GB\n🎯 Perfil: {}",
            freed_mb.abs(), free_gb, profile_name
        ),
        "fr" => format!(
            "✅ Libéré: {:.1} MB\n🧠 RAM libre: {:.2} GB\n🎯 Profil: {}",
            freed_mb.abs(), free_gb, profile_name
        ),
        "pt" => format!(
            "✅ Liberado: {:.1} MB\n🧠 RAM livre: {:.2} GB\n🎯 Perfil: {}",
            freed_mb.abs(), free_gb, profile_name
        ),
        "de" => format!(
            "✅ Freigegeben: {:.1} MB\n🧠 Freier RAM: {:.2} GB\n🎯 Profil: {}",
            freed_mb.abs(), free_gb, profile_name
        ),
        "ar" => format!(
            "✅ تم التحرير: {:.1} ميجابايت\n🧠 ذاكرة متاحة: {:.2} جيجابايت\n🎯 الملف الشخصي: {}",
            freed_mb.abs(), free_gb, profile_name
        ),
        "ja" => format!(
            "✅ 解放: {:.1} MB\n🧠 空きRAM: {:.2} GB\n🎯 プロファイル: {}",
            freed_mb.abs(), free_gb, profile_name
        ),
        "zh" => format!(
            "✅ 已释放: {:.1} MB\n🧠 可用RAM: {:.2} GB\n🎯 配置文件: {}",
            freed_mb.abs(), free_gb, profile_name
        ),
        _ => format!(
            "✅ Freed: {:.1} MB\n🧠 Free RAM: {:.2} GB\n🎯 Profile: {}",
            freed_mb.abs(), free_gb, profile_name
        )
    }
}

// ============= TRAY MENU (Tauri v2) =============
// Il menu tray è gestito direttamente nel builder, vedi ui::tray::build()

fn refresh_tray_icon(app: &AppHandle) {
    let state = app.state::<AppState>();
    
    // FIX #2 & #3: Usa try_lock per evitare deadlock e acquisisci tutte le info necessarie
    let (_show_mem_usage, mem_percent) = {
        // Prova ad acquisire il lock senza bloccare
        match state.cfg.try_lock() {
            Ok(c) => {
                let show_mem = c.tray.show_mem_usage;
                // Rilascia il lock PRIMA di chiamare engine.memory() per evitare deadlock
                drop(c);
                
                if !show_mem {
                    tracing::debug!("refresh_tray_icon: show_mem_usage is false, will load default icon");
                    (false, 0)
                } else {
                    // Ora che il lock è rilasciato, possiamo chiamare memory() in sicurezza
                    let mem_percent = state.engine.memory()
                        .map(|mem| {
                            // Clamp percentage tra 0-100 (dovrebbe essere già nel range, ma per sicurezza)
                            mem.physical.used.percentage.min(100)
                        })
                        .unwrap_or_else(|e| {
                            tracing::warn!("Failed to get memory info: {}, using 0", e);
                            0
                        });
                    (true, mem_percent)
                }
            },
            Err(_) => {
                tracing::debug!("Config lock busy, skipping tray icon update");
                (false, 0)
            }
        }
    };
    
    // Se show_mem_usage è false, update_tray_icon userà l'icona di default
    crate::ui::tray::update_tray_icon(app, mem_percent);
}


// ============= AREA PARSING =============
fn parse_areas_string(areas_str: &str) -> Areas {
    let mut result = Areas::empty();
    for flag in areas_str.split('|') {
        match flag.trim() {
            "COMBINED_PAGE_LIST" => result |= Areas::COMBINED_PAGE_LIST,
            "MODIFIED_FILE_CACHE" => result |= Areas::MODIFIED_FILE_CACHE,
            "MODIFIED_PAGE_LIST" => result |= Areas::MODIFIED_PAGE_LIST,
            "REGISTRY_CACHE" => result |= Areas::REGISTRY_CACHE,
            "STANDBY_LIST" => result |= Areas::STANDBY_LIST,
            "STANDBY_LIST_LOW" => result |= Areas::STANDBY_LIST_LOW,
            "SYSTEM_FILE_CACHE" => result |= Areas::SYSTEM_FILE_CACHE,
            "WORKING_SET" => result |= Areas::WORKING_SET,
            "" => {}, // Ignora stringhe vuote
            unknown => {
                tracing::warn!("Unknown memory area flag: '{}' in areas string: '{}'", unknown, areas_str);
            }
        }
    }
    result
}

// ============= HOTKEY MANAGEMENT =============
fn normalize_hotkey(hotkey: &str) -> Result<String, String> {
    let parts: Vec<&str> = hotkey.split('+').map(|s| s.trim()).collect();
    
    if parts.is_empty() {
        return Err("Invalid hotkey format".to_string());
    }
    
    // Valida duplicati
    let mut seen = std::collections::HashSet::<String>::new();
    for part in &parts {
        let upper = part.to_uppercase();
        let normalized_part = match upper.as_str() {
            "CTRL" | "CONTROL" | "COMMANDORCONTROL" => "CTRL".to_string(),
            "ALT" => "ALT".to_string(),
            "SHIFT" => "SHIFT".to_string(),
            key if key.len() == 1 && key.chars().all(|c| c.is_ascii_alphanumeric()) => key.to_string(),
            key if key.starts_with('F') && key.len() <= 3 => {
                if let Ok(num) = key[1..].parse::<u32>() {
                    if (1..=12).contains(&num) {
                        key.to_string()
                    } else {
                        return Err(format!("Invalid function key: {}", part));
                    }
                } else {
                    return Err(format!("Invalid key: {}", part));
                }
            }
            _ => return Err(format!("Invalid key: {}", part)),
        };
        
        if !seen.insert(normalized_part) {
            return Err(format!("Duplicate modifier/key: {}", part));
        }
    }
    
    let mut normalized = Vec::new();
    
    for part in parts {
        let upper = part.to_uppercase();
        match upper.as_str() {
            "CTRL" | "CONTROL" | "COMMANDORCONTROL" => normalized.push("Ctrl".to_string()),
            "ALT" => normalized.push("Alt".to_string()),
            "SHIFT" => normalized.push("Shift".to_string()),
            key if key.len() == 1 && key.chars().all(|c| c.is_ascii_alphanumeric()) => {
                normalized.push(upper);
            }
            key if key.starts_with('F') && key.len() <= 3 => {
                if let Ok(num) = key[1..].parse::<u32>() {
                    if (1..=12).contains(&num) {
                        normalized.push(format!("F{}", num));
                    } else {
                        return Err(format!("Invalid function key: {}", part));
                    }
                } else {
                    return Err(format!("Invalid key: {}", part));
                }
            }
            _ => return Err(format!("Invalid key: {}", part)),
        }
    }
    
    Ok(normalized.join("+"))
}

fn register_global_hotkey_v2(app: &AppHandle, hotkey: &str, state: AppState) -> Result<(), String> {
    let _guard = HOTKEY_MUTEX.lock()
        .map_err(|e| format!("Failed to acquire hotkey mutex: {}", e))?;
    
    // Tauri v2: usa il plugin global-shortcut
    // Unregister all
    app.global_shortcut().unregister_all().map_err(|e| format!("Failed to unregister hotkeys: {}", e))?;
    
    if hotkey.trim().is_empty() {
        return Ok(());
    }
    
    let normalized = normalize_hotkey(hotkey)?;
    
    // Parse hotkey per Tauri v2
    let (modifiers, key) = parse_hotkey_for_v2(&normalized)?;
    
    let engine = state.engine.clone();
    let cfg_clone = state.cfg.clone();
    let app_handle = app.clone();
    
    let shortcut = Shortcut::new(Some(modifiers), code_from_str(&key)?);
    
    app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, _event| {
        let engine = engine.clone();
        let cfg = cfg_clone.clone();
        let app = app_handle.clone();
        
        tauri::async_runtime::spawn(async move {
            perform_optimization(app, engine, cfg, Reason::Manual, true, None).await;
        });
    }).map_err(|e| format!("Failed to register hotkey '{}': {}", normalized, e))?;
    
    Ok(())
}

fn parse_hotkey_for_v2(hotkey: &str) -> Result<(Modifiers, String), String> {
    let parts: Vec<&str> = hotkey.split('+').map(|s| s.trim()).collect();
    let mut mods = Modifiers::empty();
    let mut key = String::new();
    
    for part in parts {
        match part.to_uppercase().as_str() {
            "CTRL" | "CONTROL" => mods |= Modifiers::CONTROL,
            "ALT" => mods |= Modifiers::ALT,
            "SHIFT" => mods |= Modifiers::SHIFT,
            "SUPER" | "WIN" | "META" => mods |= Modifiers::SUPER,
            _ => key = part.to_uppercase(),
        }
    }
    
    if key.is_empty() {
        return Err("No key specified in hotkey".to_string());
    }
    
    Ok((mods, key))
}

fn code_from_str(s: &str) -> Result<Code, String> {
    match s.to_uppercase().as_str() {
            "A" => Ok(Code::KeyA),
            "B" => Ok(Code::KeyB),
            "C" => Ok(Code::KeyC),
            "D" => Ok(Code::KeyD),
            "E" => Ok(Code::KeyE),
            "F" => Ok(Code::KeyF),
            "G" => Ok(Code::KeyG),
            "H" => Ok(Code::KeyH),
            "I" => Ok(Code::KeyI),
            "J" => Ok(Code::KeyJ),
            "K" => Ok(Code::KeyK),
            "L" => Ok(Code::KeyL),
            "M" => Ok(Code::KeyM),
            "N" => Ok(Code::KeyN),
            "O" => Ok(Code::KeyO),
            "P" => Ok(Code::KeyP),
            "Q" => Ok(Code::KeyQ),
            "R" => Ok(Code::KeyR),
            "S" => Ok(Code::KeyS),
            "T" => Ok(Code::KeyT),
            "U" => Ok(Code::KeyU),
            "V" => Ok(Code::KeyV),
            "W" => Ok(Code::KeyW),
            "X" => Ok(Code::KeyX),
            "Y" => Ok(Code::KeyY),
            "Z" => Ok(Code::KeyZ),
            "0" => Ok(Code::Digit0),
            "1" => Ok(Code::Digit1),
            "2" => Ok(Code::Digit2),
            "3" => Ok(Code::Digit3),
            "4" => Ok(Code::Digit4),
            "5" => Ok(Code::Digit5),
            "6" => Ok(Code::Digit6),
            "7" => Ok(Code::Digit7),
            "8" => Ok(Code::Digit8),
            "9" => Ok(Code::Digit9),
            "F1" => Ok(Code::F1),
            "F2" => Ok(Code::F2),
            "F3" => Ok(Code::F3),
            "F4" => Ok(Code::F4),
            "F5" => Ok(Code::F5),
            "F6" => Ok(Code::F6),
            "F7" => Ok(Code::F7),
            "F8" => Ok(Code::F8),
            "F9" => Ok(Code::F9),
            "F10" => Ok(Code::F10),
            "F11" => Ok(Code::F11),
            "F12" => Ok(Code::F12),
            _ => Err(format!("Unsupported key: {}", s)),
    }
}

// ============= OPTIMIZATION LOGIC =============
async fn perform_optimization(
    app: AppHandle,
    engine: Engine,
    cfg: Arc<Mutex<Config>>,
    reason: Reason,
    with_progress: bool,
    areas_override: Option<Areas>,
) {
    // Controlla se un'ottimizzazione è già in corso
    if OPTIMIZATION_RUNNING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        tracing::info!("Optimization already running, skipping");
        return;
    }
    
    // FIX: Usa scopeguard per assicurarsi che il flag venga sempre rilasciato
    // anche in caso di panic o early return
    let _guard = scopeguard::guard((), |_| {
        OPTIMIZATION_RUNNING.store(false, Ordering::SeqCst);
    });
    
    // Assicura che i privilegi siano inizializzati
    if let Err(e) = ensure_privileges_initialized() {
        tracing::warn!("Failed to initialize privileges: {}", e);
    }
    
    // FIX: Se è la prima ottimizzazione, forza l'acquisizione dei privilegi
    // Questo è CRITICO perché alcuni privilegi potrebbero non essere stati acquisiti all'avvio
    if !FIRST_OPTIMIZATION_DONE.load(Ordering::SeqCst) {
        tracing::info!("First optimization - ensuring privileges are acquired...");
        
        // Forza re-inizializzazione privilegi con retry più aggressivo
        let mut retry_count = 0;
        let max_retries = 5;
        let mut privileges_ok = false;
        
        while retry_count < max_retries && !privileges_ok {
            match ensure_privileges_initialized() {
                Ok(_) => {
                    tracing::info!("✓ Privileges acquired successfully before first optimization (attempt {})", retry_count + 1);
                    privileges_ok = true;
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count < max_retries {
                        tracing::warn!("Failed to acquire privileges (attempt {}): {}, retrying...", retry_count, e);
                        // Delay progressivo: 200ms, 400ms, 600ms, 800ms, 1000ms
                        tokio::time::sleep(Duration::from_millis(200 * retry_count as u64)).await;
                    } else {
                        tracing::error!("✗ Failed to acquire privileges after {} attempts: {}", max_retries, e);
                        tracing::error!("Optimization may fail or be incomplete without proper privileges");
                    }
                }
            }
        }
        
        // Piccolo delay per assicurarsi che i privilegi siano completamente attivi
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        FIRST_OPTIMIZATION_DONE.store(true, Ordering::SeqCst);
        tracing::info!("First optimization setup complete, proceeding with optimization");
    }
    
    let (areas, show_notif, profile, language) = {
        match cfg.lock() {
            Ok(c) => {
                // Se areas_override è specificato, usalo, altrimenti usa le aree dal profilo
                let areas = areas_override.unwrap_or_else(|| {
                    // FIX: Sempre ricarica le aree dal profilo per assicurarsi di avere tutte quelle disponibili
                    // Questo è importante perché le aree disponibili possono cambiare o essere state salvate
                    // con una versione precedente di Windows
                    c.profile.get_memory_areas()
                });
                tracing::info!("Profile: {:?}, Areas: {:?} ({} areas, override: {})", 
                    c.profile, areas, areas.bits().count_ones(), areas_override.is_some());
                (
                    areas,
                    c.show_opt_notifications || reason == Reason::Manual,
                    c.profile.clone(),
                    c.language.clone()
                )
            },
            Err(_) => (areas_override.unwrap_or(Areas::WORKING_SET), true, Profile::Balanced, "en".to_string())
        }
    };
    
    // Esegui ottimizzazione
    let _before = engine.memory().ok();
    
    let result = if with_progress {
        engine.optimize(reason, areas, Some(|v, t, s: String| {
            emit_progress(&app, v, t, &s)
        }))
    } else {
        engine.optimize::<fn(u8, u8, String)>(reason, areas, None)
    };
    
    // Delay per stabilizzazione metriche
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    let after = engine.memory().ok();
    
    if with_progress {
        let _ = app.emit(EV_DONE, ());
    }
    
    // FIX: Mostra notifica solo se l'ottimizzazione ha avuto successo reale
    if show_notif {
        if let (Ok(res), Some(aft)) = (result, after) {
            let freed_mb = res.freed_physical_bytes.abs() as f64 / 1024.0 / 1024.0;
            let free_gb = aft.physical.free.bytes as f64 / 1024.0 / 1024.0 / 1024.0;
            
            // Verifica che almeno una area sia stata ottimizzata con successo
            let has_successful_area = res.areas.iter().any(|a| a.error.is_none());
            
            // Mostra notifica solo se:
            // 1. Abbiamo liberato almeno 1MB OPPURE
            // 2. Abbiamo almeno un'area ottimizzata con successo (anche se poco memoria liberata)
            if freed_mb > 1.0 || has_successful_area {
                let title = get_notification_title(&language, reason);
                let body = get_notification_body(&language, reason, freed_mb, free_gb, &profile);
                // Ottieni il tema corrente dalla configurazione
                let theme = {
                    let state = app.state::<AppState>();
                    let theme_result = match state.cfg.try_lock() {
                        Ok(cfg_guard) => cfg_guard.theme.clone(),
                        Err(_) => {
                            tracing::debug!("Config lock busy when getting theme for notification, using default");
                            "dark".to_string()
                        }
                    };
                    theme_result
                };
                tracing::info!("Attempting to show notification - freed: {:.2} MB, has_successful_area: {}", freed_mb, has_successful_area);
                match show_windows_notification(&app, &title, &body, &theme) {
                    Ok(_) => tracing::info!("✓ Notification sent successfully"),
                    Err(e) => tracing::error!("✗ Failed to send notification: {}", e),
                }
            } else {
                tracing::debug!("Skipping notification: insufficient memory freed ({:.2} MB) and no successful areas", freed_mb);
            }
        }
    }
    
    // Il flag viene rilasciato automaticamente dal guard
}

// ============= TAURI COMMANDS =============
#[tauri::command]
fn cmd_exit(app: tauri::AppHandle) {
    tracing::info!("Exiting application...");
    app.exit(0);
}

#[tauri::command]
fn cmd_memory_info(state: tauri::State<'_, AppState>) -> Result<crate::memory::types::MemoryInfo, String> {
    state.engine.memory().map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_config(state: tauri::State<'_, AppState>) -> Result<Config, String> {
    state.cfg.lock()
        .map_err(|_| "Config lock poisoned".to_string())
        .map(|c| c.clone())
}

#[tauri::command]
fn cmd_save_config(app: tauri::AppHandle, state: tauri::State<'_, AppState>, cfg_json: serde_json::Value) -> Result<(), String> {
    let mut current_cfg = state.cfg.lock()
        .map_err(|_| "Config lock poisoned".to_string())?
        .clone();
    
    let mut _need_menu_update = false;
    let mut need_icon_update = false;
    let mut need_hotkey_update = false;
    
    if let Some(obj) = cfg_json.as_object() {
        // Profile handling
        if let Some(v) = obj.get("profile") {
            if let Ok(profile) = serde_json::from_value::<Profile>(v.clone()) {
                current_cfg.profile = profile.clone();
                current_cfg.memory_areas = profile.get_memory_areas();
                current_cfg.run_priority = profile.get_priority();
                need_icon_update = true;
            }
        }
        
        // Memory areas
        if let Some(v) = obj.get("memory_areas") {
            if let Some(areas_num) = v.as_u64() {
                current_cfg.memory_areas = Areas::from_bits_truncate(areas_num as u32);
            } else if let Some(areas_str) = v.as_str() {
                current_cfg.memory_areas = parse_areas_string(areas_str);
            }
        }
        
        // Hotkey
        if let Some(v) = obj.get("hotkey") {
            if let Some(s) = v.as_str() {
                current_cfg.hotkey = s.to_string();
                need_hotkey_update = true;
            }
        }
        
        // Language
        if let Some(v) = obj.get("language") {
            if let Some(s) = v.as_str() {
                current_cfg.language = s.to_string();
                _need_menu_update = true;
            }
        }
        
        // Theme
        if let Some(v) = obj.get("theme") {
            if let Some(s) = v.as_str() {
                current_cfg.theme = s.to_string();
                need_icon_update = true; // Tray icon cambia colore in base al tema
            }
        }
        
        // Main color - supporto per light/dark separati
        if let Some(v) = obj.get("main_color_hex_light") {
            if let Some(s) = v.as_str() {
                current_cfg.main_color_hex_light = s.to_string();
            }
        }
        
        if let Some(v) = obj.get("main_color_hex_dark") {
            if let Some(s) = v.as_str() {
                current_cfg.main_color_hex_dark = s.to_string();
            }
        }
        
        // Backward compatibility
        if let Some(v) = obj.get("main_color_hex") {
            if let Some(s) = v.as_str() {
                current_cfg.main_color_hex = s.to_string();
            }
        }
        
        // Tray
        if let Some(v) = obj.get("tray") {
            if let Ok(tray) = serde_json::from_value::<config::TrayConfig>(v.clone()) {
                current_cfg.tray = tray;
                need_icon_update = true;
            }
        }
        
        // Boolean fields
        macro_rules! update_bool {
            ($field:ident) => {
                if let Some(v) = obj.get(stringify!($field)) {
                    if let Some(b) = v.as_bool() {
                        current_cfg.$field = b;
                    }
                }
            };
        }
        
        update_bool!(always_on_top);
        update_bool!(minimize_to_tray);
        update_bool!(show_opt_notifications);
        update_bool!(auto_update);
        update_bool!(close_after_opt);
        // Handle run_on_startup specially - it needs to call the system function
        if let Some(v) = obj.get("run_on_startup") {
            if let Some(b) = v.as_bool() {
                // Esegui l'operazione e logga eventuali errori
                if let Err(e) = crate::system::startup::set_run_on_startup(b) {
                    tracing::error!("Errore attivazione avvio automatico (settings): {:?}", e);
                }
                // Forziamo il valore booleano scelto dall'utente nel config,
                // invece di ri-leggerlo dal sistema che potrebbe essere lento ad aggiornarsi
                current_cfg.run_on_startup = b;
            }
        }
        update_bool!(compact_mode);
        
        // Numeric fields
        if let Some(v) = obj.get("auto_opt_interval_hours") {
            if let Some(n) = v.as_u64() {
                if n == 0 {
                    tracing::warn!("auto_opt_interval_hours cannot be 0, using default value 1");
                    current_cfg.auto_opt_interval_hours = 1;
                } else {
                    current_cfg.auto_opt_interval_hours = n.min(24) as u32;
                }
            }
        }
        
        if let Some(v) = obj.get("auto_opt_free_threshold") {
            if let Some(n) = v.as_u64() {
                if n == 0 {
                    tracing::warn!("auto_opt_free_threshold cannot be 0, using default value 1");
                    current_cfg.auto_opt_free_threshold = 1;
                } else {
                    current_cfg.auto_opt_free_threshold = n.min(100) as u8;
                }
            }
        }
        
        if let Some(v) = obj.get("font_size") {
            if let Some(n) = v.as_f64() {
                current_cfg.font_size = (n as f32).clamp(8.0, 24.0);
            }
        }
        
        // Process exclusions
        if let Some(v) = obj.get("process_exclusion_list") {
            if let Ok(list) = serde_json::from_value::<std::collections::BTreeSet<String>>(v.clone()) {
                current_cfg.process_exclusion_list = list;
            }
        }
        
        // Priority
        if let Some(v) = obj.get("run_priority") {
            if let Ok(priority) = serde_json::from_value::<Priority>(v.clone()) {
                current_cfg.run_priority = priority;
            }
        }
    }
    
    // Validate and save
    current_cfg.validate();
    
    // FIX #2: Rilascia il lock il prima possibile - salva la config e poi rilascia
    {
        let mut guard = state.cfg.lock()
            .map_err(|_| "Config lock poisoned".to_string())?;
        *guard = current_cfg.clone();
        // Salva prima di rilasciare il lock
        guard.save().map_err(|e| e.to_string())?;
        // Lock viene rilasciato qui automaticamente
    }
    
    // Update UI - tutte queste operazioni avvengono DOPO che il lock è stato rilasciato
    // Nota: update_menu non esiste più, il menu è gestito via HTML
    
    if need_icon_update {
        refresh_tray_icon(&app);
    }
    
    if need_hotkey_update {
        if let Err(e) = register_global_hotkey_v2(&app, &current_cfg.hotkey, state.inner().clone()) {
            tracing::error!("Failed to register hotkey: {}", e);
        }
    }
    
    Ok(())
}

#[tauri::command]
fn cmd_register_hotkey(
    app: tauri::AppHandle,
    hotkey: String,
    state: tauri::State<'_, AppState>
) -> Result<(), String> {
    if !crate::os::has_hotkey_manager() {
        return Err("Hotkey manager not available on this system".to_string());
    }
    
    {
        let mut cfg = state.cfg.lock()
            .map_err(|_| "Config lock poisoned".to_string())?;
        cfg.hotkey = hotkey.clone();
        cfg.save().map_err(|e| e.to_string())?;
    }
    
    register_global_hotkey_v2(&app, &hotkey, state.inner().clone())
}

#[tauri::command]
fn cmd_list_process_names() -> Result<Vec<String>, String> {
    Ok(crate::memory::ops::list_process_names())
}

#[tauri::command]
fn cmd_optimize_async(
    app: tauri::AppHandle, 
    state: tauri::State<'_, AppState>, 
    reason: Reason, 
    areas: String
) -> Result<(), String> {
    // FIX: Non impostare il flag qui, lascia che perform_optimization lo gestisca
    // Questo evita il doppio controllo del flag
    
    let engine = state.engine.clone();
    let cfg = state.cfg.clone();
    let areas_flags = parse_areas_string(&areas);
    
    // Passa le aree direttamente a perform_optimization invece di modificare la config condivisa
    // Questo evita race conditions se due ottimizzazioni vengono avviate contemporaneamente
    tauri::async_runtime::spawn(async move {
        // Esegui l'ottimizzazione (il flag viene gestito automaticamente da perform_optimization)
        perform_optimization(app.clone(), engine, cfg.clone(), reason, true, Some(areas_flags)).await;
        
        // Gestisci chiusura dopo ottimizzazione se configurato
        if reason == Reason::Manual {
            // FIX: Rilascia il lock prima dell'await
            let should_close = cfg.lock()
                .map(|c| c.close_after_opt)
                .unwrap_or(false);
            
            if should_close {
                tokio::time::sleep(Duration::from_secs(2)).await;
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.close();
                }
            }
        }
        // NOTA: Il flag OPTIMIZATION_RUNNING viene rilasciato automaticamente da scopeguard in perform_optimization
    });
    
    Ok(())
}

#[tauri::command]
fn cmd_run_on_startup(enable: bool, state: tauri::State<'_, AppState>) -> Result<(), String> {
    crate::system::startup::set_run_on_startup(enable)
        .map_err(|e| format!("Failed to set startup: {}. Try running as administrator.", e))?;
    
    let is_enabled = crate::system::startup::is_startup_enabled();
    if enable && !is_enabled {
        return Err("Failed to enable startup. Please add the app manually to Windows startup.".to_string());
    }
    
    let mut cfg = state.cfg.lock()
        .map_err(|_| "Config lock poisoned".to_string())?;
    cfg.run_on_startup = is_enabled;
    cfg.save().map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_complete_setup(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    setup_data: serde_json::Value,
) -> Result<(), String> {
    let mut cfg = state.cfg.lock()
        .map_err(|_| "Config lock poisoned".to_string())?;
    
    // Applica le impostazioni dal setup
    if let Some(obj) = setup_data.as_object() {
        if let Some(v) = obj.get("run_on_startup") {
            if let Some(b) = v.as_bool() {
                // Esegui l'operazione e logga eventuali errori
                if let Err(e) = crate::system::startup::set_run_on_startup(b) {
                    tracing::error!("Failed to set startup during setup: {:?}", e);
                }
                // Forziamo il valore booleano scelto dall'utente nel config,
                // invece di ri-leggerlo dal sistema che potrebbe essere lento ad aggiornarsi
                cfg.run_on_startup = b;
            }
        }
        
        if let Some(v) = obj.get("theme") {
            if let Some(s) = v.as_str() {
                cfg.theme = s.to_string();
                
                // Se il tema è light e non c'è un colore principale per light, imposta il default
                if s == "light" && cfg.main_color_hex_light.is_empty() {
                    cfg.main_color_hex_light = "#9a8a72".to_string();
                }
                // Se il tema è dark e non c'è un colore principale per dark, imposta il default
                if s == "dark" && cfg.main_color_hex_dark.is_empty() {
                    cfg.main_color_hex_dark = "#0a84ff".to_string();
                }
            }
        }
        
        if let Some(v) = obj.get("always_on_top") {
            if let Some(b) = v.as_bool() {
                cfg.always_on_top = b;
                let _ = crate::system::window::set_always_on_top(&app, b);
            }
        }
        
        if let Some(v) = obj.get("show_opt_notifications") {
            if let Some(b) = v.as_bool() {
                cfg.show_opt_notifications = b;
            }
        }
        
        if let Some(v) = obj.get("language") {
            if let Some(s) = v.as_str() {
                cfg.language = s.to_string();
            }
        }
    }
    
    // Segna il setup come completato
    cfg.setup_completed = true;
    cfg.save().map_err(|e| e.to_string())?;
    
    // Log delle impostazioni applicate per debug
    tracing::info!("Setup completed - Theme: {}, Language: {}, AlwaysOnTop: {}, ShowNotifications: {}, RunOnStartup: {}", 
        cfg.theme, cfg.language, cfg.always_on_top, cfg.show_opt_notifications, cfg.run_on_startup);
    
    // Prepara i dati per la sincronizzazione PRIMA di creare/mostrare la finestra
    let theme = cfg.theme.clone();
    let main_color_light = cfg.main_color_hex_light.clone();
    let main_color_dark = cfg.main_color_hex_dark.clone();
    let main_color = if theme == "light" {
        if !main_color_light.is_empty() {
            main_color_light
        } else {
            "#9a8a72".to_string()
        }
    } else {
        if !main_color_dark.is_empty() {
            main_color_dark
        } else {
            "#0a84ff".to_string()
        }
    };
    let language = cfg.language.clone();
    let always_on_top = cfg.always_on_top;
    
    // Mostra PRIMA la finestra principale, POI chiudi il setup
    // Assicurati che la finestra principale esista, altrimenti creala
    let main_window = if let Some(window) = app.get_webview_window("main") {
        tracing::info!("Main window already exists, showing it...");
        Some(window)
    } else {
        tracing::info!("Main window not found, creating it...");
        // Crea la finestra principale se non esiste
        match tauri::WebviewWindowBuilder::new(
            &app,
            "main",
            tauri::WebviewUrl::App("index.html".into())
        )
        .title("Tommy Memory Cleaner")
        .inner_size(480.0, 680.0)
        .resizable(false)
        .center()
        .skip_taskbar(false)
        .visible(true)
        .build()
        {
            Ok(window) => {
                tracing::info!("Main window created successfully after setup");
                Some(window)
            }
            Err(e) => {
                tracing::error!("Failed to create main window: {:?}", e);
                None
            }
        }
    };
    
    // Mostra la finestra principale e applica le impostazioni
    let main_window_shown = if let Some(main_window) = main_window {
        tracing::info!("Showing main window after setup...");
        
        // Applica always_on_top (sia true che false) - fallback se la finestra principale non risponde
        let _ = crate::system::window::set_always_on_top(&app, always_on_top);
        
        // Assicurati che la finestra sia visibile e non nascosta
        // Ordine corretto secondo best practices: skip_taskbar -> unminimize -> show -> center -> focus
        let _ = main_window.set_skip_taskbar(false);
        
        // Unminimize prima di show (se minimizzata)
        let _ = main_window.unminimize();
        
        // Mostra la finestra
        let show_result = main_window.show();
        if let Err(e) = show_result {
            tracing::error!("Failed to show main window: {:?}", e);
            false
        } else {
            // Centra la finestra
            let _ = main_window.center();
            
            // Focalizza la finestra (dopo show e center)
            if let Err(e) = main_window.set_focus() {
                tracing::warn!("Failed to focus main window: {:?}", e);
            }
            
            // Applica always_on_top anche alla finestra principale direttamente
            if let Err(e) = main_window.set_always_on_top(always_on_top) {
                tracing::warn!("Failed to set always_on_top on main window: {:?}", e);
            }
            
            // Emetti evento per applicare il tema e il colore nella finestra principale
            // Il frontend ascolterà questo evento e applicherà il tema e il colore corretto
            let _ = main_window.eval(&format!(
                r#"
                (function() {{
                    // Applica il tema
                    document.documentElement.setAttribute('data-theme', '{}');
                    localStorage.setItem('tmc_theme', '{}');
                    
                    // Applica il colore principale corretto per il tema
                    const root = document.documentElement;
                    root.style.setProperty('--btn-bg', '{}');
                    root.style.setProperty('--bar-fill', '{}');
                    root.style.setProperty('--input-focus', '{}');
                    
                    // Applica la lingua se disponibile
                    if (typeof window.setLanguage === 'function') {{
                        window.setLanguage('{}');
                    }}
                    
                    // Notifica il frontend di ricaricare la config
                    if (typeof window.dispatchEvent !== 'undefined') {{
                        window.dispatchEvent(new CustomEvent('config-updated'));
                    }}
                }})();
                "#,
                theme, theme, main_color, main_color, main_color, language
            ));
            
            // Piccolo delay per assicurarsi che la finestra principale sia completamente caricata
            std::thread::sleep(Duration::from_millis(200));
            true
        }
    } else {
        tracing::error!("Failed to get or create main window");
        false
    };
    
    // Emetti evento per notificare il frontend che il setup è completato
    // Il frontend chiuderà il setup dopo aver verificato che la finestra principale è pronta
    tracing::info!("Setup completed, emitting setup-complete event (main window shown: {})...", main_window_shown);
    let _ = app.emit("setup-complete", ());
    
    // NON chiudere il setup qui - lascia che il frontend lo chiuda dopo aver verificato
    // che la finestra principale è pronta. Questo evita race conditions e crash.
    
    Ok(())
}

#[tauri::command]
fn cmd_set_always_on_top(
    app: tauri::AppHandle, 
    on: bool, 
    state: tauri::State<'_, AppState>
) -> Result<(), String> {
    crate::system::window::set_always_on_top(&app, on)?;
    
    let mut cfg = state.cfg.lock()
        .map_err(|_| "Config lock poisoned".to_string())?;
    cfg.always_on_top = on;
    cfg.save().map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_set_priority(
    state: tauri::State<'_, AppState>, 
    priority: Priority
) -> Result<(), String> {
    crate::system::priority::set_priority(priority.clone())
        .map_err(|e| e.to_string())?;
    
    let mut cfg = state.cfg.lock()
        .map_err(|_| "Config lock poisoned".to_string())?;
    cfg.run_priority = priority;
    cfg.save().map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_get_system_theme() -> Result<String, String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Registry::*;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        
        let key_path: Vec<u16> = OsStr::new(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let mut hkey: *mut std::ffi::c_void = std::ptr::null_mut();
        let value_name: Vec<u16> = OsStr::new("AppsUseLightTheme")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let result = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                key_path.as_ptr(),
                0,
                KEY_READ,
                &mut hkey,
            )
        };
        
        if result == 0 && !hkey.is_null() {
            let mut value_data: u32 = 0;
            let mut value_type: u32 = 0;
            let mut data_size: u32 = std::mem::size_of::<u32>() as u32;
            
            let read_result = unsafe {
                RegQueryValueExW(
                    hkey,
                    value_name.as_ptr(),
                    std::ptr::null_mut(),
                    &mut value_type,
                    &mut value_data as *mut _ as *mut u8,
                    &mut data_size,
                )
            };
            
            unsafe {
                RegCloseKey(hkey);
            }
            
            if read_result == 0 && value_type == REG_DWORD {
                // 0 = dark, 1 = light
                return Ok(if value_data == 0 { "dark".to_string() } else { "light".to_string() });
            }
        }
    }
    
    // Default a dark se non riusciamo a rilevare
    Ok("dark".to_string())
}

#[tauri::command]
fn cmd_get_system_language() -> Result<String, String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Registry::*;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        
        // Leggi la lingua dal registro Windows
        let key_path: Vec<u16> = OsStr::new(r"Control Panel\International")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let mut hkey: *mut std::ffi::c_void = std::ptr::null_mut();
        let value_name: Vec<u16> = OsStr::new("LocaleName")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let result = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                key_path.as_ptr(),
                0,
                KEY_READ,
                &mut hkey,
            )
        };
        
        if result == 0 && !hkey.is_null() {
            let mut value_data = [0u16; 85];
            let mut value_type: u32 = 0;
            let mut data_size: u32 = (value_data.len() * 2) as u32;
            
            let read_result = unsafe {
                RegQueryValueExW(
                    hkey,
                    value_name.as_ptr(),
                    std::ptr::null_mut(),
                    &mut value_type,
                    value_data.as_mut_ptr() as *mut u8,
                    &mut data_size,
                )
            };
            
            unsafe {
                RegCloseKey(hkey);
            }
            
            if read_result == 0 && value_type == REG_SZ {
                // Trova la fine della stringa (primo null)
                let len = value_data.iter().position(|&x| x == 0).unwrap_or(value_data.len());
                let locale_str = String::from_utf16_lossy(&value_data[..len]);
                
                // Estrai il codice lingua (es. "it-IT" -> "it", "en-US" -> "en")
                let lang_code = locale_str.split('-').next().unwrap_or("en").to_lowercase();
                
                // Mappa i codici lingua supportati
                match lang_code.as_str() {
                    "it" => return Ok("it".to_string()),
                    "es" => return Ok("es".to_string()),
                    "fr" => return Ok("fr".to_string()),
                    "pt" => return Ok("pt".to_string()),
                    "de" => return Ok("de".to_string()),
                    "ar" => return Ok("ar".to_string()),
                    "ja" => return Ok("ja".to_string()),
                    "zh" => return Ok("zh".to_string()),
                    _ => return Ok("en".to_string()),
                }
            }
        }
    }
    
    // Default a inglese se non riusciamo a rilevare
    Ok("en".to_string())
}

// ============= AUTO-OPTIMIZER FIXED =============
fn start_auto_optimizer(app: tauri::AppHandle, engine: Engine, cfg: Arc<Mutex<Config>>) {
    tauri::async_runtime::spawn(async move {
        let mut last_scheduled_opt = Instant::now();
        let mut last_low_mem_opt = Instant::now();
        let mut check_interval = Duration::from_secs(30);
        
        // Aspetta un po' prima di iniziare i controlli
        tokio::time::sleep(Duration::from_secs(10)).await;
        
        loop {
            tokio::time::sleep(check_interval).await;
            
            let conf = match cfg.lock() {
                Ok(c) => c.clone(),
                Err(_) => continue,
            };
            
            let mut action_taken = false;
            
            // SCHEDULED OPTIMIZATION
            if conf.auto_opt_interval_hours > 0 {
                let hours_passed = last_scheduled_opt.elapsed().as_secs() / 3600;
                if hours_passed >= conf.auto_opt_interval_hours as u64 {
                    tracing::info!("Triggering scheduled optimization after {} hours", hours_passed);
                    
                    // Log evento automatico
                    crate::logging::event_viewer::log_auto_optimization_event(
                        "Scheduled",
                        conf.auto_opt_interval_hours as u8
                    );
                    
                    let app_clone = app.clone();
                    let engine_clone = engine.clone();
                    let cfg_clone = cfg.clone();
                    
                    tauri::async_runtime::spawn(async move {
                        // FIX: Usa with_progress: true per aggiornare la UI durante le ottimizzazioni automatiche
                        // Questo evita sovrapposizioni e mostra correttamente lo stato
                        perform_optimization(app_clone, engine_clone, cfg_clone, Reason::Schedule, true, None).await;
                    });
                    
                    last_scheduled_opt = Instant::now();
                    action_taken = true;
                }
            }
            
            // LOW MEMORY OPTIMIZATION (FIX del bug)
            if conf.auto_opt_free_threshold > 0 && !action_taken {
                // Controlla la memoria
                if let Ok(mem) = engine.memory() {
                    let free_percent = mem.physical.free.percentage;
                    
                    // FIX: Confronta correttamente con la soglia
                    if free_percent < conf.auto_opt_free_threshold {
                        // Verifica cooldown di 5 minuti
                        if last_low_mem_opt.elapsed() >= Duration::from_secs(300) {
                            tracing::info!(
                                "Triggering low memory optimization: {}% free < {}% threshold",
                                free_percent, conf.auto_opt_free_threshold
                            );
                            
                            // Log evento automatico
                            crate::logging::event_viewer::log_auto_optimization_event(
                                "Low Memory",
                                conf.auto_opt_free_threshold
                            );
                            
                            let app_clone = app.clone();
                            let engine_clone = engine.clone();
                            let cfg_clone = cfg.clone();
                            
                            tauri::async_runtime::spawn(async move {
                                // FIX: Usa with_progress: true per aggiornare la UI durante le ottimizzazioni automatiche
                                // Questo evita sovrapposizioni e mostra correttamente lo stato
                                perform_optimization(app_clone, engine_clone, cfg_clone, Reason::LowMemory, true, None).await;
                            });
                            
                            last_low_mem_opt = Instant::now();
                            action_taken = true;
                        } else {
                            let remaining = 300 - last_low_mem_opt.elapsed().as_secs();
                            tracing::debug!(
                                "Low memory detected ({}% free) but cooldown active ({}s remaining)",
                                free_percent, remaining
                            );
                        }
                        
                        // Aumenta frequenza controlli quando memoria bassa
                        check_interval = Duration::from_secs(30);
                    } else {
                        // Memoria OK, riduci frequenza controlli
                        check_interval = Duration::from_secs(60);
                    }
                }
            }
            
            // Adaptive interval
            if !action_taken {
                check_interval = (check_interval + Duration::from_secs(10)).min(Duration::from_secs(120));
            } else {
                check_interval = Duration::from_secs(30);
            }
        }
    });
}

// ============= WINDOW MANAGEMENT =============
#[tauri::command]
fn cmd_show_or_create_window(app: tauri::AppHandle) {
    show_or_create_window(&app);
}

#[tauri::command]
fn cmd_show_notification(app: tauri::AppHandle, title: String, message: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    // Ottieni il tema corrente dalla configurazione
    let theme = {
        match state.cfg.try_lock() {
            Ok(cfg_guard) => cfg_guard.theme.clone(),
            Err(_) => {
                tracing::debug!("Config lock busy in cmd_show_notification, using default theme");
                "dark".to_string()
            }
        }
    };
    show_windows_notification(&app, &title, &message, &theme)
}

fn show_or_create_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_skip_taskbar(false);  // Mostra nella taskbar
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = window.center();
    } else {
        tracing::info!("Creating new main window...");
        let result = tauri::WebviewWindowBuilder::new(
            app,
            "main",
            tauri::WebviewUrl::App("index.html".into())
        )
        .title("Tommy Memory Cleaner")
        .inner_size(480.0, 680.0)
        .resizable(false)
        .shadow(false)  // Rimuove ombra e bordo rettangolare su Windows 10
        .center()
        .skip_taskbar(false)  // Mostra nella taskbar
        .visible(true)  // Assicurati che sia visibile
        .build();
    
        match result {
            Ok(window) => {
                tracing::info!("Window created successfully");
                let _ = window.set_skip_taskbar(false);
                if let Err(e) = window.show() {
                    tracing::error!("Failed to show newly created window: {:?}", e);
                }
                let _ = window.set_focus();
            }
            Err(e) => {
                tracing::error!("Failed to create window: {:?}", e);
                eprintln!("FATAL ERROR: Failed to create window: {:?}", e);
            }
        }
    }
}

// ============= WEBVIEW2 CHECK =============
#[cfg(windows)]
fn check_webview2() {
    use std::process::Command;
    
    if let Ok(exe_path) = std::env::current_exe() {
        let path_str = exe_path.to_string_lossy().to_lowercase();
        let is_portable = !path_str.contains("program files") && 
                         !path_str.contains("programdata") &&
                         !path_str.contains("appdata");
        
        if is_portable {
            let output = Command::new("reg")
                .args(&[
                    "query",
                    r"HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
                    "/v",
                    "pv"
                ])
                .creation_flags(0x08000000 | 0x00000200)
                .output();
            
            let output_result = match output {
                Ok(result) => {
                    if !result.status.success() {
                        true // WebView2 non trovato
                    } else {
                        false // WebView2 trovato
                    }
                }
                Err(_) => true // Errore, considera WebView2 non trovato
            };
            
            if output_result {
                eprintln!("WebView2 Runtime not found!");
                eprintln!("Please download and install it from:");
                eprintln!("https://go.microsoft.com/fwlink/p/?LinkId=2124703");
                
                use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK, MB_ICONERROR};
                use std::ptr;
                
                let title = to_wide("Tommy Memory Cleaner - WebView2 Required");
                let msg = to_wide("WebView2 Runtime is required to run this application.\n\n\
                                  Please download and install it from:\n\
                                  https://go.microsoft.com/fwlink/p/?LinkId=2124703\n\n\
                                  The application will now exit.");
                
                unsafe {
                    MessageBoxW(
                        ptr::null_mut(), 
                        msg.as_ptr(),
                        title.as_ptr(),
                        MB_OK | MB_ICONERROR
                    );
                }
                
                std::process::exit(1);
            }
        }
    }
}

// ============= TRAY MENU POSITIONING =============
#[cfg(windows)]
fn get_taskbar_rect() -> Option<(i32, i32, i32, i32)> {
    use windows_sys::Win32::UI::Shell::{SHAppBarMessage, ABM_GETTASKBARPOS, APPBARDATA};
    use std::mem::zeroed;
    
    unsafe {
        let mut app_bar_data: APPBARDATA = zeroed();
        app_bar_data.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
        
        let result = SHAppBarMessage(ABM_GETTASKBARPOS, &mut app_bar_data);
        if result != 0 {
            let rc = app_bar_data.rc;
            Some((rc.left, rc.top, rc.right, rc.bottom))
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
fn get_taskbar_rect() -> Option<(i32, i32, i32, i32)> {
    None
}

fn position_tray_menu(window: &tauri::WebviewWindow) {
    // Aspetta un po' per assicurarsi che la finestra sia pronta
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Posiziona il menu vicino alla tray icon
    let _ = window.move_window(Position::TrayBottomRight);
    
    // Aspetta ancora un po' per il posizionamento
    std::thread::sleep(std::time::Duration::from_millis(50));
    
    // Usa le API Windows per ottenere la posizione esatta della taskbar
    if let (Ok(pos), Ok(size)) = (window.outer_position(), window.outer_size()) {
        if let Some(monitor) = window.current_monitor().ok().flatten() {
            let monitor_size = monitor.size();
            let monitor_pos = monitor.position();
            let screen_bottom = monitor_pos.y + monitor_size.height as i32;
            let menu_height = size.height as i32;
            let menu_bottom = pos.y + menu_height;
            
            // Prova a ottenere la posizione esatta della taskbar
            let taskbar_top = if let Some((_taskbar_left, taskbar_top, _taskbar_right, _taskbar_bottom)) = get_taskbar_rect() {
                // Taskbar trovata, determina se è in basso
                if taskbar_top > monitor_pos.y + (monitor_size.height as i32 / 2) {
                    // Taskbar in basso
                    Some(taskbar_top)
                } else {
                    // Taskbar in alto, sinistra o destra - usa fallback conservativo
                    None
                }
            } else {
                None
            };
            
            // Calcola safe_bottom: taskbar_top se disponibile, altrimenti margine conservativo
            let safe_bottom = taskbar_top.unwrap_or(screen_bottom - 80); // 80px margine conservativo
            
            // Se il menu va sotto la taskbar (o troppo in basso), spostalo sopra con margine
            if menu_bottom > safe_bottom {
                let new_y = safe_bottom - menu_height - 5; // 5px margine extra sopra la taskbar
                let final_y = new_y.max(monitor_pos.y + 5); // Almeno 5px dal top dello schermo
                
                tracing::debug!("Repositioning menu: menu_bottom={}, safe_bottom={}, new_y={}, final_y={}", 
                    menu_bottom, safe_bottom, new_y, final_y);
                
                let _ = window.set_position(tauri::PhysicalPosition {
                    x: pos.x,
                    y: final_y,
                });
                
                // Verifica che il posizionamento sia andato a buon fine
                std::thread::sleep(std::time::Duration::from_millis(50));
                if let Ok(new_pos) = window.outer_position() {
                    let new_menu_bottom = new_pos.y + menu_height;
                    if new_menu_bottom > safe_bottom {
                        tracing::warn!("Menu still below taskbar after repositioning: new_menu_bottom={}, safe_bottom={}", 
                            new_menu_bottom, safe_bottom);
                    } else {
                        tracing::debug!("Menu successfully positioned above taskbar: new_menu_bottom={}, safe_bottom={}", 
                            new_menu_bottom, safe_bottom);
                    }
                }
            } else {
                tracing::debug!("Menu already above taskbar: menu_bottom={}, safe_bottom={}", 
                    menu_bottom, safe_bottom);
            }
        }
    }
}

// ============= MAIN ENTRY POINT =============
fn main() {
    // Inizializza logging
    logging::init();
    
    // Controllo WebView2 (solo Windows)
    #[cfg(windows)]
    check_webview2();
    
    // CRITICO: Imposta l'AppUserModelID esplicitamente PRIMA di qualsiasi altra operazione
    // Questo forza Windows a usare il DisplayName registrato invece dell'AppUserModelID
    // IMPORTANTE: Questa funzione DEVE essere chiamata prima di qualsiasi altra API Windows
    // che potrebbe usare l'AppUserModelID (come shell notifications, jump lists, ecc.)
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        
        let app_id = "TommyMemoryCleaner";
        let app_id_wide: Vec<u16> = OsStr::new(app_id).encode_wide().chain(Some(0)).collect();
        
        unsafe {
            // SetCurrentProcessExplicitAppUserModelID ritorna HRESULT:
            // S_OK (0) = successo
            // Altri valori = errore
            let result = SetCurrentProcessExplicitAppUserModelID(app_id_wide.as_ptr());
            if result == 0 {
                tracing::info!("✓ AppUserModelID set explicitly: {}", app_id);
                eprintln!("[TMC] AppUserModelID set explicitly: {}", app_id);
            } else {
                // Log l'errore ma non bloccare l'app (alcune versioni di Windows potrebbero non supportarlo)
                tracing::warn!("✗ Failed to set AppUserModelID explicitly: HRESULT 0x{:08X}", result);
                tracing::debug!("This may cause notifications to show AppID instead of DisplayName");
                eprintln!("[TMC] ERROR: Failed to set AppUserModelID explicitly: HRESULT 0x{:08X}", result);
            }
        }
    }
    
    // Registra l'app per Windows Toast notifications PRIMA di tutto
    // Questo è fondamentale per mostrare correttamente nome e icona nelle notifiche
    #[cfg(windows)]
    {
        register_app_for_notifications();
    }
    
    // CONTROLLO CRITICO: Verifica che il programma sia eseguito come amministratore
    #[cfg(windows)]
    {
        use crate::utils::is_app_elevated;
        if !is_app_elevated() {
            eprintln!("ERRORE CRITICO: Tommy Memory Cleaner deve essere eseguito come Amministratore!");
            eprintln!("CRITICAL ERROR: Tommy Memory Cleaner must be run as Administrator!");
            
            // Mostra messaggio di errore all'utente
            let error_msg = format!(
                "Tommy Memory Cleaner richiede privilegi amministratore per funzionare correttamente.\n\n\
                Tommy Memory Cleaner requires administrator privileges to work properly.\n\n\
                Per favore, clicca destro sull'eseguibile e seleziona \"Esegui come amministratore\".\n\
                Please right-click the executable and select \"Run as administrator\"."
            );
            
            #[cfg(windows)]
            {
                use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK, MB_ICONERROR};
                use std::os::windows::ffi::OsStrExt;
                
                let title: Vec<u16> = std::ffi::OsStr::new("Tommy Memory Cleaner - Privilegi Richiesti / Privileges Required")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                
                let msg: Vec<u16> = std::ffi::OsStr::new(&error_msg)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                
                unsafe {
                    use std::ptr;
                    MessageBoxW(ptr::null_mut(), msg.as_ptr(), title.as_ptr(), (MB_OK | MB_ICONERROR) as u32);
                }
            }
            
            std::process::exit(1);
        }
        
        tracing::info!("Admin privileges confirmed - application running with elevated privileges");
    }
    
    // Inizializza privilegi all'avvio con retry
    // IMPORTANTE: I privilegi devono essere acquisiti PRIMA della prima ottimizzazione
    // Alcuni privilegi potrebbero richiedere privilegi elevati, ma proviamo comunque
    let mut retry_count = 0;
    let max_retries = 3;
    while retry_count < max_retries {
        match ensure_privileges_initialized() {
            Ok(_) => {
                tracing::info!("Privileges initialized successfully at startup (attempt {})", retry_count + 1);
                break;
            }
            Err(e) => {
                retry_count += 1;
                if retry_count < max_retries {
                    tracing::warn!("Failed to initialize privileges at startup (attempt {}): {}, retrying...", retry_count, e);
                    std::thread::sleep(std::time::Duration::from_millis(500 * retry_count as u64));
                } else {
                    tracing::warn!("Failed to initialize privileges at startup after {} attempts: {}", max_retries, e);
                    tracing::warn!("Privileges will be acquired on-demand during first optimization");
                }
            }
        }
    }
    
    // Registra l'app come trusted per ridurre falsi positivi antivirus
    #[cfg(windows)]
    if let Err(e) = antivirus::whitelist::register_as_trusted() {
        tracing::debug!("Failed to register as trusted (non-critical): {}", e);
    }
    
    // Carica configurazione
    let cfg = Arc::new(Mutex::new(
        Config::load().unwrap_or_else(|e| {
            tracing::warn!("Failed to load config: {}, using defaults", e);
            Config::default()
        })
    ));
    let engine = Engine::new(cfg.clone());
    let state = AppState { 
        cfg: cfg.clone(), 
        engine: engine.clone() 
    };
    
    // Build Tauri v2 app
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_positioner::init())
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![
            cmd_exit,
            cmd_show_or_create_window,
            cmd_show_notification,
            cmd_memory_info,
            cmd_get_config,
            cmd_save_config,
            cmd_complete_setup,
            cmd_register_hotkey,
            cmd_list_process_names,
            cmd_optimize_async,
            cmd_run_on_startup,
            cmd_set_always_on_top,
            cmd_set_priority,
            cmd_get_system_theme,
            cmd_get_system_language
        ])
        .setup(move |app| {
            let app_handle = app.handle();
            
            // Log iniziale
            tracing::info!("Application setup started");
            
            // Assicurati che la finestra principale sia visibile all'avvio
            if let Some(window) = app_handle.get_webview_window("main") {
                tracing::info!("Main window found, showing it...");
                let _ = window.set_skip_taskbar(false);
                if let Err(e) = window.show() {
                    tracing::error!("Failed to show window: {:?}", e);
                } else {
                    tracing::info!("Window shown successfully");
                }
                let _ = window.set_focus();
            } else {
                tracing::warn!("Main window not found at setup start");
            }
            
            // Build tray icon - gestisci errori senza crashare
            let mut tray_builder = match ui::tray::build(app_handle) {
                Ok(builder) => {
                    tracing::info!("Tray icon builder created successfully");
                    builder
                }
                Err(e) => {
                    tracing::error!("Failed to build tray icon: {:?}", e);
                    // Continua comunque senza tray icon - wrappa l'errore
                    return Err(Box::new(e) as Box<dyn std::error::Error>);
                }
            };
            
            // FIX: Rimosso il tipo esplicito errato. Lasciamo che Rust deduca i tipi.
            tray_builder = tray_builder.on_tray_icon_event(|tray, event| {
                // Collega positioner
                tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
                
                match event {
                    tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } => {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            // FIX: Gestisci il Result per evitare errori di tipo
                            if let Err(e) = window.show() { tracing::warn!("Show window failed: {}", e); }
                            let _ = window.set_focus();
                        } else {
                            show_or_create_window(&app);
                        }
                    }
                    tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Right,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } => {
                        let app_handle = tray.app_handle();
                        tracing::info!("Right click on tray icon detected");
                        
                        if let Some(menu_win) = app_handle.get_webview_window("tray_menu") {
                            tracing::info!("Tray menu window exists, showing it...");
                            
                            // Mostra il menu prima di posizionare
                            if let Err(e) = menu_win.show() { 
                                tracing::error!("Failed to show tray menu: {:?}", e); 
                            } else {
                                tracing::info!("Tray menu shown successfully");
                                
                                // Posiziona dopo lo show (importante per finestra fullscreen)
                                std::thread::sleep(std::time::Duration::from_millis(100));
                                position_tray_menu(&menu_win);
                                // Riposiziona di nuovo dopo un altro breve delay per essere sicuri
                                std::thread::sleep(std::time::Duration::from_millis(100));
                                position_tray_menu(&menu_win);
                                
                                // FIX: Rimosso setup listener inline - la gestione è nel file tray.ts
                                // Il menu si chiude solo quando si clicca fuori, non quando perde il focus
                                
                                // Forza always on top DOPO show e posizionamento
                                let _ = menu_win.set_always_on_top(true);
                                
                                // Piccolo delay per assicurarsi che always_on_top sia applicato
                                std::thread::sleep(std::time::Duration::from_millis(50));
                                
                                // Ri-applica always_on_top per sicurezza
                                let _ = menu_win.set_always_on_top(true);
                                
                                // Aspetta che il DOM sia pronto prima di chiamare loadConfig
                                std::thread::sleep(std::time::Duration::from_millis(100));
                                
                                // Chiama loadConfig per applicare tema e colori
                                let _ = menu_win.eval(r#"
                                    if (typeof loadConfig === 'function') {
                                        loadConfig();
                                    }
                                "#);
                            }
                        } else {
                            // Creazione lazy della finestra
                            tracing::info!("Creating tray menu window...");
                            let app_clone = app_handle.clone();
                            match WebviewWindowBuilder::new(
                                &app_clone,
                                "tray_menu",
                                WebviewUrl::App("tray.html".into())
                            )
                            .inner_size(1920.0, 1080.0)  // Fullscreen per overlay click capture (verrà ridimensionata dinamicamente)
                            .skip_taskbar(true)
                            .decorations(false)
                            .transparent(true)
                            .always_on_top(true)
                            .visible(false)
                            .shadow(false)  // Nessuna ombra per finestra trasparente
                            .resizable(false)
                            .focused(false)  // FIX: Non richiedere focus immediato
                            .build() {
                                Ok(menu_win) => {
                                    tracing::info!("Tray menu window created successfully");
                                    
                                    // Posiziona prima di mostrare
                                    position_tray_menu(&menu_win);
                                    
                                    // Mostra la finestra
                                    if let Err(e) = menu_win.show() {
                                        tracing::error!("Failed to show newly created tray menu: {:?}", e);
                                    } else {
                                        tracing::info!("Newly created tray menu shown");
                                        
                                        // Riposiziona dopo lo show
                                        position_tray_menu(&menu_win);
                                        // Riposiziona di nuovo dopo un altro breve delay per essere sicuri
                                        std::thread::sleep(std::time::Duration::from_millis(100));
                                        position_tray_menu(&menu_win);
                                        
                                        // Forza always on top DOPO show e posizionamento
                                        let _ = menu_win.set_always_on_top(true);
                                        
                                        // Piccolo delay per assicurarsi che always_on_top sia applicato
                                        std::thread::sleep(std::time::Duration::from_millis(50));
                                        
                                        // Ri-applica always_on_top per sicurezza
                                        let _ = menu_win.set_always_on_top(true);
                                        
                                        // Aspetta che il DOM sia pronto prima di chiamare loadConfig
                                        std::thread::sleep(std::time::Duration::from_millis(100));
                                        
                                        // Chiama loadConfig per applicare tema e colori
                                        let _ = menu_win.eval(r#"
                                            if (typeof loadConfig === 'function') {
                                                loadConfig();
                                            }
                                        "#);
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to create tray menu window: {:?}", e);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            });
            
            let tray = match tray_builder.build(app) {
                Ok(t) => {
                    tracing::info!("Tray icon built successfully");
                    t
                }
                Err(e) => {
                    tracing::error!("Failed to build tray: {:?}", e);
                    return Err(Box::new(e) as Box<dyn std::error::Error>);
                }
            };
            
            // Salviamo l'ID per usarlo in tray.rs
            let tray_id = tray.id().0.clone();
            if let Ok(mut id) = TRAY_ICON_ID.lock() {
                *id = Some(tray_id.clone());
            }
            
            // FIX: Rinomina variabili non usate con _ per rimuovere warning
            let _cfg_for_setup = cfg.clone();
            
            // FIX: Controlla se è stato chiamato con --startup-config dall'installer
            let args: Vec<String> = std::env::args().collect();
            let is_startup_config = args.iter().any(|a| a == "--startup-config");
            
            if is_startup_config {
                // Configura startup se richiesto dall'installer
                let _ = crate::system::startup::set_run_on_startup(true);
                if let Ok(mut c) = _cfg_for_setup.lock() {
                    c.run_on_startup = true;
                    let _ = c.save();
                }
                std::process::exit(0);
            }
            
            // Controlla se è il primo avvio e mostra il setup
            let show_setup = {
                let cfg_guard = _cfg_for_setup.lock();
                cfg_guard.as_ref()
                    .map(|c| !c.setup_completed)
                    .unwrap_or(true)
            };
            
            if show_setup {
                // Nascondi la finestra principale
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.hide();
                }
                
                // Crea e mostra la finestra di setup
                tracing::info!("First run detected, showing setup window...");
                let setup_url = WebviewUrl::App("setup.html".into());
                let app_clone = app_handle.clone();
                match WebviewWindowBuilder::new(&app_clone, "setup", setup_url)
                    .title("Tommy Memory Cleaner - Setup")
                    .inner_size(480.0, 600.0)
                    .min_inner_size(380.0, 500.0)
                    .max_inner_size(480.0, 700.0)
                    .resizable(false)
                    .decorations(false)
                    .transparent(true)
                    .shadow(false)
                    .center()
                    .skip_taskbar(false)
                    .always_on_top(true)
                    .visible(true)
                    .build()
                {
                    Ok(setup_window) => {
                        tracing::info!("Setup window created successfully");
                        // Assicura che sia sempre in primo piano
                        let _ = setup_window.set_always_on_top(true);
                        let _ = setup_window.set_focus();
                        // Ri-applica always_on_top dopo un breve delay per sicurezza
                        let app_clone = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            if let Some(window) = app_clone.get_webview_window("setup") {
                                let _ = window.set_always_on_top(true);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Failed to create setup window: {:?}", e);
                        // Fallback: mostra la finestra principale
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                        }
                    }
                }
            } else {
                // Mostra finestra all'avvio - usa app_handle invece di app
                tracing::info!("Checking main window visibility...");
                if let Some(window) = app_handle.get_webview_window("main") {
                    tracing::info!("Main window exists, ensuring it's visible...");
                    let _ = window.set_skip_taskbar(false);
                    if let Err(e) = window.show() {
                        tracing::error!("Failed to show window: {:?}", e);
                    } else {
                        tracing::info!("Window shown successfully");
                    }
                    let _ = window.unminimize();
                    if let Err(e) = window.center() {
                        tracing::warn!("Failed to center window: {:?}", e);
                    }
                    if let Err(e) = window.set_focus() {
                        tracing::warn!("Failed to focus window: {:?}", e);
                    }
                    // FIX: Abilita devtools per debug (tasto destro -> Inspect)
                    #[cfg(debug_assertions)]
                    {
                        let _ = window.open_devtools();
                    }
                } else {
                    // Se la finestra non esiste, creala
                    tracing::warn!("Main window not found, creating it...");
                    show_or_create_window(&app_handle);
                    // Verifica che sia stata creata
                    if let Some(window) = app_handle.get_webview_window("main") {
                        tracing::info!("Window created successfully");
                        let _ = window.set_skip_taskbar(false);
                        let _ = window.show();
                        let _ = window.set_focus();
                    } else {
                        tracing::error!("Failed to create main window!");
                    }
                }
            }
            
            // Aggiorna menu tray (Tauri v2 - gestito dal builder)
            
            // Applica configurazioni iniziali
            if let Ok(c) = _cfg_for_setup.lock() {
                // Startup
                if c.run_on_startup && !crate::system::startup::is_startup_enabled() {
                    let _ = crate::system::startup::set_run_on_startup(true);
                }
                
                // Registra l'app per Windows Toast notifications (richiesto per applicazioni non confezionate)
                // IMPORTANTE: deve essere chiamato PRIMA di qualsiasi notifica
                // La registrazione per le notifiche è già stata fatta all'avvio in main()
                
                // Hotkey
                if !c.hotkey.is_empty() && crate::os::has_hotkey_manager() {
                    if let Err(e) = register_global_hotkey_v2(&app_handle, &c.hotkey, state.clone()) {
                        tracing::error!("Failed to register hotkey at startup: {}", e);
                    }
                }
                
                // Always on top
                if c.always_on_top {
                    let _ = crate::system::window::set_always_on_top(&app_handle, true);
                }
                
                // Priority
                let _ = crate::system::priority::set_priority(c.run_priority.clone());
            }
            
            // Avvia i thread background
            // Avvia i thread background
            let engine_for_tray = state.engine.clone();
            crate::ui::tray::start_tray_updater(
                app_handle.clone(), 
                engine_for_tray
            );
            
            let engine_for_auto = state.engine.clone();
            start_auto_optimizer(
                app_handle.clone(), 
                engine_for_auto, 
                cfg.clone()
            );
            
            Ok(())
        })
        .on_window_event(|app, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // In Tauri v2, otteniamo la finestra dal parametro app usando il window dall'evento
                // Ma dobbiamo controllare quale finestra ha emesso l'evento
                // Controlla tutte le finestre per vedere quale sta chiudendo
                if let Some(setup_window) = app.get_webview_window("setup") {
                    // Se la finestra setup esiste e sta chiudendo, permette sempre la chiusura
                    if let Ok(is_visible) = setup_window.is_visible() {
                        if is_visible {
                            tracing::info!("Setup window close requested, allowing close");
                            // Permetti la chiusura del setup
                            return;
                        }
                    }
                }
                
                // Gestisci la chiusura della finestra principale
                if let Some(main_window) = app.get_webview_window("main") {
                    if let Ok(cfg) = main_window.app_handle().state::<AppState>().cfg.lock() {
                        if cfg.minimize_to_tray {
                            if let Err(e) = main_window.hide() {
                                tracing::warn!("Failed to hide window: {}", e);
                            }
                            api.prevent_close();
                        } else {
                            // Se non minimizza al tray, chiudi l'app e logga lo shutdown
                            crate::logging::shutdown();
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .map_err(|e| {
            tracing::error!("Failed to run TMC application: {:?}", e);
            eprintln!("FATAL ERROR: Failed to run TMC application: {:?}", e);
            e
        })
        .unwrap_or_else(|e| {
            eprintln!("FATAL: Application failed to start: {:?}", e);
            std::process::exit(1);
        });
}