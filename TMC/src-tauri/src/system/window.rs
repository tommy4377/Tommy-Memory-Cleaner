use tauri::{AppHandle, Manager};

pub fn set_always_on_top(app: &AppHandle, on: bool) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.set_always_on_top(on).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Apply rounded corners and shadow to a window (used for both setup and main window)
#[cfg(windows)]
pub fn apply_window_decorations(window: &tauri::WebviewWindow) -> Result<(), String> {
    // WAIT longer for window to be fully rendered
    std::thread::sleep(std::time::Duration::from_millis(300));
    
    // PRIMA: Applica shadow (come nel setup)
    let _ = enable_shadow_for_win11(window);
    
    // DOPO: Applica rounded corners (come nel setup)
    if let Ok(hwnd) = window.hwnd() {
        let _ = set_rounded_corners(hwnd.0 as windows_sys::Win32::Foundation::HWND);
        
        // FORZA RIDISEGNO dopo un breve delay per Windows 10
        std::thread::sleep(std::time::Duration::from_millis(100));
        use windows_sys::Win32::Graphics::Gdi::InvalidateRect;
        unsafe {
            InvalidateRect(hwnd.0 as windows_sys::Win32::Foundation::HWND, std::ptr::null(), 1);
        }
    }
    
    Ok(())
}

/// Show window with rounded corners (centralized function)
pub fn show_window_with_rounded_corners(window: &tauri::WebviewWindow) -> Result<(), String> {
    let _ = window.set_skip_taskbar(false);
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.center();
    let _ = window.set_focus();
    
    // Apply rounded corners on Windows
    #[cfg(windows)]
    {
        let _ = apply_window_decorations(window);
    }
    
    Ok(())
}

#[cfg(windows)]
pub fn set_rounded_corners(hwnd: windows_sys::Win32::Foundation::HWND) -> Result<(), String> {
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE,
    };

    unsafe {
        // Use centralized version detection (RtlGetVersion-based, more reliable than GetVersionExW)
        let is_win11 = crate::os::is_windows_11();

        if is_win11 {
            // Windows 11: Use native DWM rounded corners
            tracing::info!("Windows 11 detected - enabling native DWM rounded corners");

            // DWMWCP_ROUND = 2 (rounded corners)
            let preference: i32 = 2;

            let result = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                &preference as *const _ as *const _,
                std::mem::size_of::<i32>() as u32,
            );

            if result == 0 {
                tracing::info!("✓ Successfully applied native rounded corners (Windows 11)");
            } else {
                tracing::warn!(
                    "Failed to set rounded corners on Windows 11: HRESULT 0x{:08X}",
                    result
                );
            }
        } else {
            // Windows 10: Use region-based approach
            apply_win10_rounded_corners(hwnd);
        }
    }
    Ok(())
}

#[cfg(windows)]
fn apply_win10_rounded_corners(hwnd: windows_sys::Win32::Foundation::HWND) {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn, InvalidateRect};
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
    use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    
    unsafe {
        tracing::info!("Applying region-based rounded corners (Windows 10 method)");
        
        // Get the actual window rect (includes DWM invisible borders)
        let mut window_rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut window_rect) == 0 {
            tracing::warn!("Failed to get window rect");
            return;
        }
        
        let window_width = window_rect.right - window_rect.left;
        let window_height = window_rect.bottom - window_rect.top;
        
        tracing::debug!("Window rect: left={}, top={}, right={}, bottom={} ({}x{})",
            window_rect.left, window_rect.top, window_rect.right, window_rect.bottom,
            window_width, window_height);
        
        // Try to get the EXTENDED_FRAME_BOUNDS (actual visible area without invisible DWM borders)
        let mut extended_frame: RECT = std::mem::zeroed();
        let hr = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            &mut extended_frame as *mut _ as *mut _,
            std::mem::size_of::<RECT>() as u32,
        );
        
        // Calculate the invisible border offset on each side
        let (left_offset, top_offset, right_offset, bottom_offset) = if hr == 0 {
            // DWM reported the actual visible bounds
            let left_off = extended_frame.left - window_rect.left;
            let top_off = extended_frame.top - window_rect.top;
            let right_off = window_rect.right - extended_frame.right;
            let bottom_off = window_rect.bottom - extended_frame.bottom;
            
            tracing::debug!("Extended frame bounds: left={}, top={}, right={}, bottom={}",
                extended_frame.left, extended_frame.top, extended_frame.right, extended_frame.bottom);
            tracing::debug!("DWM invisible border offsets: left={}, top={}, right={}, bottom={}",
                left_off, top_off, right_off, bottom_off);
            
            (left_off, top_off, right_off, bottom_off)
        } else {
            // Fallback: assume no invisible borders for transparent windows
            tracing::debug!("DwmGetWindowAttribute failed (hr=0x{:08X}), using zero offsets", hr);
            (0, 0, 0, 0)
        };
        
        // Calculate the visible content dimensions
        // For a transparent window with decorations=false, the content should fill the entire window
        let content_width = window_width - left_offset - right_offset;
        let content_height = window_height - top_offset - bottom_offset;
        
        tracing::info!("Content dimensions: {}x{} (offsets: l={}, t={}, r={}, b={})",
            content_width, content_height, left_offset, top_offset, right_offset, bottom_offset);
        
        // Radius for rounded corners (matches CSS --window-border-radius)
        let radius = 16;
        
        // CreateRoundRectRgn takes window-relative coordinates
        // The region should start at (left_offset, top_offset) to skip invisible borders
        // and extend to cover the visible content area
        let hrgn = CreateRoundRectRgn(
            left_offset,                          // x1: start after left invisible border
            top_offset,                           // y1: start after top invisible border  
            left_offset + content_width,          // x2: extend to visible right edge
            top_offset + content_height,          // y2: extend to visible bottom edge
            radius, 
            radius
        );
        
        if hrgn != std::ptr::null_mut() {
            let result = SetWindowRgn(hwnd, hrgn, 1);
            if result != 0 {
                tracing::info!(
                    "✓ Applied rounded region: x1={}, y1={}, x2={}, y2={}, radius={}",
                    left_offset, top_offset, 
                    left_offset + content_width, top_offset + content_height,
                    radius
                );
                // Force redraw
                InvalidateRect(hwnd, std::ptr::null(), 1);
            } else {
                tracing::warn!("SetWindowRgn returned 0 (failed)");
            }
        } else {
            tracing::warn!("Failed to create rounded region");
        }
    }
}

/// Enable window shadow for Windows 11 rounded corners
#[cfg(windows)]
pub fn enable_shadow_for_win11(window: &tauri::WebviewWindow) -> Result<(), String> {
    // Use centralized version detection (RtlGetVersion-based, more reliable than GetVersionExW)
    let is_win11 = crate::os::is_windows_11();

    if is_win11 {
        tracing::info!("Enabling shadow for Windows 11 rounded corners");
        window.set_shadow(true).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn set_rounded_corners(_hwnd: u64) -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn enable_shadow_for_win11(_window: &tauri::WebviewWindow) -> Result<(), String> {
    Ok(())
}
