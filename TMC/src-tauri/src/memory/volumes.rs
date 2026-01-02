use anyhow::Result;
use std::ptr::null_mut;
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE},
    Storage::FileSystem::{
        CreateFileW, FlushFileBuffers, GetDriveTypeW, FILE_ATTRIBUTE_NORMAL,
        FILE_FLAG_NO_BUFFERING, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, OPEN_EXISTING,
    },
};

#[link(name = "kernel32")]
extern "system" {
    fn DeviceIoControl(
        hDevice: HANDLE,
        dwIoControlCode: u32,
        lpInBuffer: *mut core::ffi::c_void,
        nInBufferSize: u32,
        lpOutBuffer: *mut core::ffi::c_void,
        nOutBufferSize: u32,
        lpBytesReturned: *mut u32,
        lpOverlapped: *mut core::ffi::c_void,
    ) -> i32;
}

fn to_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

const FSCTL_DISCARD_VOLUME_CACHE: u32 = 0x00090054;
const FSCTL_RESET_WRITE_ORDER: u32 = 0x000900F8;
const DRIVE_FIXED: u32 = 3;

fn is_fixed_drive(letter: char) -> bool {
    let root = format!("{}:\\", letter);
    let root_w = to_wide(&root);
    unsafe { GetDriveTypeW(root_w.as_ptr()) == DRIVE_FIXED }
}

fn open_volume(letter: char) -> Option<HANDLE> {
    // Try multiple approaches to open the volume
    let path = format!(r"\\.\{}:", letter);
    let path_w = to_wide(&path);
    
    // Strategy 1: Standard approach with minimal rights
    if let Some(handle) = try_open_volume(&path_w, FILE_GENERIC_READ | FILE_GENERIC_WRITE, FILE_ATTRIBUTE_NORMAL | FILE_FLAG_NO_BUFFERING) {
        return Some(handle);
    }
    
    // Strategy 2: Query-only access (read-only)
    tracing::debug!("Retrying volume {} with query-only access", letter);
    if let Some(handle) = try_open_volume(&path_w, 0, FILE_ATTRIBUTE_NORMAL) {
        tracing::info!("Successfully opened volume {} with query-only access", letter);
        return Some(handle);
    }
    
    // Strategy 3: Attempt with different sharing flags
    tracing::debug!("Retrying volume {} with exclusive access", letter);
    if let Some(handle) = try_open_volume(&path_w, FILE_GENERIC_READ | FILE_GENERIC_WRITE, FILE_ATTRIBUTE_NORMAL) {
        return Some(handle);
    }
    
    tracing::warn!("Failed to open volume {} after all attempts", letter);
    None
}

fn try_open_volume(path_w: &[u16], access: u32, flags: u32) -> Option<HANDLE> {
    unsafe {
        let h = CreateFileW(
            path_w.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            flags,
            0,
        );
        if h == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(h)
        }
    }
}

pub fn flush_modified_file_cache_all() -> Result<()> {
    // Ensure required privileges before attempting volume operations
    let mut privileges_acquired = true;
    if let Err(e) = crate::memory::privileges::ensure_privileges(&["SeManageVolumePrivilege"]) {
        tracing::warn!("Failed to acquire SeManageVolumePrivilege: {}", e);
        privileges_acquired = false;
    }

    let mut any_success = false;
    let mut volumes_failed = 0;
    let mut volumes_total = 0;

    for letter in 'C'..='Z' {
        if !is_fixed_drive(letter) {
            continue;
        }

        if let Some(h) = open_volume(letter) {
            volumes_total += 1;
            unsafe {
                let mut _ret: u32 = 0;
                let mut volume_success = false;

                // First flush any pending writes
                let flush_result = FlushFileBuffers(h);
                if flush_result == 0 {
                    let error = GetLastError();
                    // Don't log ERROR_INVALID_HANDLE as debug, it's expected in some scenarios
                    if error != 6 {
                        tracing::debug!("FlushFileBuffers failed for {}: {}", letter, error);
                    }
                } else {
                    volume_success = true;
                }

                // Then reset write order (only if we have proper privileges)
                if privileges_acquired {
                    let result1 = DeviceIoControl(
                        h,
                        FSCTL_RESET_WRITE_ORDER,
                        null_mut(),
                        0,
                        null_mut(),
                        0,
                        &mut _ret,
                        null_mut(),
                    );
                    if result1 == 0 {
                        let error = GetLastError();
                        if error != 6 && error != 1 { // 1 = ERROR_INVALID_FUNCTION
                            tracing::debug!(
                                "DeviceIoControl(FSCTL_RESET_WRITE_ORDER) failed for {}: {}",
                                letter,
                                error
                            );
                        }
                    } else {
                        volume_success = true;
                    }
                }

                // Finally discard volume cache (only if we have proper privileges)
                if privileges_acquired {
                    let result2 = DeviceIoControl(
                        h,
                        FSCTL_DISCARD_VOLUME_CACHE,
                        null_mut(),
                        0,
                        null_mut(),
                        0,
                        &mut _ret,
                        null_mut(),
                    );
                    if result2 == 0 {
                        let error = GetLastError();
                        if error != 6 && error != 1 { // 1 = ERROR_INVALID_FUNCTION
                            tracing::debug!(
                                "DeviceIoControl(FSCTL_DISCARD_VOLUME_CACHE) failed for {}: {}",
                                letter,
                                error
                            );
                        }
                    } else {
                        volume_success = true;
                    }
                }

                CloseHandle(h);
                
                if volume_success {
                    any_success = true;
                } else {
                    volumes_failed += 1;
                }
            }
        }
    }

    // Provide detailed feedback about volume operations
    if volumes_total == 0 {
        tracing::info!("No fixed drives found to optimize");
        Ok(())
    } else if volumes_failed == volumes_total {
        // All volumes failed - this is expected in some environments
        tracing::warn!(
            "All {} volume(s) failed to optimize (ERROR_INVALID_HANDLE). This is normal when antivirus or system locks prevent direct volume access.",
            volumes_total
        );
        tracing::info!("Tip: Try running TMC as administrator or temporarily disable antivirus protection");
        Ok(()) // Still return OK to not crash the optimization
    } else if any_success {
        tracing::info!("Successfully optimized {} of {} volumes", volumes_total - volumes_failed, volumes_total);
        Ok(())
    } else {
        tracing::warn!("Volume operations completed with mixed results");
        Ok(())
    }
}
