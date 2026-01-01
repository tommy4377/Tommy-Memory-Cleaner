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
mod hotkeys;
mod auto_optimizer;
mod cli;
mod commands;
mod notifications;
mod antivirus {
    pub mod whitelist;
}

use crate::config::{Config, Profile};
use crate::engine::Engine;
use crate::memory::types::{Areas, Reason};
use crate::ui::bridge::{emit_progress, EV_DONE};
use crate::hotkeys::{register_global_hotkey_v2, cmd_register_hotkey};
use crate::auto_optimizer::start_auto_optimizer;
use crate::cli::run_console_mode;
use crate::notifications::{show_windows_notification, get_notification_title, get_notification_body, register_app_for_notifications};
use crate::commands::{show_or_create_window, position_tray_menu};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{Manager, AppHandle, Emitter};
use tauri::webview::WebviewWindowBuilder;
use tauri::WebviewUrl;
use tauri_plugin_positioner;
use once_cell::sync::Lazy;
use parking_lot::RwLock;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

static OPTIMIZATION_RUNNING: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
static PRIVILEGES_INITIALIZED: Lazy<RwLock<bool>> = Lazy::new(|| RwLock::new(false));
static FIRST_OPTIMIZATION_DONE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
pub(crate) static TRAY_ICON_ID: Lazy<std::sync::Mutex<Option<String>>> = Lazy::new(|| std::sync::Mutex::new(None));

#[derive(Clone)]
struct AppState { 
    cfg: Arc<Mutex<Config>>, 
    engine: Engine 
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
// All notification functions moved to notifications/ module

// ============= NOTIFICATION HELPERS =============
// Notification helpers moved to notifications/ module

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
// normalize_hotkey moved to hotkeys/manager.rs
// parse_hotkey_for_v2 moved to hotkeys/manager.rs
// code_from_str moved to hotkeys/codes.rs

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
// All commands moved to commands/ module

// ============= AUTO-OPTIMIZER FIXED =============
// start_auto_optimizer moved to auto_optimizer/scheduler.rs

// ============= WINDOW MANAGEMENT =============

// ============= TRAY MENU MANAGEMENT (ROBUST) =============
/// Mostra il tray menu con retry e fallback robusti
async fn show_tray_menu_with_retry(app: &AppHandle) {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_MS: u64 = 100;
    
    for attempt in 1..=MAX_RETRIES {
        tracing::debug!("Attempting to show tray menu (attempt {}/{})", attempt, MAX_RETRIES);
        
        // Prova prima a ottenere la finestra esistente
        if let Some(menu_win) = app.get_webview_window("tray_menu") {
            // ⭐ Aggiungi event handler per chiusura automatica (se non già presente)
            let menu_win_clone = menu_win.clone();
            menu_win.on_window_event(move |event| {
                match event {
                    tauri::WindowEvent::Focused(false) => {
                        // Quando il menu perde il focus, nascondilo
                        tracing::debug!("Tray menu lost focus, hiding...");
                        let _ = menu_win_clone.hide();
                    }
                    _ => {}
                }
            });
            
            // Verifica che la finestra sia valida
            if let Ok(is_visible) = menu_win.is_visible() {
                // Se già visibile, non fare nulla
                if is_visible {
                    tracing::debug!("Tray menu already visible, resetting auto-close timer");
                    // Reset del timer di chiusura automatica nel frontend
                    let _ = menu_win.eval(r#"
                        if (typeof showMenu === 'function') {
                            showMenu();
                        }
                    "#);
                    return;
                }
            }
            
            // Posiziona prima di mostrare (evita lampeggio)
            position_tray_menu(&menu_win);
            
            // Piccolo delay per assicurarsi che il posizionamento sia completato
            tokio::time::sleep(Duration::from_millis(50)).await;
            
            // Mostra il menu con retry
            match menu_win.show() {
                Ok(_) => {
                    tracing::info!("Tray menu shown successfully (attempt {})", attempt);
                    
                    // ⭐ INDISPENSABILE: Imposta il focus per ricevere eventi di focus su Windows
                    if let Err(e) = menu_win.set_focus() {
                        tracing::warn!("Failed to set focus on tray menu: {:?}", e);
                    }
                    
                    // Verifica che sia effettivamente visibile
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    
                    if let Ok(is_visible) = menu_win.is_visible() {
                        if is_visible {
                            // Chiama loadConfig per applicare tema e colori
                            let _ = menu_win.eval(r#"
                                if (typeof loadConfig === 'function') {
                                    loadConfig();
                                }
                                if (typeof showMenu === 'function') {
                                    showMenu();
                                }
                            "#);
                            
                            return;
                        } else {
                            tracing::warn!("Menu show() succeeded but window is not visible (attempt {})", attempt);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to show tray menu (attempt {}): {:?}", attempt, e);
                }
            }
        } else {
            // Finestra non esiste, creala
            tracing::info!("Tray menu window does not exist, creating it (attempt {})", attempt);
            
            let app_clone = app.clone();
            match WebviewWindowBuilder::new(
                &app_clone,
                "tray_menu",
                WebviewUrl::App("tray.html".into())
            )
            .inner_size(160.0, 120.0)
            .skip_taskbar(true)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .visible(false)
            .shadow(false)
            .resizable(false)
            .focused(true)  // ⭐ INDISPENSABILE su Windows per ricevere eventi di focus
            .build() {
                Ok(menu_win) => {
                    tracing::info!("Tray menu window created successfully (attempt {})", attempt);
                    
                    // ⭐ Gestisci la perdita di focus per chiudere automaticamente il menu
                    let menu_win_clone = menu_win.clone();
                    menu_win.on_window_event(move |event| {
                        match event {
                            tauri::WindowEvent::Focused(false) => {
                                // Quando il menu perde il focus, nascondilo
                                tracing::debug!("Tray menu lost focus, hiding...");
                                let _ = menu_win_clone.hide();
                            }
                            _ => {}
                        }
                    });
                    
                    // Posiziona prima di mostrare
                    position_tray_menu(&menu_win);
                    
                    // Piccolo delay per assicurarsi che il posizionamento sia completato
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    
                    // Mostra la finestra
                    match menu_win.show() {
                        Ok(_) => {
                            tracing::info!("Newly created tray menu shown successfully (attempt {})", attempt);
                            
                            // ⭐ INDISPENSABILE: Imposta il focus per ricevere eventi di focus su Windows
                            if let Err(e) = menu_win.set_focus() {
                                tracing::warn!("Failed to set focus on newly created tray menu: {:?}", e);
                            }
                            
                            // Verifica che sia effettivamente visibile
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            
                            if let Ok(is_visible) = menu_win.is_visible() {
                                if is_visible {
                                    // Chiama loadConfig per applicare tema e colori
                                    let _ = menu_win.eval(r#"
                                        if (typeof loadConfig === 'function') {
                                            loadConfig();
                                        }
                                        if (typeof showMenu === 'function') {
                                            showMenu();
                                        }
                                    "#);
                                    
                                    return;
                                } else {
                                    tracing::warn!("Menu show() succeeded but window is not visible after creation (attempt {})", attempt);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to show newly created tray menu (attempt {}): {:?}", attempt, e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create tray menu window (attempt {}): {:?}", attempt, e);
                }
            }
        }
        
        // Se non è riuscito, aspetta prima di riprovare
        if attempt < MAX_RETRIES {
            tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS * attempt as u64)).await;
        }
    }
    
    tracing::error!("Failed to show tray menu after {} attempts", MAX_RETRIES);
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
                
                let title = to_wide("Tommy Memory Cleaner - WebView2 Required");
                let msg = to_wide("WebView2 Runtime is required to run this application.\n\n\
                                  Please download and install it from:\n\
                                  https://go.microsoft.com/fwlink/p/?LinkId=2124703\n\n\
                                  The application will now exit.");
                
                unsafe {
                    MessageBoxW(
                        0 as _, 
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

// ============= MAIN ENTRY POINT =============
fn main() {
    // Inizializza logging
    logging::init();
    
    // Console mode: controlla se ci sono argomenti da linea di comando
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        return run_console_mode(&args);
    }
    
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
                    MessageBoxW(0 as _, msg.as_ptr(), title.as_ptr(), (MB_OK | MB_ICONERROR) as u32);
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
            // Commands from config module
            commands::config::cmd_exit,
            commands::config::cmd_get_config,
            commands::config::cmd_save_config,
            commands::config::cmd_complete_setup,
            // Commands from memory module
            commands::memory::cmd_memory_info,
            commands::memory::cmd_list_process_names,
            commands::memory::cmd_optimize_async,
            // Commands from system module
            commands::system::cmd_run_on_startup,
            commands::system::cmd_set_always_on_top,
            commands::system::cmd_set_priority,
            // Commands from theme module
            commands::theme::cmd_get_system_theme,
            commands::theme::cmd_get_system_language,
            // Commands from ui module
            commands::ui::cmd_show_or_create_window,
            commands::ui::cmd_show_notification,
            // Commands from hotkeys module
            cmd_register_hotkey
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
                        
                        // Usa async runtime per gestire l'apertura in modo non bloccante
                        let app_clone = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            show_tray_menu_with_retry(&app_clone).await;
                        });
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
            
            // ⭐ Controlla se è il primo avvio e mostra il setup
            // Verifica anche che il file di config esista per evitare setup multipli
            let show_setup = {
                // ⭐ Fallback 1: Verifica se la finestra setup è già aperta
                if app_handle.get_webview_window("setup").is_some() {
                    tracing::info!("Setup window already exists, skipping creation");
                    return Ok(());
                }
                
                let cfg_guard = _cfg_for_setup.lock();
                let should_show = cfg_guard.as_ref()
                    .map(|c| !c.setup_completed)
                    .unwrap_or(true);
                
                // ⭐ Fallback 2: verifica anche se il file config esiste
                // Se il file esiste ma setup_completed è false, potrebbe essere un problema
                // In quel caso, assumiamo che il setup sia già stato fatto
                if should_show {
                    let config_path = crate::config::get_portable_detector().config_path();
                    if config_path.exists() {
                        // Il file esiste, verifica se contiene setup_completed
                        if let Ok(content) = std::fs::read_to_string(&config_path) {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                                if let Some(setup_completed) = json.get("setup_completed").and_then(|v| v.as_bool()) {
                                    if setup_completed {
                                        tracing::info!("Config file exists with setup_completed=true, skipping setup");
                                        return Ok(());
                                    } else {
                                        tracing::warn!("Config file exists but setup_completed=false, this might indicate a corrupted config");
                                    }
                                }
                            }
                        }
                    }
                }
                
                should_show
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
                    .shadow(true)
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
                    if let Err(e) = register_global_hotkey_v2(&app_handle, &c.hotkey, state.cfg.clone()) {
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
