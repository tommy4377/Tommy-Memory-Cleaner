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
    let path = format!(r"\\.\{}:", letter);
    let path_w = to_wide(&path);
    unsafe {
        let h = CreateFileW(
            path_w.as_ptr(),
            FILE_GENERIC_READ | FILE_GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_NO_BUFFERING,
            0, // lpTemplateFile: HANDLE in windows-sys is isize, use 0 instead of null_mut()
        );
        // HANDLE in windows-sys is isize, INVALID_HANDLE_VALUE is -1
        if h == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(h)
        }
    }
}

pub fn flush_modified_file_cache_all() -> Result<()> {
    // Ensure required privileges before attempting volume operations
    if let Err(e) = crate::memory::privileges::ensure_privileges(&["SeManageVolumePrivilege"]) {
        tracing::warn!("Failed to acquire SeManageVolumePrivilege: {}", e);
        // Continue anyway as some operations might still work
    }

    let mut any_success = false;
    let mut last_error = 0;

    for letter in 'C'..='Z' {
        if !is_fixed_drive(letter) {
            continue;
        }

        if let Some(h) = open_volume(letter) {
            unsafe {
                let mut _ret: u32 = 0;

                // First flush any pending writes
                let flush_result = FlushFileBuffers(h);
                if flush_result == 0 {
                    tracing::debug!("FlushFileBuffers failed for {}: {}", letter, GetLastError());
                }

                // Then reset write order
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
                    tracing::debug!(
                        "DeviceIoControl(FSCTL_RESET_WRITE_ORDER) failed for {}: {}",
                        letter,
                        error
                    );
                    // ERROR_INVALID_HANDLE (6) is critical
                    if error == 6 {
                        tracing::error!("Invalid handle detected for volume {} - possible permission issue", letter);
                        CloseHandle(h);
                        continue;
                    }
                }

                // Finally discard volume cache
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
                    tracing::debug!(
                        "DeviceIoControl(FSCTL_DISCARD_VOLUME_CACHE) failed for {}: {}",
                        letter,
                        error
                    );
                    // ERROR_INVALID_FUNCTION (1) means operation not supported
                    if error != 1 {
                        last_error = error;
                    }
                }

                CloseHandle(h);
                
                // Consider success if at least one operation completed
                if result1 != 0 || result2 != 0 || flush_result != 0 {
                    any_success = true;
                }
            }
        }
    }

    // Se almeno un volume è stato ottimizzato con successo, considera OK
    if any_success {
        Ok(())
    } else if last_error != 0 {
        // Se c'è stato almeno un tentativo ma tutti falliti
        tracing::warn!("All volume flush attempts failed, but continuing");
        Ok(()) // Non far crashare, continua comunque
    } else {
        // Nessun volume trovato
        tracing::info!("No fixed drives found to optimize");
        Ok(())
    }
}
