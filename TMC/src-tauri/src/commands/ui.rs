/// UI-related commands for window management and notifications.
///
/// This module provides Tauri commands for showing windows,
/// displaying notifications, and positioning UI elements.
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager, State};

/// Atomic guard to prevent concurrent window creation races.
static WINDOW_CREATING: AtomicBool = AtomicBool::new(false);

/// Returns the window configuration values including border radius.
///
/// This command exposes the window styling values to the frontend
/// so they can be synchronized dynamically instead of being hardcoded.
///
/// The border radius is platform-aware:
/// - Windows 11: 0 — the OS rounds the window natively via DWM
///   (DWMWCP_ROUND), so the CSS must NOT round the content again
///   (double rounding causes curvature mismatch and corner artifacts).
/// - Windows 10 (and anything else): 16 — the rounded shape is drawn
///   entirely in CSS on the transparent window.
#[tauri::command]
pub fn cmd_get_window_config() -> Result<serde_json::Value, String> {
    // Windows 10: must match CORNER_RADIUS_PX in system/window.rs so the CSS
    // edge and the GDI clip region coincide. Windows 11 rounds natively (0).
    #[cfg(windows)]
    let border_radius = if crate::os::is_windows_11() { 0 } else { 12 };
    #[cfg(not(windows))]
    let border_radius = 12;

    Ok(serde_json::json!({
        "border_radius": border_radius,
        "titlebar_height": 32
    }))
}

/// Returns the current platform information.
///
/// This command allows the frontend to detect the specific OS version
/// to apply platform-specific styling (e.g., Windows 10 rounded corners).
/// Uses centralized RtlGetVersion-based detection for accuracy.
#[tauri::command]
pub fn cmd_get_platform() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        if crate::os::is_windows_11() {
            Ok("windows-11".to_string())
        } else if crate::os::is_windows_10() {
            Ok("windows-10".to_string())
        } else {
            Ok("windows".to_string())
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    Ok("other".to_string())
}

/// Update tray icon with current theme
#[tauri::command]
pub fn cmd_update_tray_theme(app: AppHandle, theme: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        let _ = crate::ui::tray::update_tray_icon_with_theme(&app, &theme);
    }
    Ok(())
}

/// Re-assert the platform-appropriate corner decorations on the main window.
///
/// Idempotent and cheap: on Windows 11 it re-applies the persistent DWM corner
/// preference; on Windows 10 it rebuilds the rounded GDI clip region for the
/// current window size. Resizes are additionally handled automatically by the
/// global window-event handler in main.rs.
#[tauri::command]
pub fn cmd_apply_rounded_corners(app: AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        if let Some(window) = app.get_webview_window("main") {
            let _ = crate::system::window::apply_window_decorations(&window);
        }
    }

    Ok(())
}

/// Shows the main window or creates it if it doesn't exist.
///
/// This command delegates to the helper function to handle both
/// showing existing windows and creating new ones if needed.
#[tauri::command]
pub fn cmd_show_or_create_window(app: AppHandle) {
    crate::show_or_create_window(&app);
}

/// Displays a system notification with the specified title and message.
///
/// Uses the current theme from configuration to style the notification.
/// Falls back to dark theme if configuration is unavailable.
///
/// # Arguments
///
/// * `app` - The application handle for displaying notifications
/// * `title` - The notification title
/// * `message` - The notification message
/// * `state` - The application state containing the configuration
///
/// # Returns
///
/// Returns `Ok(())` if the notification is displayed successfully,
/// or an error string if the operation fails.
#[tauri::command]
pub fn cmd_show_notification(
    app: AppHandle,
    title: String,
    message: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    // Get the current theme from configuration
    let theme = {
        match state.cfg.try_lock() {
            Ok(cfg_guard) => cfg_guard.theme.clone(),
            Err(_) => {
                tracing::debug!("Config lock busy in cmd_show_notification, using default theme");
                "dark".to_string()
            }
        }
    };
    // Use the notifications module function
    crate::notifications::show_windows_notification(&app, &title, &message, &theme)
}

/// Helper function to show or create the main application window.
///
/// This function is accessible from main.rs and handles both
/// showing existing windows and creating new ones if needed.
/// Uses a check-then-create pattern with verification to prevent race conditions.
pub fn show_or_create_window(app: &AppHandle) {
    // Check if window exists
    if let Some(window) = app.get_webview_window("main") {
        tracing::info!("Found existing main window");
        if let Ok(size) = window.inner_size() {
            tracing::info!("Current window size: {}x{}", size.width, size.height);
        }
        
        // Re-assert decorations once, BEFORE the window becomes visible,
        // so no frame is ever presented with square corners
        #[cfg(windows)]
        {
            tracing::info!("Reapplying window decorations to existing window");
            let _ = crate::system::window::apply_window_decorations(&window);
        }

        let _: Result<(), _> = window.set_skip_taskbar(false); // Show in taskbar
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = window.center();
    } else {
        // Use compare_exchange to prevent concurrent window creation races
        if WINDOW_CREATING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            tracing::warn!("Window creation already in progress, skipping duplicate request");
            return;
        }

        tracing::info!("Creating new main window...");
        tracing::info!("Window dimensions will be: 500x700");
        let result = tauri::WebviewWindowBuilder::new(
            app,
            "main",
            tauri::WebviewUrl::App("index.html".into())
        )
        .title("Tommy Memory Cleaner")
        .inner_size(500.0, 700.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)  // Off by default; enabled on Win11 by apply_window_decorations
        .skip_taskbar(false)  // Show in taskbar
        .visible(false)  // Keep hidden until decorations are applied (prevents square-corner flash)
        .build();

        // Release the guard immediately after build, regardless of outcome
        WINDOW_CREATING.store(false, Ordering::SeqCst);

        match result {
            Ok(window) => {
                tracing::info!("Window created successfully");

                // Center and decorate while still hidden, then show:
                // the first presented frame already has the correct corners
                let _ = window.center();

                #[cfg(windows)]
                {
                    let _ = crate::system::window::apply_window_decorations(&window);
                }

                let _ = window.show();

                if let Ok(size) = window.inner_size() {
                    tracing::info!("Actual window size: {}x{}", size.width, size.height);
                }
                let _ = window.set_skip_taskbar(false);
                let _ = window.set_focus();
                
                // Verify window was actually created and is accessible
                if let Some(_created_window) = app.get_webview_window("main") {
                    tracing::info!("✓ Window creation verified");
                } else {
                    tracing::error!("Window creation verification failed - window not found immediately after creation");
                }
            }
            Err(e) => {
                tracing::error!("Failed to create window: {:?}", e);
                eprintln!("FATAL ERROR: Failed to create window: {:?}", e);
            }
        }
    }
}

/// Positions the tray menu relative to the system tray icon.
///
/// This function calculates the optimal position for the tray menu
/// based on the cursor position and taskbar location.
///
/// # Arguments
///
/// * `window` - The tray menu window to position
pub fn position_tray_menu(window: &tauri::WebviewWindow) {
    // Get the menu dimensions
    let menu_size = match window.outer_size() {
        Ok(size) => size,
        Err(e) => {
            tracing::error!("Failed to get menu size: {:?}", e);
            return;
        }
    };

    let menu_width = menu_size.width as i32;
    let menu_height = menu_size.height as i32;

    // FIX: Get cursor position FIRST (near tray icon)
    let cursor_pos = match window.cursor_position() {
        Ok(pos) => pos,
        Err(_) => {
            tracing::error!("Failed to get cursor position");
            // Fallback: use primary monitor
            if let Ok(Some(monitor)) = window.primary_monitor() {
                let monitor_size = monitor.size();
                let monitor_pos = monitor.position();
                let fallback_pos = tauri::PhysicalPosition {
                    x: (monitor_pos.x + monitor_size.width as i32 - 50) as f64,
                    y: (monitor_pos.y + monitor_size.height as i32 - 50) as f64,
                };
                tracing::warn!("Using fallback cursor position: {:?}", fallback_pos);
                fallback_pos
            } else {
                tracing::error!("Failed to get primary monitor for fallback");
                return;
            }
        }
    };

    // FIX: Find monitor containing cursor (not the window's monitor)
    let cursor_x = cursor_pos.x as i32;
    let cursor_y = cursor_pos.y as i32;

    let monitor = match window.available_monitors() {
        Ok(monitors) => {
            // Find monitor containing the cursor
            let mut found_monitor = None;
            for m in monitors {
                let m_pos = m.position();
                let m_size = m.size();

                let m_left = m_pos.x;
                let m_top = m_pos.y;
                let m_right = m_pos.x + m_size.width as i32;
                let m_bottom = m_pos.y + m_size.height as i32;

                // Check if cursor is inside this monitor
                if cursor_x >= m_left
                    && cursor_x < m_right
                    && cursor_y >= m_top
                    && cursor_y < m_bottom
                {
                    // Log before moving m
                    tracing::debug!(
                        "Found monitor containing cursor: {}x{} at {:?}",
                        m_size.width,
                        m_size.height,
                        m_pos
                    );
                    found_monitor = Some(m);
                    break;
                }
            }

            // If not found, use primary monitor as fallback
            if let Some(m) = found_monitor {
                m
            } else {
                tracing::warn!("Cursor not found in any monitor, using primary monitor");
                if let Some(m) = window.primary_monitor().ok().flatten() {
                    m
                } else {
                    tracing::error!("No primary monitor available");
                    // Return current monitor as last resort
                    if let Some(m) = window.current_monitor().ok().flatten() {
                        m
                    } else {
                        tracing::error!("No current monitor available");
                        // Return a default monitor position
                        return;
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to get available monitors: {:?}", e);
            // Fallback: use current_monitor or primary_monitor directly
            if let Some(m) = window.current_monitor().ok().flatten() {
                m
            } else if let Some(m) = window.primary_monitor().ok().flatten() {
                m
            } else {
                tracing::error!("No monitor available");
                return;
            }
        }
    };

    let monitor_size = monitor.size();
    let monitor_pos = monitor.position();

    tracing::debug!(
        "Cursor position: {:?}, Using monitor: {}x{} at {:?}",
        cursor_pos,
        monitor_size.width,
        monitor_size.height,
        monitor_pos
    );

    // Determine taskbar position
    let (final_x, final_y) = if let Some((
        taskbar_left,
        taskbar_top,
        taskbar_right,
        taskbar_bottom,
    )) = get_taskbar_rect()
    {
        let taskbar_height = taskbar_bottom - taskbar_top;
        let taskbar_width = taskbar_right - taskbar_left;
        let is_taskbar_vertical = taskbar_width < taskbar_height;

        tracing::debug!(
            "Taskbar rect: ({}, {}, {}, {}), vertical: {}",
            taskbar_left,
            taskbar_top,
            taskbar_right,
            taskbar_bottom,
            is_taskbar_vertical
        );

        let cursor_x = cursor_pos.x as i32;
        let cursor_y = cursor_pos.y as i32;

        if is_taskbar_vertical {
            // Vertical taskbar (left or right)
            if taskbar_left < monitor_pos.x + 100 {
                // Taskbar on LEFT - menu to the right of tray
                let x = taskbar_right + 5;
                let y = (cursor_y - menu_height / 2).max(monitor_pos.y + 5);
                (x, y)
            } else {
                // Taskbar on RIGHT - menu to the left of tray
                let x = (taskbar_left - menu_width - 5).max(monitor_pos.x + 5);
                let y = (cursor_y - menu_height / 2).max(monitor_pos.y + 5);
                (x, y)
            }
        } else {
            // Horizontal taskbar (top or bottom)
            // Center menu horizontally relative to cursor
            let x = (cursor_x - menu_width / 2)
                .max(monitor_pos.x + 5)  // Not too far left
                .min(monitor_pos.x + monitor_size.width as i32 - menu_width - 5); // Not too far right

            if taskbar_top < monitor_pos.y + 100 {
                // Taskbar on TOP - menu BELOW taskbar
                let y = taskbar_bottom + 5;
                (x, y)
            } else {
                // Taskbar on BOTTOM - menu ABOVE taskbar
                let y = taskbar_top - menu_height - 5;
                (x, y)
            }
        }
    } else {
        // Fallback: no taskbar info, use safe position
        tracing::warn!("Could not get taskbar rect, using fallback positioning");
        let x = (cursor_pos.x as i32 - menu_width / 2)
            .max(monitor_pos.x + 5)
            .min(monitor_pos.x + monitor_size.width as i32 - menu_width - 5);
        let y =
            (monitor_pos.y + monitor_size.height as i32 - menu_height - 80).max(monitor_pos.y + 5);
        (x, y)
    };

    tracing::info!("Positioning tray menu at: ({}, {})", final_x, final_y);

    // Apply the position
    if let Err(e) = window.set_position(tauri::PhysicalPosition {
        x: final_x,
        y: final_y,
    }) {
        tracing::error!("Failed to set menu position: {:?}", e);
    }
}

/// Retrieves the Windows taskbar rectangle coordinates.
///
/// Returns (left, top, right, bottom) of the taskbar area.
/// Only available on Windows.
#[cfg(windows)]
pub fn get_taskbar_rect() -> Option<(i32, i32, i32, i32)> {
    use std::mem::zeroed;
    use windows_sys::Win32::UI::Shell::{SHAppBarMessage, ABM_GETTASKBARPOS, APPBARDATA};

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

/// Stub implementation for non-Windows platforms.
#[cfg(not(windows))]
fn get_taskbar_rect() -> Option<(i32, i32, i32, i32)> {
    None
}

/// Check if the application is running with administrator privileges.
/// 
/// Returns a JSON object with elevation status and available privileges:
/// - `is_elevated`: boolean - whether the app is running as admin
/// - `privileges`: object - status of required privileges for memory optimization
#[tauri::command]
pub fn cmd_check_elevation() -> Result<serde_json::Value, String> {
    #[cfg(target_os = "windows")]
    {
        let is_elevated = crate::system::is_app_elevated();
        
        // Check if key privileges are available
        let has_debug_priv = crate::memory::privileges::ensure_privilege("SeDebugPrivilege").is_ok();
        let has_quota_priv = crate::memory::privileges::ensure_privilege("SeIncreaseQuotaPrivilege").is_ok();
        let has_profile_priv = crate::memory::privileges::ensure_privilege("SeProfileSingleProcessPrivilege").is_ok();
        
        tracing::info!(
            "Elevation check: is_elevated={}, SeDebugPrivilege={}, SeIncreaseQuotaPrivilege={}, SeProfileSingleProcessPrivilege={}",
            is_elevated,
            has_debug_priv,
            has_quota_priv,
            has_profile_priv
        );
        
        Ok(serde_json::json!({
            "is_elevated": is_elevated,
            "privileges": {
                "SeDebugPrivilege": has_debug_priv,
                "SeIncreaseQuotaPrivilege": has_quota_priv,
                "SeProfileSingleProcessPrivilege": has_profile_priv
            }
        }))
    }
    
    #[cfg(not(target_os = "windows"))]
    Ok(serde_json::json!({
        "is_elevated": true,
        "privileges": {}
    }))
}

/// Check if the application started without administrator privileges.
/// Used by the frontend to show a warning banner on startup.
#[tauri::command]
pub fn cmd_is_elevation_required() -> bool {
    #[cfg(target_os = "windows")]
    {
        crate::STARTED_WITHOUT_ELEVATION.load(std::sync::atomic::Ordering::SeqCst)
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}
