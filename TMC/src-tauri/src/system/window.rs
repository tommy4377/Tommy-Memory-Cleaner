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
    use windows_sys::Win32::System::SystemInformation::GetVersionExW;

    unsafe {
        // Check Windows version
        let mut version = windows_sys::Win32::System::SystemInformation::OSVERSIONINFOEXW {
            dwOSVersionInfoSize: std::mem::size_of::<
                windows_sys::Win32::System::SystemInformation::OSVERSIONINFOEXW,
            >() as u32,
            ..std::mem::zeroed()
        };

        let is_win11 = if GetVersionExW(&mut version as *mut _ as *mut _) != 0 {
            version.dwMajorVersion == 10
                && version.dwMinorVersion == 0
                && version.dwBuildNumber >= 22000
        } else {
            false
        };

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
    use windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect;
    
    unsafe {
        tracing::info!("Applying region-based rounded corners (Windows 10 method)");
        
        let mut client_rect: RECT = std::mem::zeroed();
        
        if GetClientRect(hwnd, &mut client_rect) != 0 {
            let client_width = client_rect.right - client_rect.left;
            let client_height = client_rect.bottom - client_rect.top;
            
            tracing::info!("Client dimensions: {}x{}", client_width, client_height);
            
            // OFFSET SPERIMENTALE: Windows 10 ha bordo invisibile di ~4px sui lati
            let offset_x = -4;  // Sposta a SINISTRA di 4px
            let offset_y = 0;   // Altezza già corretta
            
            let radius = 16;
            
            // Crea region con offset per compensare bordi DWM
            let hrgn = CreateRoundRectRgn(
                offset_x,                      // Inizia 4px a sinistra
                offset_y,                      // Inizia dall'alto
                client_width + offset_x.abs(), // Larghezza compensata
                client_height,                 // Altezza normale
                radius, 
                radius
            );
            
            if hrgn != std::ptr::null_mut() {
                let result = SetWindowRgn(hwnd, hrgn, 1);
                if result != 0 {
                    tracing::info!(
                        "✓ Successfully applied rounded region with offset_x={}, width={}, height={}",
                        offset_x, client_width + offset_x.abs(), client_height
                    );
                    InvalidateRect(hwnd, std::ptr::null(), 1);
                } else {
                    tracing::warn!("SetWindowRgn returned 0 (failed)");
                }
            } else {
                tracing::warn!("Failed to create rounded region");
            }
        } else {
            tracing::warn!("Failed to get client rect");
        }
    }
}

/// Enable window shadow for Windows 11 rounded corners
#[cfg(windows)]
pub fn enable_shadow_for_win11(window: &tauri::WebviewWindow) -> Result<(), String> {
    use windows_sys::Win32::System::SystemInformation::GetVersionExW;

    unsafe {
        let mut version = windows_sys::Win32::System::SystemInformation::OSVERSIONINFOEXW {
            dwOSVersionInfoSize: std::mem::size_of::<
                windows_sys::Win32::System::SystemInformation::OSVERSIONINFOEXW,
            >() as u32,
            ..std::mem::zeroed()
        };

        let is_win11 = if GetVersionExW(&mut version as *mut _ as *mut _) != 0 {
            version.dwMajorVersion == 10
                && version.dwMinorVersion == 0
                && version.dwBuildNumber >= 22000
        } else {
            false
        };

        if is_win11 {
            tracing::info!("Enabling shadow for Windows 11 rounded corners");
            window.set_shadow(true).map_err(|e| e.to_string())?;
        }
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
