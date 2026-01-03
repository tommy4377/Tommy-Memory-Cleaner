use tauri::{AppHandle, Manager};

pub fn set_always_on_top(app: &AppHandle, on: bool) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.set_always_on_top(on).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(windows)]
pub fn set_rounded_corners(_hwnd: windows_sys::Win32::Foundation::HWND) -> Result<(), String> {
    // CSS-only rounded corners for both Windows 10 and 11
    // Windows 11 DWMWCP_ROUND requires shadow: true which breaks Windows 10
    // Using CSS provides consistent appearance across both versions
    tracing::info!("Using CSS-only rounded corners for all Windows versions");
    Ok(())
}

#[cfg(not(windows))]
pub fn set_rounded_corners(_hwnd: u64) -> Result<(), String> {
    Ok(())
}
