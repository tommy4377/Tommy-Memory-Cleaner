/// UI-related commands for window management and notifications.
///
/// This module provides Tauri commands for showing windows,
/// displaying notifications, and positioning UI elements.
use tauri::{AppHandle, Manager, State};

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
pub fn show_or_create_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        tracing::info!("Found existing main window");
        if let Ok(size) = window.inner_size() {
            tracing::info!("Current window size: {}x{}", size.width, size.height);
        }
        
        // Reapply rounded corners when showing existing window
        #[cfg(windows)]
        {
            tracing::info!("Reapplying rounded corners to existing window");
            // PRIMA: Applica i bordi arrotondati
            if let Ok(hwnd) = window.hwnd() {
                let _ = crate::system::window::set_rounded_corners(
                    hwnd.0 as windows_sys::Win32::Foundation::HWND
                );
            }
            // DOPO: Applica shadow per Win11
            let _ = crate::system::window::enable_shadow_for_win11(&window);
        }
        
        let _: Result<(), _> = window.set_skip_taskbar(false); // Show in taskbar
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = window.center();
    } else {
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
        .center()
        .skip_taskbar(false)  // Show in taskbar
        .visible(true)  // Show window immediately for SetWindowRgn
        .build();

        match result {
            Ok(window) => {
                tracing::info!("Window created successfully");
                
                // Apply rounded corners on Windows 10/11
                #[cfg(windows)]
                {
                    // PRIMA: Applica i bordi arrotondati
                    if let Ok(hwnd) = window.hwnd() {
                        let _ = crate::system::window::set_rounded_corners(hwnd.0 as windows_sys::Win32::Foundation::HWND);
                    }
                    
                    // DOPO: Applica shadow per Win11
                    let _ = crate::system::window::enable_shadow_for_win11(&window);
                }
                
                if let Ok(size) = window.inner_size() {
                    tracing::info!("Actual window size: {}x{}", size.width, size.height);
                }
                let _ = window.set_skip_taskbar(false);
                let _ = window.set_focus();
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

    // Get all available monitors and find the one containing the cursor
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
            found_monitor.unwrap_or_else(|| {
                tracing::warn!("Cursor not found in any monitor, using primary monitor");
                window
                    .primary_monitor()
                    .ok()
                    .flatten()
                    .expect("No primary monitor available")
            })
        }
        Err(e) => {
            tracing::error!("Failed to get available monitors: {:?}", e);
            // Fallback: use current_monitor or primary_monitor directly
            match window
                .current_monitor()
                .ok()
                .flatten()
                .or_else(|| window.primary_monitor().ok().flatten())
            {
                Some(m) => {
                    tracing::warn!("Using fallback monitor (current or primary)");
                    m
                }
                None => {
                    tracing::error!("No monitors available");
                    return;
                }
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
