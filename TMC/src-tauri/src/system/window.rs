use tauri::{AppHandle, Manager};

pub fn set_always_on_top(app: &AppHandle, on: bool) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.set_always_on_top(on).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Apply platform-appropriate window decorations (rounded corners + shadow).
///
/// - Windows 11: native DWM rounded corners (`DWMWCP_ROUND`) + DWM shadow.
///   The DWM attribute persists across resizes, so it never needs reapplying.
/// - Windows 10: no OS-level rounding. The window is transparent/undecorated and
///   the frontend draws the rounded shape with CSS (`--window-border-radius`),
///   which scales with the window automatically and therefore cannot flicker
///   during Compact/Full transitions. Any stale GDI region is cleared here.
#[cfg(windows)]
pub fn apply_window_decorations(window: &tauri::WebviewWindow) -> Result<(), String> {
    // Shadow first (Windows 11 only), then corner preference
    let _ = enable_shadow_for_win11(window);

    if let Ok(hwnd) = window.hwnd() {
        let _ = set_rounded_corners(hwnd.0 as windows_sys::Win32::Foundation::HWND);
    }

    Ok(())
}

/// Apply the platform-specific corner strategy to a raw HWND.
///
/// Idempotent: safe to call multiple times on the same window.
#[cfg(windows)]
pub fn set_rounded_corners(hwnd: windows_sys::Win32::Foundation::HWND) -> Result<(), String> {
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE,
    };

    unsafe {
        // Use centralized version detection (RtlGetVersion-based, more reliable than GetVersionExW)
        if crate::os::is_windows_11() {
            // Windows 11: native DWM rounded corners (anti-aliased, system radius,
            // automatically maintained by the compositor across moves/resizes)
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
            // Windows 10: CSS-only rounding.
            //
            // SetWindowRgn/CreateRoundRectRgn is intentionally NOT used here: GDI
            // regions are 1-bit clip masks with no anti-aliasing (jagged corners),
            // and a region does not scale with the window, which caused visible
            // corner glitches on every Compact/Full resize until it was rebuilt.
            // The transparent window + CSS border-radius produces smooth corners
            // with zero per-resize work. We only clear any region that a previous
            // code path may have installed on this window.
            clear_window_region(hwnd);
            tracing::info!(
                "Windows 10 detected - rounded corners are handled by CSS (--window-border-radius)"
            );
        }
    }
    Ok(())
}

/// Remove any GDI window region so the (transparent) window surface is unclipped.
/// The visible rounded shape on Windows 10 is drawn entirely by the webview CSS.
#[cfg(windows)]
fn clear_window_region(hwnd: windows_sys::Win32::Foundation::HWND) {
    use windows_sys::Win32::Graphics::Gdi::SetWindowRgn;

    unsafe {
        // Passing a null region removes clipping; bRedraw=1 repaints the frame
        SetWindowRgn(hwnd, std::ptr::null_mut(), 1);
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
