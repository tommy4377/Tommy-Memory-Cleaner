use tauri::{AppHandle, Manager};

pub fn set_always_on_top(app: &AppHandle, on: bool) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.set_always_on_top(on).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(windows)]
pub fn set_rounded_corners(hwnd: windows_sys::Win32::Foundation::HWND) -> Result<(), String> {
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DWMWA_TRANSITIONS_FORCEDISABLED};
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
            // Windows 11: Use DWMWCP_ROUND
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
                tracing::info!("Successfully set rounded corners for Windows 11 window");
            }
            
            // Disable transitions on Windows 11
            let disable_transitions: i32 = 1;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_TRANSITIONS_FORCEDISABLED as i32,
                &disable_transitions as *const _ as *const _,
                std::mem::size_of::<i32>() as u32,
            );
        } else {
            // Windows 10: Skip SetWindowRgn - use CSS-only rounded corners
            tracing::info!("Windows 10 detected - using CSS-only rounded corners (no SetWindowRgn)");
        }
        
        Ok(())
    }
}

#[cfg(not(windows))]
pub fn set_rounded_corners(_hwnd: u64) -> Result<(), String> {
    Ok(())
}
