use tauri::{AppHandle, Manager};

#[cfg(windows)]
use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DWMWA_TRANSITIONS_FORCEDISABLED};

pub fn set_always_on_top(app: &AppHandle, on: bool) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.set_always_on_top(on).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(windows)]
pub fn set_rounded_corners(hwnd: windows_sys::Win32::Foundation::HWND) -> Result<(), String> {
    unsafe {
        // Set corner preference to round
        let preference: u32 = DWMWCP_ROUND as u32;
        let attribute: i32 = DWMWA_WINDOW_CORNER_PREFERENCE as i32;
        let result = DwmSetWindowAttribute(
            hwnd,
            attribute,
            &preference as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );
        
        if result != 0 {
            tracing::warn!("Failed to set rounded corners: HRESULT 0x{:08X}", result);
            return Err(format!("Failed to set rounded corners: 0x{:08X}", result));
        }
        
        tracing::info!("Successfully set rounded corners for window");
        
        // Also ensure the window background is opaque on Windows 10/11
        let disable_transitions: i32 = 1;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED as i32,
            &disable_transitions as *const _ as *const _,
            std::mem::size_of::<i32>() as u32,
        );
        
        Ok(())
    }
}

#[cfg(not(windows))]
pub fn set_rounded_corners(_hwnd: u64) -> Result<(), String> {
    Ok(())
}
