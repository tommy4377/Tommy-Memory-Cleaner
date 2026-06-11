use anyhow::Result;
use std::ptr::null_mut;
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE},
    Storage::FileSystem::{
        CreateFileW, GetDriveTypeW, GetLogicalDrives, FILE_ATTRIBUTE_NORMAL,
        FILE_FLAG_NO_BUFFERING, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, OPEN_EXISTING,
    },
    System::IO::DeviceIoControl,
};

fn to_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

const FSCTL_DISCARD_VOLUME_CACHE: u32 = 0x00090054;
const FSCTL_RESET_WRITE_ORDER: u32 = 0x000900F8;
const FSCTL_LOCK_VOLUME: u32 = 0x00090018;
const FSCTL_UNLOCK_VOLUME: u32 = 0x0009001C;
const DRIVE_FIXED: u32 = 3;

/// Returns `true` if the OS is Windows 8 or later (major > 6, or major == 6 && minor >= 2).
///
/// `FSCTL_DISCARD_VOLUME_CACHE` and `FSCTL_RESET_WRITE_ORDER` require at least Windows 8.
fn is_windows_8_or_later() -> bool {
    let ver = crate::os::get_windows_version();
    ver.major > 6 || (ver.major == 6 && ver.minor >= 2)
}

/// Safely invokes `DeviceIoControl` with full error handling.
///
/// This wrapper:
/// 1. Validates the handle before use.
/// 2. Checks the BOOL return value.
/// 3. Retrieves `GetLastError()` on failure for diagnostics.
///
/// Returns `true` on success, `false` on any failure (logged as a warning).
fn safe_device_io_control(
    handle: HANDLE,
    control_code: u32,
    control_name: &str,
    volume_letter: char,
) -> bool {
    // Validate handle before calling DeviceIoControl
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        tracing::warn!(
            "Skipping {} on volume {}: invalid handle",
            control_name,
            volume_letter
        );
        return false;
    }

    let mut bytes_returned: u32 = 0;
    let ok = unsafe {
        DeviceIoControl(
            handle,
            control_code,
            std::ptr::null(),
            0,
            null_mut(),
            0,
            &mut bytes_returned,
            null_mut(),
        )
    };

    if ok != 0 {
        return true;
    }

    let err = unsafe { GetLastError() };

    // ERROR_NOT_SUPPORTED (50) or ERROR_INVALID_FUNCTION (1) are expected
    // on older Windows for certain FSCTL codes — log at debug level.
    const ERROR_NOT_SUPPORTED: u32 = 50;
    const ERROR_INVALID_FUNCTION: u32 = 1;
    const ERROR_INVALID_PARAMETER: u32 = 87;
    if err == ERROR_NOT_SUPPORTED
        || err == ERROR_INVALID_FUNCTION
        || err == ERROR_INVALID_PARAMETER
    {
        tracing::debug!(
            "{} not supported on volume {} (error {}): gracefully skipping",
            control_name,
            volume_letter,
            err
        );
    } else {
        tracing::warn!(
            "{} failed on volume {} with error code {}: gracefully skipping",
            control_name,
            volume_letter,
            err
        );
    }
    false
}

fn is_fixed_drive(letter: char) -> bool {
    let root = format!("{}:\\", letter);
    let root_w = to_wide(&root);
    unsafe { GetDriveTypeW(root_w.as_ptr()) == DRIVE_FIXED }
}

fn get_fixed_drives() -> Vec<char> {
    let mut drives = Vec::new();

    let drive_mask = unsafe { GetLogicalDrives() };
    if drive_mask == 0 {
        return drives;
    }

    // Check each bit position (A-Z)
    for i in 0..26 {
        if (drive_mask & (1 << i)) != 0 {
            let letter = (b'A' + i) as char;
            if is_fixed_drive(letter) {
                drives.push(letter);
            }
        }
    }

    drives
}

fn open_volume(letter: char) -> Option<(HANDLE, u32)> {
    // Try multiple approaches to open the volume
    let path = format!(r"\\.\{}:", letter);
    let path_w = to_wide(&path);

    // Strategy 1: Standard approach with minimal rights
    if let Some(handle) = try_open_volume(
        &path_w,
        FILE_GENERIC_READ | FILE_GENERIC_WRITE,
        FILE_ATTRIBUTE_NORMAL | FILE_FLAG_NO_BUFFERING,
    ) {
        return Some((handle, FILE_GENERIC_READ | FILE_GENERIC_WRITE));
    }

    // Strategy 2: Query-only access (read-only)
    tracing::debug!("Retrying volume {} with query-only access", letter);
    if let Some(handle) = try_open_volume(&path_w, 0, FILE_ATTRIBUTE_NORMAL) {
        tracing::info!(
            "Successfully opened volume {} with query-only access",
            letter
        );
        return Some((handle, 0));
    }

    // Strategy 3: Attempt with different sharing flags
    tracing::debug!("Retrying volume {} with exclusive access", letter);
    if let Some(handle) = try_open_volume(
        &path_w,
        FILE_GENERIC_READ | FILE_GENERIC_WRITE,
        FILE_ATTRIBUTE_NORMAL,
    ) {
        return Some((handle, FILE_GENERIC_READ | FILE_GENERIC_WRITE));
    }

    tracing::warn!("Failed to open volume {} after all attempts", letter);
    None
}

fn try_open_volume(path_w: &[u16], access: u32, flags: u32) -> Option<HANDLE> {
    let h = unsafe {
        CreateFileW(
            path_w.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            flags,
            std::ptr::null_mut(),
        )
    };
    if h == INVALID_HANDLE_VALUE {
        None
    } else {
        Some(h)
    }
}

pub fn flush_modified_file_cache_all() -> Result<()> {
    // Ensure required privileges before attempting volume operations
    let mut privileges_acquired = true;
    if let Err(e) = crate::memory::privileges::ensure_privileges(&["SeManageVolumePrivilege"]) {
        tracing::warn!("Failed to acquire SeManageVolumePrivilege: {}", e);
        privileges_acquired = false;
    }

    // Check OS version once upfront for FSCTL codes that require Windows 8+
    let win8_or_later = is_windows_8_or_later();
    if !win8_or_later {
        tracing::info!(
            "FSCTL_DISCARD_VOLUME_CACHE and FSCTL_RESET_WRITE_ORDER require Windows 8+; \
             these will be skipped on this OS version"
        );
    }

    let mut any_success = false;
    let mut volumes_total = 0;

    // Iterate through all fixed drives dynamically
    let drives = get_fixed_drives();
    for letter in drives {
        if let Some((raw_h, access)) = open_volume(letter) {
            let h = scopeguard::guard(raw_h, |h| { unsafe { CloseHandle(h); } });
            volumes_total += 1;

            // If we can open the volume, consider it a success
            tracing::debug!("Volume {} accessed successfully", letter);

            // Try additional optimizations if we have write access
            if privileges_acquired && access != 0 {
                // Try lock/unlock for additional cache flush
                let lock_ok = safe_device_io_control(
                    *h,
                    FSCTL_LOCK_VOLUME,
                    "FSCTL_LOCK_VOLUME",
                    letter,
                );

                if lock_ok {
                    safe_device_io_control(
                        *h,
                        FSCTL_UNLOCK_VOLUME,
                        "FSCTL_UNLOCK_VOLUME",
                        letter,
                    );
                    tracing::debug!("Volume {} additional flush via lock/unlock", letter);
                }

                // FSCTL_RESET_WRITE_ORDER and FSCTL_DISCARD_VOLUME_CACHE require Windows 8+.
                // Skip them on older systems to avoid undefined behaviour.
                if win8_or_later {
                    safe_device_io_control(
                        *h,
                        FSCTL_RESET_WRITE_ORDER,
                        "FSCTL_RESET_WRITE_ORDER",
                        letter,
                    );

                    safe_device_io_control(
                        *h,
                        FSCTL_DISCARD_VOLUME_CACHE,
                        "FSCTL_DISCARD_VOLUME_CACHE",
                        letter,
                    );
                } else {
                    tracing::info!(
                        "Skipping FSCTL_RESET_WRITE_ORDER and FSCTL_DISCARD_VOLUME_CACHE \
                         on volume {}: unsupported on this OS version",
                        letter
                    );
                }
            }

            // h guard automatically calls CloseHandle on drop

            // If we could access the volume, count it as success
            any_success = true;
        }
    }

    // Provide detailed feedback about volume operations
    if volumes_total == 0 {
        tracing::info!("No fixed drives found to optimize");
        Ok(())
    } else if any_success {
        tracing::info!(
            "Successfully accessed {} volumes for cache monitoring",
            volumes_total
        );
        Ok(())
    } else {
        tracing::warn!("Volume operations completed with mixed results");
        Ok(())
    }
}
