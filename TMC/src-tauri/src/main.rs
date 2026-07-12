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
/// Tracks whether the application started without administrator elevation.
/// Used by the frontend to show a warning banner.
pub static STARTED_WITHOUT_ELEVATION: AtomicBool = AtomicBool::new(false);
/// Stores the tray icon ID for updates
pub(crate) static TRAY_ICON_ID: Lazy<std::sync::Mutex<Option<String>>> =
    Lazy::new(|| std::sync::Mutex::new(None));

/// Stores the single-instance mutex handle so it can be explicitly closed
/// before launching a new elevated instance (to avoid blocking the new process).
#[cfg(windows)]
static SINGLE_INSTANCE_MUTEX_HANDLE: parking_lot::Mutex<Option<usize>> =
    parking_lot::Mutex::new(None);

/// Explicitly close the single-instance mutex handle.
///
/// This MUST be called BEFORE launching a new elevated process (via `ShellExecuteW("runas")`
/// or `schtasks /run`) so the new instance can successfully acquire the mutex.
/// Without this, the new instance would see `ERROR_ALREADY_EXISTS` and self-terminate.
#[cfg(windows)]
fn close_single_instance_mutex() {
    use windows_sys::Win32::Foundation::CloseHandle;
    let mut guard = SINGLE_INSTANCE_MUTEX_HANDLE.lock();
    if let Some(handle_val) = guard.take() {
        unsafe { CloseHandle(handle_val as _) };
        tracing::info!("Single-instance mutex handle closed to allow new instance");
    }
}

/// Application state shared across Tauri commands
#[derive(Clone)]
struct AppState {
    cfg: Arc<Mutex<Config>>,
    engine: Engine,
    translations: crate::commands::TranslationState,
    rate_limiter: Arc<Mutex<crate::security::RateLimiter>>,
    registered_hotkey: Arc<Mutex<Option<String>>>,
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

/// Re-create the single-instance mutex after a failed elevation attempt.
///
/// `close_single_instance_mutex()` must be called before launching an elevated
/// instance; if that launch fails (e.g., the user declines the UAC prompt),
/// this restores single-instance protection for the still-running process.
#[cfg(windows)]
fn reacquire_single_instance_mutex() {
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;

    let mut guard = SINGLE_INSTANCE_MUTEX_HANDLE.lock();
    if guard.is_some() {
        return; // Already held
    }

    // Same name/fallback strategy as the acquisition in main()
    let global_name = to_wide("Global\\TommyMemoryCleaner_SingleInstance");
    let local_name = to_wide("TommyMemoryCleaner_SingleInstance");

    let handle = unsafe { CreateMutexW(std::ptr::null_mut(), 1, global_name.as_ptr()) };
    let handle = if handle.is_null() && unsafe { GetLastError() } == ERROR_ACCESS_DENIED {
        unsafe { CreateMutexW(std::ptr::null_mut(), 1, local_name.as_ptr()) }
    } else {
        handle
    };

    if !handle.is_null() {
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS as u32 {
            // An elevated instance came up despite the reported failure;
            // it now owns single-instance protection.
            tracing::warn!("Mutex already exists while reacquiring - an elevated instance may be running");
        }
        *guard = Some(handle as usize);
        tracing::info!("Single-instance mutex reacquired after failed elevation");
    } else {
        tracing::error!(
            "Failed to reacquire single-instance mutex: GetLastError={}",
            unsafe { GetLastError() }
        );
    }
}

/// Launch a new instance of this executable with administrator privileges
/// via the ShellExecuteW "runas" verb (triggers a UAC consent prompt).
///
/// The caller MUST close the single-instance mutex first, and is responsible
/// for exiting the current process on success (or reacquiring the mutex on
/// failure). Returns the sentinel error "cancelled" when the user declines
/// the UAC prompt so callers can treat that case gracefully.
#[cfg(windows)]
fn launch_elevated_instance() -> Result<(), String> {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;

    /// Set by ShellExecuteW when the user declines the UAC consent dialog
    const ERROR_CANCELLED: u32 = 1223;

    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;

    // Keep the wide strings alive for the duration of the call
    let runas = to_wide("runas");
    let exe_wide = to_wide(&current_exe.to_string_lossy());
    // Give the elevated instance a sane working directory (the exe's folder)
    // instead of inheriting whatever CWD the launcher had
    let dir_wide = current_exe
        .parent()
        .map(|d| to_wide(&d.to_string_lossy()));

    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(), // HWND null
            runas.as_ptr(),
            exe_wide.as_ptr(),
            std::ptr::null(), // no parameters
            dir_wide
                .as_ref()
                .map_or(std::ptr::null(), |d| d.as_ptr()),
            1, // SW_SHOWNORMAL
        )
    };

    // ShellExecuteW returns a value > 32 on success
    if (result as isize) > 32 {
        return Ok(());
    }

    let error_code = unsafe { GetLastError() };
    if error_code == ERROR_CANCELLED {
        tracing::warn!("User declined the UAC elevation prompt");
        Err("cancelled".to_string())
    } else {
        tracing::error!(
            "ShellExecuteW(runas) failed: return={}, GetLastError={}",
            result as isize,
            error_code
        );
        Err(format!(
            "ShellExecuteW failed (return: {}, error: {})",
            result as isize, error_code
        ))
    }
}

/// Restart the application with elevated privileges
///
/// Launches a new admin instance via ShellExecuteW "runas", then gracefully
/// exits the current process through Tauri's shutdown mechanism so all
/// cleanup hooks run. On failure (including a declined UAC prompt) the
/// single-instance mutex is reacquired and the current process keeps running.
#[cfg(windows)]
fn restart_with_elevation(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Restarting application with elevated privileges...");

    // Close the single-instance mutex BEFORE launching the new elevated process.
    // If we don't, the new instance will see ERROR_ALREADY_EXISTS and self-terminate.
    close_single_instance_mutex();

    match launch_elevated_instance() {
        Ok(()) => {
            // Graceful shutdown via Tauri — runs cleanup hooks, flushes files, closes windows
            app.exit(0);
            Ok(())
        }
        Err(e) => {
            // The elevated instance did not start — restore single-instance
            // protection so this process keeps behaving correctly.
            reacquire_single_instance_mutex();
            if e == "cancelled" {
                Err("Elevation cancelled by user".into())
            } else {
                Err(format!("Failed to restart with elevation: {}", e).into())
            }
        }
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
    // Atomically acquire the lock and install the release guard in a single expression.
    // The compare_exchange runs as an argument to scopeguard::guard, so the guard is
    // constructed in the same statement that acquires the lock. This eliminates any gap
    // between "lock acquired" and "guard installed" — a panic at any point after the
    // flag is set will correctly release it via the guard's Drop implementation.
    // The guard's inner value tracks whether we actually acquired the lock so the
    // cleanup closure only resets the flag when appropriate.
    let _guard = scopeguard::guard(
        OPTIMIZATION_RUNNING
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok(),
        |acquired| {
            if acquired {
                OPTIMIZATION_RUNNING.store(false, Ordering::SeqCst);
            }
        },
    );

    if !*_guard {
        tracing::info!("Optimization already running, skipping");
        return;
    }

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

    let (areas, _show_notif, profile, _language) = {
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

    // FIX: Verify notification setting (reload from disk to be sure)
    let show_notif = {
        // Force reload config to pick up changes from Setup
        match crate::config::Config::load() {
            Ok(loaded) => loaded.show_opt_notifications,
            Err(_) => {
                // Fallback to memory if load fails
                if let Ok(guard) = cfg.lock() {
                    guard.show_opt_notifications
                } else {
                    true
                }
            }
        }
    };

    // Debug log to verify logic
    tracing::info!("Notification check: show_settings={}, reason={:?}", show_notif, reason);

    // Check if notifications are globally disabled for this reason
    if !show_notif && reason != Reason::Manual {
        tracing::debug!("Notifications disabled in config, suppressing");
        // Only suppress if NOT manual (user clicked Optimize Now)
        return; 
    } else if show_notif || reason == Reason::Manual {
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

                // Single-pass format substitution to prevent chain-injection
                let mut body = String::with_capacity(body_template.len());
                let mut chars = body_template.chars().peekable();
                while let Some(c) = chars.next() {
                    if c == '%' {
                        let remaining: String = chars.clone().take(3).collect();
                        if remaining.starts_with(".1f") {
                            body.push_str(&format!("{:.1}", freed_mb.abs()));
                            chars.next(); chars.next(); chars.next();
                        } else if remaining.starts_with(".2f") {
                            body.push_str(&format!("{:.2}", free_gb));
                            chars.next(); chars.next(); chars.next();
                        } else if remaining.starts_with('s') {
                            body.push_str(&profile_name);
                            chars.next();
                        } else {
                            body.push(c);
                        }
                    } else {
                        body.push(c);
                    }
                }

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
            // NOTE: Do NOT register on_window_event here — it accumulates handlers
            // every time show_tray_menu_with_retry is called, causing an IPC infinite
            // loop (Bug 6). The focus-loss handler is registered only once when the
            // window is first created (see the "create new window" branch below),
            // and the frontend closeMenu() in tray.ts also handles focus loss.

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

            // Position before showing (avoids flicker)
            position_tray_menu(&menu_win);

            // Small delay to make sure positioning has completed
            tokio::time::sleep(Duration::from_millis(50)).await;

            // Show the menu with retry
            match menu_win.show() {
                Ok(_) => {
                    tracing::info!("Tray menu shown successfully (attempt {})", attempt);

                    // Emit event globally to trigger config reload in frontend
                    let _ = app.emit("tray-menu-open", ());

                    // ⭐ REQUIRED: set focus so the window receives focus events on Windows
                    if let Err(e) = menu_win.set_focus() {
                        tracing::warn!("Failed to set focus on tray menu: {:?}", e);
                    }

                    // Verify that it is actually visible
                    tokio::time::sleep(Duration::from_millis(100)).await;

                    if let Ok(is_visible) = menu_win.is_visible() {
                        if is_visible {
                            // Call loadConfig to apply theme and colors
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
            // Window does not exist, create it
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
            .focused(true)  // ⭐ REQUIRED on Windows to receive focus events
            .build()
            {
                Ok(menu_win) => {
                    tracing::info!(
                        "Tray menu window created successfully (attempt {})",
                        attempt
                    );

                    // ⭐ Handle focus loss to auto-close menu (registered only once at creation)
                    // Uses an AtomicBool guard to prevent re-entrant hide() calls that would
                    // create an IPC infinite loop with the frontend closeMenu() (Bug 6 fix).
                    let menu_win_clone = menu_win.clone();
                    let is_hiding = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                    let is_hiding_clone = is_hiding.clone();
                    menu_win.on_window_event(move |event| {
                        match event {
                            tauri::WindowEvent::Focused(false) => {
                                // Guard: if already hiding, skip to break the feedback loop
                                if is_hiding_clone
                                    .compare_exchange(
                                        false,
                                        true,
                                        std::sync::atomic::Ordering::SeqCst,
                                        std::sync::atomic::Ordering::SeqCst,
                                    )
                                    .is_ok()
                                {
                                    tracing::debug!("Tray menu lost focus, hiding...");
                                    let _ = menu_win_clone.hide();
                                    // Release guard after a short delay
                                    let is_hiding_release = is_hiding_clone.clone();
                                    std::thread::spawn(move || {
                                        std::thread::sleep(std::time::Duration::from_millis(200));
                                        is_hiding_release.store(
                                            false,
                                            std::sync::atomic::Ordering::SeqCst,
                                        );
                                    });
                                }
                            }
                            _ => {}
                        }
                    });

                    // Position before showing
                    position_tray_menu(&menu_win);

                    // Small delay to make sure positioning has completed
                    tokio::time::sleep(Duration::from_millis(50)).await;

                    // Show the window
                    match menu_win.show() {
                        Ok(_) => {
                            tracing::info!(
                                "Newly created tray menu shown successfully (attempt {})",
                                attempt
                            );
                            // Emit event globally to trigger config reload in frontend
                            let _ = app.emit("tray-menu-open", ());

                            // ⭐ REQUIRED: set focus so the window receives focus events on Windows
                            if let Err(e) = menu_win.set_focus() {
                                tracing::warn!(
                                    "Failed to set focus on newly created tray menu: {:?}",
                                    e
                                );
                            }

                            // Verify that it is actually visible
                            tokio::time::sleep(Duration::from_millis(100)).await;

                            if let Ok(is_visible) = menu_win.is_visible() {
                                if is_visible {
                                    // Call loadConfig to apply theme and colors
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
                        true // WebView2 not found
                    } else {
                        false // WebView2 found
                    }
                }
                Err(_) => true, // Error: treat WebView2 as not found
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

    // ===== SINGLE-INSTANCE MUTEX (Bug 10 fix) =====
    // Create a named Windows mutex to prevent multiple instances from running simultaneously.
    // This must be done early, before any heavy initialization, to avoid conflicting operations
    // when multiple startup mechanisms (Shortcut, Elevated Task, Registry) fire concurrently.
    // The mutex handle is stored in a static so it can be explicitly closed before launching
    // a new elevated instance (via ShellExecuteW "runas" or schtasks /run).
    #[cfg(windows)]
    let _single_instance_mutex = {
        use windows_sys::Win32::System::Threading::CreateMutexW;
        use windows_sys::Win32::Foundation::{GetLastError, CloseHandle, ERROR_ALREADY_EXISTS, ERROR_ACCESS_DENIED};
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK, MB_ICONWARNING};

        // Try Global\ prefix first (works across sessions when running elevated).
        // If that fails with ERROR_ACCESS_DENIED (standard user without SeCreateGlobalPrivilege),
        // fall back to a session-local mutex name (bare name without Global\ prefix).
        let global_name = to_wide("Global\\TommyMemoryCleaner_SingleInstance");
        let local_name = to_wide("TommyMemoryCleaner_SingleInstance");

        let handle = unsafe {
            CreateMutexW(std::ptr::null_mut(), 1, global_name.as_ptr())
        };

        let handle = if handle.is_null() && unsafe { GetLastError() } == ERROR_ACCESS_DENIED {
            tracing::warn!(
                "Global\\ mutex creation failed with ERROR_ACCESS_DENIED (standard user). \
                 Falling back to session-local mutex."
            );
            unsafe {
                CreateMutexW(std::ptr::null_mut(), 1, local_name.as_ptr())
            }
        } else {
            handle
        };

        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS as u32 {
            // Another instance is already running — close our handle to the existing mutex
            if !handle.is_null() {
                unsafe { CloseHandle(handle) };
            }

            tracing::warn!("Another instance of Tommy Memory Cleaner is already running. Exiting.");

            let title = to_wide("Tommy Memory Cleaner");
            let msg = to_wide(
                "Another instance of Tommy Memory Cleaner is already running.\n\n\
                 The application will now exit.",
            );
            unsafe {
                MessageBoxW(
                    std::ptr::null_mut(),
                    msg.as_ptr(),
                    title.as_ptr(),
                    MB_OK | MB_ICONWARNING,
                );
            }

            std::process::exit(0);
        }

        if handle.is_null() {
            // CreateMutexW failed for an unexpected reason — log but continue
            tracing::error!(
                "Failed to create single-instance mutex: GetLastError={}",
                unsafe { GetLastError() }
            );
        } else {
            // Store the handle in the static so it can be explicitly closed
            // before launching a new elevated instance.
            *SINGLE_INSTANCE_MUTEX_HANDLE.lock() = Some(handle as usize);
        }

        // Return the handle so it stays alive for the app's lifetime (auto-released on exit)
        handle
    };

    // Check if running with elevated privileges and manage task scheduler
    #[cfg(windows)]
    {
        use crate::system::{is_app_elevated, elevated_task::{create_elevated_task, run_via_elevated_task, elevated_task_matches_current_exe}};
        let is_elevated = is_app_elevated();

        // Load config to check elevation preference
        let config_path = crate::config::get_portable_detector().config_path();

        if config_path.exists() {
            if let Ok(config_str) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<crate::config::Config>(&config_str) {
                    if config.request_elevation_on_startup {
                        // Create (or repair) the elevated task only when running as admin
                        // (schtasks requires elevation for /rl highest). create_elevated_task
                        // deletes any existing task first, so a stale exe path is also fixed here.
                        if is_elevated && !elevated_task_matches_current_exe() {
                            tracing::info!("Creating elevated task for silent admin startup...");
                            if let Err(e) = create_elevated_task() {
                                tracing::error!("Failed to create elevated task: {}", e);
                            }
                        }

                        // Not elevated: self-elevate. Prefer the silent scheduled task;
                        // fall back to an explicit UAC prompt (ShellExecuteW "runas").
                        if !is_elevated {
                            // 1) Silent path: only usable if the task exists AND points at
                            //    this exe. Running a stale task "succeeds" per schtasks but
                            //    launches nothing, which previously made the app vanish
                            //    at startup without ever elevating.
                            if elevated_task_matches_current_exe() {
                                tracing::info!("Elevating silently via scheduled task...");
                                // Close the single-instance mutex BEFORE launching the
                                // elevated task so the new instance can acquire it.
                                close_single_instance_mutex();
                                match run_via_elevated_task() {
                                    Ok(true) => {
                                        // Elevated task triggered — exit gracefully by returning
                                        // from main() so Rust destructors run.
                                        tracing::info!("Elevated task launched, exiting current process gracefully");
                                        logging::shutdown();
                                        return;
                                    }
                                    Ok(false) => {
                                        // No exit needed — restore single-instance protection
                                        reacquire_single_instance_mutex();
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to run via elevated task: {}", e);
                                        reacquire_single_instance_mutex();
                                    }
                                }
                            }

                            // 2) UAC fallback: first run (task not created yet), stale task,
                            //    or the task trigger failed. Once the elevated instance is
                            //    up it creates the task, so this prompt appears only once.
                            tracing::info!("Elevating via UAC prompt (scheduled task unavailable)...");
                            close_single_instance_mutex();
                            match launch_elevated_instance() {
                                Ok(()) => {
                                    tracing::info!("Elevated instance launched via UAC, exiting current process gracefully");
                                    logging::shutdown();
                                    return;
                                }
                                Err(e) => {
                                    reacquire_single_instance_mutex();
                                    if e == "cancelled" {
                                        tracing::warn!("User declined UAC prompt - continuing without elevation");
                                    } else {
                                        tracing::error!("UAC elevation failed: {} - continuing without elevation", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        if is_elevated {
            tracing::info!("Application running with elevated privileges");
            STARTED_WITHOUT_ELEVATION.store(false, Ordering::SeqCst);
        } else {
            tracing::warn!("Application running without elevated privileges - some features may be limited");
            STARTED_WITHOUT_ELEVATION.store(true, Ordering::SeqCst);
        }
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
        registered_hotkey: Arc::new(Mutex::new(None)),
    };

    // DPI Awareness for Windows - Fix blurry edges on high DPI
    #[cfg(target_os = "windows")]
    {
        unsafe {
            use windows_sys::Win32::UI::HiDpi::SetProcessDpiAwareness;
            SetProcessDpiAwareness(2); // PROCESS_PER_MONITOR_DPI_AWARE
        }
    }

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
            commands::ui::cmd_get_window_config,
            commands::ui::cmd_get_platform,
            commands::ui::cmd_apply_rounded_corners,
            commands::ui::cmd_update_tray_theme,
            commands::ui::cmd_check_elevation,
            commands::ui::cmd_is_elevation_required,
            // Commands from i18n module
            commands::i18n::cmd_set_translations,
            // Commands from hotkeys module
            cmd_register_hotkey
        ])
        .setup(move |app| {
            let app_handle = app.handle();

            // Initial log
            tracing::info!("Application setup started");

            // Check if this is first run - if so, don't show main window yet
            let is_first_run = {
                if let Ok(cfg) = state.cfg.try_lock() {
                    !cfg.setup_completed
                } else {
                    false
                }
            };

            // Only show main window if setup is already completed
            if !is_first_run {
                if let Some(window) = app_handle.get_webview_window("main") {
                    tracing::info!("Setup already completed, showing main window...");
                    let _ = window.set_skip_taskbar(false);
                    if let Err(e) = window.show() {
                        tracing::error!("Failed to show window: {:?}", e);
                    }
                }
            } else {
                tracing::info!("First run detected - main window will be shown after setup");
            }

            // Build tray icon - handle errors without crashing
            // NOTE: During first run (setup), we build the tray but delay activation
            let mut tray_builder = match ui::tray::build(app_handle) {
                Ok(builder) => {
                    tracing::info!("Tray icon builder created successfully");
                    builder
                }
                Err(e) => {
                    tracing::error!("Failed to build tray icon: {:?}", e);
                    // Wrap the error; the app cannot continue building the tray icon
                    return Err(Box::new(e) as Box<dyn std::error::Error>);
                }
            };

            // FIX: Removed the incorrect explicit type; let Rust infer the types.
            // Check is_first_run to prevent tray actions during setup
            let is_first_run_for_tray = is_first_run;
            tray_builder = tray_builder.on_tray_icon_event(move |tray, event| {
                // During first run (setup), ignore tray clicks
                if is_first_run_for_tray {
                    // Check if setup is now completed by looking for main window
                    let app = tray.app_handle();
                    if let Some(state) = app.try_state::<AppState>() {
                        if let Ok(cfg) = state.cfg.try_lock() {
                            if !cfg.setup_completed {
                                tracing::debug!("Ignoring tray click during setup");
                                return;
                            }
                        }
                    }
                }
                
                // Hook up the positioner plugin
                tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);

                match event {
                    tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } => {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            // FIX: Handle the Result to avoid type errors
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

                        // Use the async runtime to open the menu without blocking
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

            // Store the ID so it can be used in tray.rs
            let tray_id = tray.id().0.clone();
            if let Ok(mut id) = TRAY_ICON_ID.lock() {
                *id = Some(tray_id.clone());
            }

            // FIX: Prefix unused variables with _ to silence warnings
            let _cfg_for_setup = cfg.clone();

            // FIX: Check whether the app was launched with --startup-config by the installer
            let args: Vec<String> = std::env::args().collect();
            let is_startup_config = args.iter().any(|a| a == "--startup-config");

            if is_startup_config {
                // Configure run-on-startup when requested by the installer
                let _ = crate::system::startup::set_run_on_startup(true);
                if let Ok(mut c) = _cfg_for_setup.lock() {
                    c.run_on_startup = true;
                    let _ = c.save();
                }
                // Graceful shutdown via Tauri — runs cleanup hooks, flushes files, closes windows
                app_handle.exit(0);
                return Ok(());
            }

            // ⭐ Check whether this is the first run and show the setup.
            // Also verify the config file exists to avoid launching multiple setups.
            let show_setup = {
                // ⭐ Fallback 1: check whether the setup window is already open
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
                // Hide the main window
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.hide();
                }

                // Create and show the setup window
                tracing::info!("First run detected, showing setup window...");
                let setup_url = WebviewUrl::App("setup.html".into());
                let app_clone = app_handle.clone();
                match WebviewWindowBuilder::new(&app_clone, "setup", setup_url)
                    .title("Tommy Memory Cleaner - Setup")
                    .inner_size(500.0, 600.0)
                    .min_inner_size(380.0, 500.0)
                    .max_inner_size(500.0, 600.0)
                    .resizable(false)
                    .decorations(false)
                    .transparent(true)
                    .shadow(false)
                    .skip_taskbar(false)
                    .always_on_top(true)
                    .visible(false)  // Show window only after customizations
                    .build()
                {
                    Ok(setup_window) => {
                        tracing::info!("Setup window created successfully");
                        // Center the setup window
                        let _ = setup_window.center();
                        
                        // Ensure it always stays on top
                        let _ = setup_window.set_always_on_top(true);
                        
                        // Apply rounded corners on Windows 10/11
                        #[cfg(windows)]
                        {
                            // Enable shadow for Windows 11
                            let _ = crate::system::window::enable_shadow_for_win11(&setup_window);
                            // Apply DWM attributes
                            if let Ok(hwnd) = setup_window.hwnd() {
                                let _ = crate::system::window::set_rounded_corners(hwnd.0 as windows_sys::Win32::Foundation::HWND);
                            }
                        }
                        
                        // Show window after customizations are applied
                        // This prevents the "XP bar" flash on Windows 10
                        tracing::info!("Showing setup window after applying styles");
                        let _ = setup_window.show();
                        
                        let _ = setup_window.set_focus();
                        
                        // Re-apply always_on_top after a short delay, to be safe
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
                        // Fallback: show the main window
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                        }
                    }
                }
            } else {
                // Show the window at startup - use app_handle instead of app
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
                    
                    // CRITICAL FIX: Apply rounded corners on Windows 10/11 at startup
                    // This ensures borders are applied even when setup is already completed
                    #[cfg(windows)]
                    {
                        tracing::info!("Applying window decorations at startup (setup already completed)");
                        let _ = crate::system::window::apply_window_decorations(&window);
                    }
                    
                    // FIX: Enable devtools for debugging (right-click -> Inspect)
                    #[cfg(debug_assertions)]
                    {
                        let _ = window.open_devtools();
                    }
                } else {
                    // If the window does not exist, create it
                    tracing::warn!("Main window not found, creating it...");
                    show_or_create_window(&app_handle);
                    // Verify that it was created
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

            // Tray menu updates (Tauri v2 - handled by the builder)

            // Apply initial configurations
            if let Ok(c) = _cfg_for_setup.lock() {
                // Startup
                if c.run_on_startup && !crate::system::startup::is_startup_enabled() {
                    let _ = crate::system::startup::set_run_on_startup(true);
                }

                // Registering the app for Windows Toast notifications (required for unpackaged apps)
                // IMPORTANT: must be called BEFORE any notification is sent.
                // Notification registration already happened at startup in main().

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

            // Start background threads ONLY if setup is already completed
            // During first run, these will be started after setup completes via event
            if !is_first_run {
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
            } else {
                tracing::info!("First run: background processes delayed until setup completion");
            }

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
