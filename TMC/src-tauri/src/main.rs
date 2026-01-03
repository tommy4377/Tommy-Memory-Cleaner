#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

/// Tommy Memory Cleaner - Main Application Entry Point
///
/// This is the main entry point for the Tommy Memory Cleaner application.
/// It initializes all subsystems including:
/// - Memory optimization engine
/// - System tray integration
/// - Global hotkeys
/// - Auto-optimization scheduler
/// - Notification system
/// - Security checks
mod antivirus;
mod auto_optimizer;
mod cli;
mod commands;
mod config;
mod engine;
mod hotkeys;
mod logging;
mod memory;
mod notifications;
mod os;
mod security;
mod system;
mod ui;

use crate::auto_optimizer::start_auto_optimizer;
use crate::cli::run_console_mode;
use crate::commands::{position_tray_menu, show_or_create_window};
use crate::config::{Config, Profile};
use crate::engine::Engine;
use crate::hotkeys::{cmd_register_hotkey, register_global_hotkey_v2};
use crate::memory::types::{Areas, Reason};
use crate::notifications::{register_app_for_notifications, show_windows_notification};
use crate::ui::bridge::{emit_progress, EV_DONE};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::webview::WebviewWindowBuilder;
use tauri::WebviewUrl;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_positioner;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Global state tracking optimization status
static OPTIMIZATION_RUNNING: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
/// Tracks if admin privileges have been initialized
static PRIVILEGES_INITIALIZED: Lazy<RwLock<bool>> = Lazy::new(|| RwLock::new(false));
/// Tracks if first optimization has been completed
static FIRST_OPTIMIZATION_DONE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));
/// Stores the tray icon ID for updates
pub(crate) static TRAY_ICON_ID: Lazy<std::sync::Mutex<Option<String>>> =
    Lazy::new(|| std::sync::Mutex::new(None));

/// Application state shared across Tauri commands
#[derive(Clone)]
struct AppState {
    cfg: Arc<Mutex<Config>>,
    engine: Engine,
    translations: crate::commands::TranslationState,
    rate_limiter: Arc<Mutex<crate::security::RateLimiter>>,
}

// ============= WINDOWS HELPERS =============
#[cfg(windows)]
/// Convert UTF-8 string to Windows wide string (UTF-16)
fn to_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

// ============= PRIVILEGE MANAGEMENT =============
/// Restart the application with elevated privileges
#[cfg(windows)]
fn restart_with_elevation() -> Result<(), Box<dyn std::error::Error>> {
    use std::env;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::Foundation::GetLastError;
    
    let current_exe = env::current_exe()?;
    let exe_path = current_exe.to_string_lossy();
    
    tracing::info!("Restarting application with elevated privileges...");
    
    // Keep the wide string alive for the duration of the call
    let runas = to_wide("runas");
    let exe_wide = exe_path.encode_utf16().chain(std::iter::once(0)).collect::<Vec<_>>();
    
    let result = unsafe {
        ShellExecuteW(
            0, // HWND null - deve essere 0 non std::ptr::null_mut()
            runas.as_ptr(),
            exe_wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1, // SW_SHOWNORMAL
        )
    };
    
    if result <= 32 {
        let error_code = unsafe { GetLastError() };
        tracing::error!("Failed to restart with elevation. ShellExecuteW returned: {}, GetLastError: {}", result, error_code);
        Err(format!("Failed to restart with elevation (code: {}, error: {})", result, error_code).into())
    } else {
        std::process::exit(0);
    }
}

/// Initialize required Windows privileges for memory optimization
///
/// This function ensures the process has the necessary privileges
/// to perform advanced memory operations on other processes.
fn ensure_privileges_initialized() -> Result<(), String> {
    // Check if already initialized
    if *PRIVILEGES_INITIALIZED.read() {
        return Ok(());
    }

    // Acquire write lock and re-check
    let mut guard = PRIVILEGES_INITIALIZED.write();
    if *guard {
        return Ok(());
    }

    tracing::info!("Initializing Windows privileges...");

    // List of all required privileges
    let privileges = [
        "SeDebugPrivilege",                // To optimize working set of other processes
        "SeIncreaseQuotaPrivilege",        // To modify system cache
        "SeProfileSingleProcessPrivilege", // For advanced memory operations
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
                // Don't fail completely, just warn
            }
        }
    }

    tracing::info!(
        "Privileges initialized: {}/{} acquired",
        success_count,
        privileges.len()
    );
    
    // Mark as initialized even if not all privileges were acquired
    *guard = true;
    Ok(())
}

// ============= NOTIFICATIONS =============
// All notification functions moved to notifications/ module

// ============= NOTIFICATION HELPERS =============
// Notification helpers moved to notifications/ module

// ============= TRAY MENU (Tauri v2) =============
// Tray menu is managed directly in the builder, see ui::tray::build()

/// Refresh the tray icon based on current memory usage
///
/// This function updates the system tray icon to reflect current memory
/// usage when enabled in settings. Uses non-blocking locks to prevent deadlocks.
fn refresh_tray_icon(app: &AppHandle) {
    let state = app.state::<AppState>();

    // Use try_lock to avoid deadlocks and acquire all necessary info
    let (_show_mem_usage, mem_percent) = {
        // Try to acquire lock without blocking
        match state.cfg.try_lock() {
            Ok(c) => {
                let show_mem = c.tray.show_mem_usage;
                // Release lock BEFORE calling engine.memory() to avoid deadlock
                drop(c);

                if !show_mem {
                    tracing::debug!(
                        "refresh_tray_icon: show_mem_usage is false, will load default icon"
                    );
                    (false, 0)
                } else {
                    // Now that lock is released, we can safely call memory()
                    let mem_percent = state
                        .engine
                        .memory()
                        .map(|mem| {
                            // Clamp percentage between 0-100 (should already be in range, but for safety)
                            mem.physical.used.percentage.min(100)
                        })
                        .unwrap_or_else(|e| {
                            tracing::warn!("Failed to get memory info: {}, using 0", e);
                            0
                        });
                    (true, mem_percent)
                }
            }
            Err(_) => {
                tracing::debug!("Config lock busy, skipping tray icon update");
                (false, 0)
            }
        }
    };

    // If show_mem_usage is false, update_tray_icon will use default icon
    crate::ui::tray::update_tray_icon(app, mem_percent);
}

// ============= AREA PARSING =============
/// Parse areas string from configuration into Areas bitflags
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
            "" => {} // Ignore empty strings
            unknown => {
                tracing::warn!(
                    "Unknown memory area flag: '{}' in areas string: '{}'",
                    unknown,
                    areas_str
                );
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
/// Perform memory optimization with specified parameters
///
/// This is the core optimization function that:
/// - Checks if optimization is already running
/// - Ensures proper privileges are acquired
/// - Executes optimization with progress updates
/// - Handles cleanup and error recovery
async fn perform_optimization(
    app: AppHandle,
    engine: Engine,
    cfg: Arc<Mutex<Config>>,
    reason: Reason,
    with_progress: bool,
    areas_override: Option<Areas>,
) {
    // Check if optimization is already running
    if OPTIMIZATION_RUNNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        tracing::info!("Optimization already running, skipping");
        return;
    }

    // Use scopeguard to ensure flag is always released
    // even in case of panic or early return
    let _guard = scopeguard::guard((), |_| {
        OPTIMIZATION_RUNNING.store(false, Ordering::SeqCst);
    });

    // Ensure privileges are initialized
    if let Err(e) = ensure_privileges_initialized() {
        tracing::warn!("Failed to initialize privileges: {}", e);
    }

    // If this is the first optimization, force privilege acquisition
    // This is CRITICAL because some privileges might not have been acquired at startup
    if !FIRST_OPTIMIZATION_DONE.load(Ordering::SeqCst) {
        tracing::info!("First optimization - ensuring privileges are acquired...");

        // Force re-initialization of privileges with more aggressive retry
        let mut retry_count = 0;
        let max_retries = 5;
        let mut privileges_ok = false;

        while retry_count < max_retries && !privileges_ok {
            match ensure_privileges_initialized() {
                Ok(_) => {
                    tracing::info!(
                        "✓ Privileges acquired successfully before first optimization (attempt {})",
                        retry_count + 1
                    );
                    privileges_ok = true;
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count < max_retries {
                        tracing::warn!(
                            "Failed to acquire privileges (attempt {}): {}, retrying...",
                            retry_count,
                            e
                        );
                        // Progressive delay: 200ms, 400ms, 600ms, 800ms, 1000ms
                        tokio::time::sleep(Duration::from_millis(200 * retry_count as u64)).await;
                    } else {
                        tracing::error!(
                            "✗ Failed to acquire privileges after {} attempts: {}",
                            max_retries,
                            e
                        );
                        tracing::error!(
                            "Optimization may fail or be incomplete without proper privileges"
                        );
                    }
                }
            }
        }

        // Small delay to ensure privileges are fully active
        tokio::time::sleep(Duration::from_millis(200)).await;

        FIRST_OPTIMIZATION_DONE.store(true, Ordering::SeqCst);
        tracing::info!("First optimization setup complete, proceeding with optimization");
    }

    let (areas, show_notif, profile, _language) = {
        match cfg.lock() {
            Ok(c) => {
                // If areas_override is specified, use it, otherwise use areas from profile
                let areas = if let Some(override_areas) = areas_override {
                    override_areas
                } else {
                    // This is important because available areas can change or have been saved
                    // with a previous version of Windows
                    c.profile.get_memory_areas()
                };
                tracing::info!(
                    "Profile: {:?}, Areas: {:?} ({} areas, override: {})",
                    c.profile,
                    areas,
                    areas.bits().count_ones(),
                    areas_override.is_some()
                );
                (
                    areas,
                    c.show_opt_notifications || reason == Reason::Manual,
                    c.profile.clone(),
                    c.language.clone(),
                )
            }
            Err(_) => (
                areas_override.unwrap_or(Areas::WORKING_SET),
                true,
                Profile::Balanced,
                "en".to_string(),
            ),
        }
    };

    // Execute optimization
    let _before = engine.memory().ok();

    let result = if with_progress {
        engine.optimize(
            reason,
            areas,
            Some(|v, t, s: String| emit_progress(&app, v, t, &s)),
        )
    } else {
        engine.optimize::<fn(u8, u8, String)>(reason, areas, None)
    };

    // Delay for metrics stabilization
    tokio::time::sleep(Duration::from_millis(300)).await;

    let after = engine.memory().ok();

    if with_progress {
        let _ = app.emit(EV_DONE, ());
    }

    // FIX: Only show notification if optimization was actually successful
    if show_notif {
        if let (Ok(res), Some(aft)) = (result, after) {
            let freed_mb = res.freed_physical_bytes.abs() as f64 / 1024.0 / 1024.0;
            let free_gb = aft.physical.free.bytes as f64 / 1024.0 / 1024.0 / 1024.0;

            // Verify that at least one area was successfully optimized
            let has_successful_area = res.areas.iter().any(|a| a.error.is_none());

            // Show notification only if:
            // 1. We freed at least 1MB OR
            // 2. We have at least one successfully optimized area (even if little memory freed)
            if freed_mb > 1.0 || has_successful_area {
                // Use cached translations from frontend
                let title_key = match reason {
                    Reason::Manual => "TMC • Optimization completed",
                    Reason::Schedule => "TMC • Scheduled optimization",
                    Reason::LowMemory => "TMC • Low memory optimization",
                    Reason::Hotkey => "TMC • Hotkey optimization",
                };

                let title = {
                    let state = app.state::<AppState>();
                    crate::commands::get_translation(&state.translations, title_key)
                };

                // Format notification body using translations
                let profile_key = match profile {
                    Profile::Normal => "Normal",
                    Profile::Balanced => "Balanced",
                    Profile::Gaming => "Gaming",
                };

                let profile_name = {
                    let state = app.state::<AppState>();
                    crate::commands::get_translation(&state.translations, profile_key)
                };

                let body_template = {
                    let state = app.state::<AppState>();
                    crate::commands::get_translation(
                        &state.translations,
                        "✅ Freed: %.1f MB\n🧠 Free RAM: %.2f GB\n🎯 Profile: %s",
                    )
                };

                let body = body_template
                    .replace("%.1f", &format!("{:.1}", freed_mb.abs()))
                    .replace("%.2f", &format!("{:.2}", free_gb))
                    .replace("%s", &profile_name);

                // Emit event to frontend for memory stats tracking
                let event_result = app.emit("optimization-completed", serde_json::json!({
                    "freed_physical_mb": freed_mb.abs()
                }));
                tracing::debug!("Emitted optimization-completed event with {} MB freed, result: {:?}", freed_mb.abs(), event_result);
                // Get current theme from configuration
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
                tracing::info!(
                    "Attempting to show notification - freed: {:.2} MB, has_successful_area: {}",
                    freed_mb,
                    has_successful_area
                );
                match show_windows_notification(&app, &title, &body, &theme) {
                    Ok(_) => tracing::info!("✓ Notification sent successfully"),
                    Err(e) => tracing::error!("✗ Failed to send notification: {}", e),
                }
            } else {
                tracing::debug!("Skipping notification: insufficient memory freed ({:.2} MB) and no successful areas", freed_mb);
            }
        }
    }

    // The flag is automatically released by the guard
}

// ============= TAURI COMMANDS =============
// All commands moved to commands/ module

// ============= AUTO-OPTIMIZER FIXED =============
// start_auto_optimizer moved to auto_optimizer/scheduler.rs

// ============= WINDOW MANAGEMENT =============

// ============= TRAY MENU MANAGEMENT (ROBUST) =============
/// Show tray menu with retry and robust fallbacks
async fn show_tray_menu_with_retry(app: &AppHandle) {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_MS: u64 = 100;

    for attempt in 1..=MAX_RETRIES {
        tracing::debug!(
            "Attempting to show tray menu (attempt {}/{})",
            attempt,
            MAX_RETRIES
        );

        // First try to get existing window
        if let Some(menu_win) = app.get_webview_window("tray_menu") {
            // Add event handler for auto-close (if not already present)
            let menu_win_clone = menu_win.clone();
            menu_win.on_window_event(move |event| {
                match event {
                    tauri::WindowEvent::Focused(false) => {
                        // When menu loses focus, hide it
                        tracing::debug!("Tray menu lost focus, hiding...");
                        let _ = menu_win_clone.hide();
                    }
                    _ => {}
                }
            });

            // Verify window is valid
            if let Ok(is_visible) = menu_win.is_visible() {
                // If already visible, do nothing
                if is_visible {
                    tracing::debug!("Tray menu already visible, resetting auto-close timer");
                    // Reset auto-close timer in frontend
                    let _ = menu_win.eval(
                        r#"
                        if (typeof showMenu === 'function') {
                            showMenu();
                        }
                    "#,
                    );
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

                    // Emit event globally to trigger config reload in frontend
                    let _ = app.emit("tray-menu-open", ());

                    // ⭐ INDISPENSABILE: Imposta il focus per ricevere eventi di focus su Windows
                    if let Err(e) = menu_win.set_focus() {
                        tracing::warn!("Failed to set focus on tray menu: {:?}", e);
                    }

                    // Verifica che sia effettivamente visibile
                    tokio::time::sleep(Duration::from_millis(100)).await;

                    if let Ok(is_visible) = menu_win.is_visible() {
                        if is_visible {
                            // Chiama loadConfig per applicare tema e colori
                            let _ = menu_win.eval(
                                r#"
                                if (typeof loadConfig === 'function') {
                                    loadConfig();
                                }
                                if (typeof showMenu === 'function') {
                                    showMenu();
                                }
                            "#,
                            );

                            return;
                        } else {
                            tracing::warn!(
                                "Menu show() succeeded but window is not visible (attempt {})",
                                attempt
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to show tray menu (attempt {}): {:?}", attempt, e);
                }
            }
        } else {
            // Finestra non esiste, creala
            tracing::info!(
                "Tray menu window does not exist, creating it (attempt {})",
                attempt
            );

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
            .build()
            {
                Ok(menu_win) => {
                    tracing::info!(
                        "Tray menu window created successfully (attempt {})",
                        attempt
                    );

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
                            tracing::info!(
                                "Newly created tray menu shown successfully (attempt {})",
                                attempt
                            );
                            // Emit event globally to trigger config reload in frontend
                            let _ = app.emit("tray-menu-open", ());

                            // ⭐ INDISPENSABILE: Imposta il focus per ricevere eventi di focus su Windows
                            if let Err(e) = menu_win.set_focus() {
                                tracing::warn!(
                                    "Failed to set focus on newly created tray menu: {:?}",
                                    e
                                );
                            }

                            // Verifica che sia effettivamente visibile
                            tokio::time::sleep(Duration::from_millis(100)).await;

                            if let Ok(is_visible) = menu_win.is_visible() {
                                if is_visible {
                                    // Chiama loadConfig per applicare tema e colori
                                    let _ = menu_win.eval(
                                        r#"
                                        if (typeof loadConfig === 'function') {
                                            loadConfig();
                                        }
                                        if (typeof showMenu === 'function') {
                                            showMenu();
                                        }
                                    "#,
                                    );

                                    return;
                                } else {
                                    tracing::warn!("Menu show() succeeded but window is not visible after creation (attempt {})", attempt);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to show newly created tray menu (attempt {}): {:?}",
                                attempt,
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to create tray menu window (attempt {}): {:?}",
                        attempt,
                        e
                    );
                }
            }
        }

        // If failed, wait before retrying
        if attempt < MAX_RETRIES {
            tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS * attempt as u64)).await;
        }
    }

    tracing::error!("Failed to show tray menu after {} attempts", MAX_RETRIES);
}

// ============= WEBVIEW2 CHECK =============
#[cfg(windows)]
/// Check if WebView2 runtime is installed
fn check_webview2() {
    use std::process::Command;

    if let Ok(exe_path) = std::env::current_exe() {
        let path_str = exe_path.to_string_lossy().to_lowercase();
        let is_portable = !path_str.contains("program files")
            && !path_str.contains("programdata")
            && !path_str.contains("appdata");

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
                Err(_) => true, // Errore, considera WebView2 non trovato
            };

            if output_result {
                eprintln!("WebView2 Runtime not found!");
                eprintln!("Please download and install it from:");
                eprintln!("https://go.microsoft.com/fwlink/p/?LinkId=2124703");

                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    MessageBoxW, MB_ICONERROR, MB_OK,
                };

                let title = to_wide("Tommy Memory Cleaner - WebView2 Required");
                let msg = to_wide(
                    "WebView2 Runtime is required to run this application.\n\n\
                                  Please download and install it from:\n\
                                  https://go.microsoft.com/fwlink/p/?LinkId=2124703\n\n\
                                  The application will now exit.",
                );

                unsafe {
                    MessageBoxW(0 as _, msg.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR);
                }

                std::process::exit(1);
            }
        }
    }
}

// ============= MAIN ENTRY POINT =============
fn main() {
    // Initialize logging
    logging::init();

    // Console mode: check if there are command line arguments
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        return run_console_mode(&args);
    }

    // WebView2 check (Windows only)
    #[cfg(windows)]
    check_webview2();

    // CRITICAL: Set AppUserModelID explicitly BEFORE any other operation
    // This forces Windows to use the registered DisplayName instead of AppUserModelID
    // IMPORTANT: This function MUST be called before any other Windows API
    // that might use AppUserModelID (like shell notifications, jump lists, etc.)
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

        let app_id = "TommyMemoryCleaner";
        let app_id_wide: Vec<u16> = OsStr::new(app_id).encode_wide().chain(Some(0)).collect();

        unsafe {
            // SetCurrentProcessExplicitAppUserModelID returns HRESULT:
            // S_OK (0) = success
            // Other values = error
            let result = SetCurrentProcessExplicitAppUserModelID(app_id_wide.as_ptr());
            if result == 0 {
                tracing::info!("✓ AppUserModelID set explicitly: {}", app_id);
                eprintln!("[TMC] AppUserModelID set explicitly: {}", app_id);
            } else {
                // Log error but don't block the app (some Windows versions might not support it)
                tracing::warn!(
                    "✗ Failed to set AppUserModelID explicitly: HRESULT 0x{:08X}",
                    result
                );
                tracing::debug!(
                    "This may cause notifications to show AppID instead of DisplayName"
                );
                eprintln!(
                    "[TMC] ERROR: Failed to set AppUserModelID explicitly: HRESULT 0x{:08X}",
                    result
                );
            }
        }
    }

    // Register app for Windows Toast notifications BEFORE everything else
    // This is critical to correctly show name and icon in notifications
    #[cfg(windows)]
    {
        register_app_for_notifications();
    }

    // Check if running with elevated privileges and manage task scheduler
    #[cfg(windows)]
    {
        use crate::system::{is_app_elevated, elevated_task::{create_elevated_task, run_via_elevated_task, elevated_task_exists}};
        let is_elevated = is_app_elevated();
        
        // Load config to check elevation preference
        let config_path = crate::config::get_portable_detector().config_path();
        
        if config_path.exists() {
            if let Ok(config_str) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<crate::config::Config>(&config_str) {
                    if config.request_elevation_on_startup {
                        // First time setup: create elevated task if needed
                        if !elevated_task_exists() {
                            tracing::info!("Creating elevated task for admin access...");
                            if let Err(e) = create_elevated_task() {
                                tracing::error!("Failed to create elevated task: {}", e);
                            }
                        }
                        
                        // If not elevated, run via task scheduler
                        if !is_elevated {
                            tracing::info!("Running via elevated task...");
                            if let Err(e) = run_via_elevated_task() {
                                tracing::error!("Failed to run via elevated task: {}", e);
                            }
                        }
                    }
                }
            }
        }
        
        if is_elevated {
            tracing::info!("Application running with elevated privileges");
        } else {
            tracing::warn!("Application running without elevated privileges - some features may be limited");
        }
    }
    
    // Initialize advanced optimization features
    tracing::warn!("Initializing advanced optimization features");
    if let Err(e) = crate::memory::advanced::init_advanced_features() {
        tracing::warn!("Failed to initialize advanced features: {}", e);
    }

    // Initialize privileges at startup with retry
    // IMPORTANT: Privileges must be acquired BEFORE first optimization
    // Some privileges might require elevated privileges, but we try anyway
    let mut retry_count = 0;
    let max_retries = 3;
    while retry_count < max_retries {
        match ensure_privileges_initialized() {
            Ok(_) => {
                tracing::info!(
                    "Privileges initialized successfully at startup (attempt {})",
                    retry_count + 1
                );
                break;
            }
            Err(e) => {
                retry_count += 1;
                if retry_count < max_retries {
                    tracing::warn!(
                        "Failed to initialize privileges at startup (attempt {}): {}, retrying...",
                        retry_count,
                        e
                    );
                    std::thread::sleep(std::time::Duration::from_millis(500 * retry_count as u64));
                } else {
                    tracing::warn!(
                        "Failed to initialize privileges at startup after {} attempts: {}",
                        max_retries,
                        e
                    );
                    tracing::warn!(
                        "Privileges will be acquired on-demand during first optimization"
                    );
                }
            }
        }
    }

    // Register app as trusted to reduce antivirus false positives
    #[cfg(windows)]
    if let Err(e) = antivirus::whitelist::register_as_trusted() {
        tracing::debug!("Failed to register as trusted (non-critical): {}", e);
    }

    // Load configuration
    let cfg = Arc::new(Mutex::new(Config::load().unwrap_or_else(|e| {
        tracing::warn!("Failed to load config: {}, using defaults", e);
        Config::default()
    })));
    let engine = Engine::new(cfg.clone());
    let rate_limiter = crate::security::RateLimiter::new(
        100,                                // max 100 requests
        std::time::Duration::from_secs(60), // per minute
    );
    let state = AppState {
        cfg: cfg.clone(),
        engine: engine.clone(),
        translations: crate::commands::TranslationState::default(),
        rate_limiter: Arc::new(Mutex::new(rate_limiter)),
    };

    // Build Tauri v2 app
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new()
            .with_handler(move |app, shortcut, event| {
                if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                    tracing::info!("Hotkey pressed: {}", shortcut.id());

                    // Trigger optimization when hotkey is pressed
                    let app_clone = app.clone();
                    tauri::async_runtime::spawn(async move {
                        // Get current configuration
                        if let Some(state) = app_clone.try_state::<crate::AppState>() {
                            let cfg = state.cfg.clone();
                            let engine = state.engine.clone();

                            // Perform optimization with hotkey reason
                            crate::perform_optimization(
                                app_clone,
                                engine,
                                cfg,
                                crate::memory::types::Reason::Hotkey,
                                true,
                                None
                            ).await;
                        }
                    });
                }
            })
            .build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_positioner::init())
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![
            // Commands from app_info module
            commands::app_info::get_app_info,
            commands::app_info::get_app_version,
            commands::app_info::get_company_name,
            // Commands from config module
            commands::config::cmd_exit,
            commands::config::cmd_get_config,
            commands::config::cmd_save_config,
            commands::config::cmd_complete_setup,
            // Commands from memory module
            commands::memory::cmd_memory_info,
            commands::memory::cmd_list_process_names,
            commands::memory::cmd_get_critical_processes,
            commands::memory::cmd_optimize_async,
            // Commands from memory_stats module
            commands::memory_stats::get_memory_stats,
            commands::memory_stats::save_memory_stats,
            // Commands from system module
            commands::system::cmd_run_on_startup,
            commands::system::cmd_set_always_on_top,
            commands::system::cmd_set_priority,
            commands::system::cmd_restart_with_elevation,
            commands::system::cmd_manage_elevated_task,
            // Commands from theme module
            commands::theme::cmd_get_system_theme,
            commands::theme::cmd_get_system_language,
            // Commands from ui module
            commands::ui::cmd_show_or_create_window,
            commands::ui::cmd_show_notification,
            // Commands from i18n module
            commands::i18n::cmd_set_translations,
            // Commands from hotkeys module
            cmd_register_hotkey
        ])
        .setup(move |app| {
            let app_handle = app.handle();

            // Initial log
            tracing::info!("Application setup started");

            // Ensure main window is visible on startup
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

            // Build tray icon - handle errors without crashing
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

                // Fallback 2: also check if config file exists
                // If file exists but setup_completed is false, it might be an issue
                // In that case, we assume setup has already been done
                if should_show {
                    let config_path = crate::config::get_portable_detector().config_path();
                    if config_path.exists() {
                        // File exists, check if it contains setup_completed
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
                        
                        // Apply rounded corners on Windows 10/11
                        #[cfg(windows)]
                        {
                            // Enable shadow for Windows 11 rounded corners FIRST
                            let _ = crate::system::window::enable_shadow_for_win11(&setup_window);
                            // Then apply rounded corners
                            if let Ok(hwnd) = setup_window.hwnd() {
                                let _ = crate::system::window::set_rounded_corners(hwnd.0 as windows_sys::Win32::Foundation::HWND);
                            }
                        }
                        
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
                // In Tauri v2, we get the window from app parameter using the window from event
                // But we need to check which window emitted the event
                // Check all windows to see which one is closing
                if let Some(setup_window) = app.get_webview_window("setup") {
                    // If setup window exists and is closing, always allow close
                    if let Ok(is_visible) = setup_window.is_visible() {
                        if is_visible {
                            tracing::info!("Setup window close requested, allowing close");
                            // Allow setup to close
                            return;
                        }
                    }
                }

                // Handle main window close
                if let Some(main_window) = app.get_webview_window("main") {
                    if let Ok(cfg) = main_window.app_handle().state::<AppState>().cfg.lock() {
                        if cfg.minimize_to_tray {
                            if let Err(e) = main_window.hide() {
                                tracing::warn!("Failed to hide window: {}", e);
                            }
                            api.prevent_close();
                        } else {
                            // If not minimizing to tray, close app and log shutdown
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
