#[cfg(windows)]
use tauri::AppHandle;

// Helper to convert ICO to high-resolution PNG
#[cfg(windows)]
fn convert_ico_to_highres_png(ico_data: &[u8]) -> Result<Vec<u8>, String> {
    // Load the ICO using image::load_from_memory, which handles the format automatically
    let img =
        image::load_from_memory(ico_data).map_err(|e| format!("Failed to load ICO: {}", e))?;

    // Convert to RGBA8
    let rgba_img = img.to_rgba8();

    // Resize to 256x256 (high resolution for Windows Toast)
    let resized =
        image::imageops::resize(&rgba_img, 256, 256, image::imageops::FilterType::Lanczos3);

    // Encode as PNG using DynamicImage::save (image API 0.25)
    // Convert RgbaImage to DynamicImage in order to use save
    let dynamic_img = image::DynamicImage::ImageRgba8(resized);

    // Save to an in-memory buffer using the save_with_format method
    let mut png_data = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut png_data);
        dynamic_img
            .write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| format!("Failed to encode PNG: {}", e))?;
    }

    Ok(png_data)
}

// Helper to get the path to an accessible high-resolution PNG icon
// Windows Toast works better with high-resolution PNG (128x128 or larger) instead of ICO
#[cfg(windows)]
fn ensure_notification_icon_available() -> Option<std::path::PathBuf> {
    use std::fs;

    // Try reading a 128x128 PNG from the runtime directory first (if bundled with the app)
    // Otherwise use the embedded ICO and convert it to PNG using the image library
    let (icon_data, icon_ext) = {
        let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();

        // Try reading PNG from the runtime directory (if the app ships with icons)
        if let Ok(png_data) = fs::read(exe_dir.join("icons").join("128x128.png")) {
            (png_data, "png")
        } else if let Ok(png_data) = fs::read(exe_dir.join("128x128.png")) {
            (png_data, "png")
        } else if let Ok(png_data) = fs::read(exe_dir.join("icons").join("icon.png")) {
            (png_data, "png")
        } else if let Ok(png_data) = fs::read(exe_dir.join("icon.png")) {
            (png_data, "png")
        } else {
            // Fallback: convert the embedded ICO to a high-resolution 256x256 PNG
            // This fixes the pixelation/blurriness issue
            match convert_ico_to_highres_png(include_bytes!("../../icons/icon.ico")) {
                Ok(png_data) => {
                    tracing::debug!(
                        "Converted ICO to high-res PNG (256x256) for better notification quality"
                    );
                    (png_data, "png")
                }
                Err(e) => {
                    tracing::warn!("Failed to convert ICO to PNG, using ICO: {}", e);
                    (include_bytes!("../../icons/icon.ico").to_vec(), "ico")
                }
            }
        }
    };

    // Try to save the icon in the app's data directory
    let icon_path = {
        let detector = crate::config::get_portable_detector();
        detector.data_dir().join(format!("icon.{}", icon_ext))
    };

    // Create the directory if it doesn't exist
    if let Some(parent) = icon_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            tracing::warn!("Failed to create icon directory: {}", e);
            return None;
        }
    }

    // Copy the icon only if it doesn't exist or has changed
    // Check whether the file exists and has the same size
    let needs_copy = match fs::metadata(&icon_path) {
        Ok(meta) => meta.len() != icon_data.len() as u64,
        Err(_) => true, // File doesn't exist, needs to be copied
    };

    if needs_copy {
        if let Err(e) = fs::write(&icon_path, &icon_data) {
            tracing::warn!("Failed to write notification icon: {}", e);
            return None;
        }
        tracing::debug!(
            "Notification icon (format: {}) copied to: {}",
            icon_ext,
            icon_path.display()
        );
    }

    Some(icon_path)
}

/// Show Windows notification with proper icon and theme
///
/// Attempt chain (ordered by efficiency):
/// 1. Tauri Plugin Notification (native Rust, zero overhead) - PRIMARY
/// 2. winrt-notification crate (native Rust WinRT toast) - FALLBACK
/// 3. PowerShell Balloon (LAST RESORT - for stripped-down Windows installs)
#[cfg(windows)]
pub fn show_windows_notification(
    app: &AppHandle,
    title: &str,
    body: &str,
    _theme: &str,
) -> Result<(), String> {
    tracing::info!(
        "Attempting to show notification - Title: '{}', Body: '{}'",
        title,
        body
    );

    // ── Attempt 1: Tauri plugin notification (native Rust, zero overhead) - PRIMARY ──
    tracing::debug!("Attempt 1: Tauri plugin notification (PRIMARY)...");
    {
        use tauri_plugin_notification::NotificationExt;
        
        // Try with icon if available
        if let Some(icon_path) = ensure_notification_icon_available() {
            if let Some(icon_str) = icon_path.to_str() {
                tracing::debug!("Using icon path: {}", icon_str);
                // Convert filesystem path to file URI for Tauri plugin
                let icon_uri = format!("file:///{}", icon_str.replace('\\', "/"));
                match app
                    .notification()
                    .builder()
                    .title(title)
                    .body(body)
                    .icon(&icon_uri)
                    .show()
                {
                    Ok(_) => {
                        tracing::info!("✓ Notification sent via Tauri plugin (native, zero overhead)");
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::warn!("Tauri plugin with icon failed: {}, retrying without icon", e);
                    }
                }
            }
        }
        
        // Fallback: try without icon
        tracing::debug!("Retrying Tauri plugin without icon...");
        match app
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
        {
            Ok(_) => {
                tracing::info!("✓ Notification sent via Tauri plugin (no icon)");
                return Ok(());
            }
            Err(e) => {
                tracing::warn!("Tauri plugin notification failed entirely: {}, trying fallback", e);
            }
        }
    }

    // ── Attempt 2: winrt-notification crate (native Rust WinRT toast) - FALLBACK ──
    tracing::debug!("Attempt 2: winrt-notification crate (FALLBACK)...");
    {
        use winrt_notification::{IconCrop, Toast};

        let icon_path = ensure_notification_icon_available();

        let mut toast = Toast::new("TommyMemoryCleaner")
            .title(title)
            .text1(body);

        if let Some(ref path) = icon_path {
            toast = toast.icon(path, IconCrop::Circular, "Tommy Memory Cleaner");
        }

        match toast.show() {
            Ok(_) => {
                tracing::info!("✓ Notification sent via winrt-notification (native WinRT)");
                return Ok(());
            }
            Err(e) => {
                tracing::warn!(
                    "winrt-notification failed: {}, fallbacks exhausted",
                    e
                );
            }
        }
    }

    // ── Attempt 3: PowerShell Balloon (LAST RESORT) ──
    // Only used when both Tauri plugin and winrt-notification fail.
    // This is rare and typically happens on stripped-down Windows installations.
    tracing::debug!("Attempt 3: PowerShell balloon notification (LAST RESORT)...");
    {
        use std::process::Command;

        let exe_path = std::env::current_exe()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Escape single quotes in title and body for PowerShell
        let safe_title = title.replace('\'', "''");
        let safe_body = body.replace('\'', "''");
        let safe_exe = exe_path.replace('\'', "''");

        let ps_script = format!(
            r#"Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; $icon = [System.Drawing.Icon]::ExtractAssociatedIcon('{safe_exe}'); $notify = New-Object System.Windows.Forms.NotifyIcon; $notify.Icon = $icon; $notify.Visible = $true; $notify.BalloonTipTitle = '{safe_title}'; $notify.BalloonTipText = '{safe_body}'; $notify.BalloonTipIcon = [System.Windows.Forms.ToolTipIcon]::Info; $notify.ShowBalloonTip(5000); Start-Sleep -Seconds 6; $notify.Dispose()"#
        );

        let mut cmd = Command::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &ps_script]);

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        match cmd.output() {
            Ok(output) if output.status.success() => {
                tracing::info!("✓ Notification sent via PowerShell balloon (last resort)");
                return Ok(());
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::warn!("PowerShell balloon failed: {}", stderr);
            }
            Err(e) => {
                tracing::warn!("Failed to spawn PowerShell: {}", e);
            }
        }
    }

    tracing::error!("✗ All notification methods failed (Tauri plugin, winrt-notification, PowerShell balloon)");
    Err("All notification methods failed. Ensure system notifications are enabled.".to_string())
}

#[cfg(not(windows))]
pub fn show_windows_notification(
    _app: &AppHandle,
    _title: &str,
    _body: &str,
    _theme: &str,
) -> Result<(), String> {
    Ok(())
}

/// Register the app for Windows Toast notifications
#[cfg(windows)]
pub fn register_app_for_notifications() {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::Registry::{RegSetValueExW, HKEY_CURRENT_USER, REG_SZ};

    let _app_id = "TommyMemoryCleaner";
    // Use to_string_lossy() to correctly handle paths with Unicode characters
    let exe_path = std::env::current_exe()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    if exe_path.is_empty() {
        tracing::warn!("Cannot register app for notifications: exe path not found");
        return;
    }

    // Register AppUserModelID in the registry with DisplayName and IconUri
    // IMPORTANT: Windows requires this registration to happen BEFORE any notification
    // WE USE "TommyMemoryCleaner" as the AppUserModelID to show a user-friendly name in notifications
    let key_path = r"Software\Classes\AppUserModelId\TommyMemoryCleaner";
    let display_name = "Tommy Memory Cleaner";

    // Recursively delete the existing key to force re-creation (useful if it was modified)
    // Use SHDeleteKey to also remove subkeys
    unsafe {
        use windows_sys::Win32::System::Registry::{
            RegCloseKey, RegDeleteKeyW, RegOpenKeyExW, KEY_ALL_ACCESS,
        };
        // First try opening the key to check whether it exists
        let key_path_wide: Vec<u16> = OsStr::new(key_path).encode_wide().chain(Some(0)).collect();
        let mut hkey_test: windows_sys::Win32::Foundation::HANDLE = std::ptr::null_mut();
        let open_result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            key_path_wide.as_ptr(),
            0,
            KEY_ALL_ACCESS,
            &mut hkey_test,
        );
        if open_result == 0 && hkey_test != std::ptr::null_mut() {
            RegCloseKey(hkey_test);
            // Delete the key - may require multiple attempts
            let delete_result = RegDeleteKeyW(HKEY_CURRENT_USER, key_path_wide.as_ptr());
            if delete_result != 0 {
                tracing::debug!(
                    "Note: Could not delete existing registry key (may have subkeys): {}",
                    delete_result
                );
            } else {
                tracing::debug!("Deleted existing registry key for re-creation");
            }
        }
    }

    // Try to use a dedicated .ico file for better results with Windows Toast
    // Fall back to the exe if it fails
    let icon_path = ensure_notification_icon_available()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| exe_path.clone());

    // Convert strings to wide strings
    let key_path_wide: Vec<u16> = OsStr::new(key_path).encode_wide().chain(Some(0)).collect();
    let display_name_wide: Vec<u16> = OsStr::new(display_name)
        .encode_wide()
        .chain(Some(0))
        .collect();

    unsafe {
        // Create the key if it doesn't exist and set the values
        let mut hkey: windows_sys::Win32::Foundation::HANDLE = std::ptr::null_mut();
        let result = windows_sys::Win32::System::Registry::RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_path_wide.as_ptr(),
            0,
            std::ptr::null(),
            0,
            0x20006, // KEY_WRITE
            std::ptr::null(),
            &mut hkey,
            0 as *mut u32,
        );

        if result == 0 {
            // Set DisplayName
            let display_name_value: Vec<u16> = OsStr::new("DisplayName")
                .encode_wide()
                .chain(Some(0))
                .collect();
            RegSetValueExW(
                hkey,
                display_name_value.as_ptr(),
                0,
                REG_SZ,
                display_name_wide.as_ptr() as *const u8,
                (display_name_wide.len() * 2) as u32,
            );

            // Set IconUri
            let icon_uri_value: Vec<u16> =
                OsStr::new("IconUri").encode_wide().chain(Some(0)).collect();
            let icon_path_wide: Vec<u16> = OsStr::new(&icon_path)
                .encode_wide()
                .chain(Some(0))
                .collect();
            RegSetValueExW(
                hkey,
                icon_uri_value.as_ptr(),
                0,
                REG_SZ,
                icon_path_wide.as_ptr() as *const u8,
                (icon_path_wide.len() * 2) as u32,
            );

            windows_sys::Win32::System::Registry::RegCloseKey(hkey);
            tracing::info!("App registered for Windows notifications: {}", display_name);
        } else {
            tracing::error!("Failed to register app for notifications: 0x{:08X}", result);
        }
    }
}

#[cfg(not(windows))]
pub fn register_app_for_notifications() {
    // No-op on non-Windows platforms
}
