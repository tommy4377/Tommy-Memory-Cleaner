use tauri::{AppHandle, Manager};

pub fn set_always_on_top(app: &AppHandle, on: bool) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.set_always_on_top(on).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Corner radius in logical pixels. Must match the frontend's
/// `--window-border-radius` (see `cmd_get_window_config` and Titlebar.svelte)
/// so the Windows 10 clip region and the CSS-drawn edge coincide.
/// 12px matches the radius the app historically used on Windows 10
/// (16px was judged too visibly rounded).
#[cfg(windows)]
const CORNER_RADIUS_PX: i32 = 12;

/// Apply platform-appropriate window decorations (rounded corners + shadow).
///
/// - Windows 11: native DWM rounded corners (`DWMWCP_ROUND`) + DWM shadow.
///   The DWM attribute persists across resizes, so it never needs reapplying.
/// - Windows 10: GDI clip region (`CreateRoundRectRgn` + `SetWindowRgn`) sized
///   to the current window bounds. A region does not scale with the window, so
///   `main.rs` reapplies it from the global window-event handler on every
///   `Resized` event. The CSS `--window-border-radius` still draws the
///   anti-aliased rounded edge inside the clipped surface.
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
            // Windows 10: DWMWA_WINDOW_CORNER_PREFERENCE does not exist, so clip
            // the window to a rounded rectangle with a GDI region. The CSS
            // border-radius draws the smooth edge; the region cuts away the
            // square corner pixels that WebView2 paints opaque on Windows 10.
            apply_win10_region(hwnd);
        }
    }
    Ok(())
}

/// Clip the window to a rounded rectangle matching its current bounds
/// (Windows 10 fallback for the missing DWM corner-preference API).
///
/// Must be called again after every resize: `SetWindowRgn` regions are in
/// fixed window coordinates and do not track the window size.
#[cfg(windows)]
fn apply_win10_region(hwnd: windows_sys::Win32::Foundation::HWND) {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};
    use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;

    unsafe {
        let mut rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            tracing::warn!("apply_win10_region: GetWindowRect failed");
            return;
        }
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        if width <= 0 || height <= 0 {
            return;
        }

        // Scale the logical radius to the window's DPI (GetDpiForWindow is
        // available since Windows 10 1607; 0 means failure — assume 96).
        let dpi = match GetDpiForWindow(hwnd) {
            0 => 96,
            dpi => dpi,
        } as i32;
        let diameter = 2 * CORNER_RADIUS_PX * dpi / 96;

        // Region coordinates are window-relative; right/bottom are exclusive,
        // so +1 keeps the outermost row/column of pixels visible.
        let hrgn = CreateRoundRectRgn(0, 0, width + 1, height + 1, diameter, diameter);
        if hrgn.is_null() {
            tracing::warn!("apply_win10_region: CreateRoundRectRgn failed");
            return;
        }

        // On success the system owns the region — it must NOT be deleted here.
        if SetWindowRgn(hwnd, hrgn, 1) == 0 {
            // Ownership was not transferred; free the region to avoid a GDI leak.
            windows_sys::Win32::Graphics::Gdi::DeleteObject(
                hrgn as windows_sys::Win32::Graphics::Gdi::HGDIOBJ,
            );
            tracing::warn!("apply_win10_region: SetWindowRgn failed");
        } else {
            tracing::info!(
                "✓ Applied Windows 10 rounded region ({}x{}, radius {}px @ {} DPI)",
                width,
                height,
                diameter / 2,
                dpi
            );
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
