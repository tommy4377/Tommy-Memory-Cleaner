use tauri::{AppHandle, Manager};

pub fn set_always_on_top(app: &AppHandle, on: bool) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.set_always_on_top(on).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(windows)]
pub fn set_rounded_corners(hwnd: windows_sys::Win32::Foundation::HWND) -> Result<(), String> {
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND};
    use windows_sys::Win32::System::SystemInformation::GetVersionExW;

    unsafe {
        // Check Windows version
        let mut version = windows_sys::Win32::System::SystemInformation::OSVERSIONINFOEXW {
            dwOSVersionInfoSize: std::mem::size_of::<windows_sys::Win32::System::SystemInformation::OSVERSIONINFOEXW>() as u32,
            ..std::mem::zeroed()
        };

        let is_win11 = if GetVersionExW(&mut version as *mut _ as *mut _) != 0 {
            // Windows 11 is version 10.0.22000 or higher
            version.dwMajorVersion == 10 && version.dwMinorVersion == 0 && version.dwBuildNumber >= 22000
        } else {
            false
        };

        if is_win11 {
            // Windows 11: Use native DWM rounded corners
            tracing::info!("Windows 11 detected - enabling native DWM rounded corners");
            let preference: u32 = DWMWCP_ROUND as u32;
            let attribute: i32 = DWMWA_WINDOW_CORNER_PREFERENCE as i32;
            let result = DwmSetWindowAttribute(
                hwnd,
                attribute,
                &preference as *const _ as *const _,
                std::mem::size_of::<u32>() as u32,
            );

            if result != 0 {
                tracing::warn!("Failed to set rounded corners on Windows 11: HRESULT 0x{:08X}", result);
            } else {
                tracing::info!("✓ Successfully applied native rounded corners (Windows 11)");
            }
        } else {
            // Windows 10: Use region-based approach
            tracing::info!("Windows 10 detected - using region-based rounded corners");
            
            use windows_sys::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};
            use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowRect, RECT};
            
            // Get window dimensions
            let mut rect: RECT = std::mem::zeroed();
            if GetWindowRect(hwnd, &mut rect) != 0 {
                let width = rect.right - rect.left;
                let height = rect.bottom - rect.top;
                
                // Create rounded region (8px radius to match CSS)
                let hrgn = CreateRoundRectRgn(0, 0, width, height, 16, 16);
                
                if hrgn != 0 {
                    // Apply the region (TRUE = redraw immediately)
                    SetWindowRgn(hwnd, hrgn, 1);
                    tracing::info!("✓ Successfully applied rounded region (Windows 10)");
                } else {
                    tracing::warn!("Failed to create rounded region for Windows 10");
                }
            } else {
                tracing::warn!("Failed to get window rect for Windows 10");
            }
        }
    }

    Ok(())
}

/// Enable window shadow for Windows 11 rounded corners
#[cfg(windows)]
pub fn enable_shadow_for_win11(window: &tauri::WebviewWindow) -> Result<(), String> {
    use windows_sys::Win32::System::SystemInformation::GetVersionExW;

    unsafe {
        let mut version = windows_sys::Win32::System::SystemInformation::OSVERSIONINFOEXW {
            dwOSVersionInfoSize: std::mem::size_of::<windows_sys::Win32::System::SystemInformation::OSVERSIONINFOEXW>() as u32,
            ..std::mem::zeroed()
        };

        let is_win11 = if GetVersionExW(&mut version as *mut _ as *mut _) != 0 {
            version.dwMajorVersion == 10 && version.dwMinorVersion == 0 && version.dwBuildNumber >= 22000
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
