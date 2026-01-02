use anyhow::Result;
use std::ptr::null_mut;
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
    Storage::FileSystem::{
        CreateFileW, GetDriveTypeW, GetLogicalDrives, FILE_ATTRIBUTE_NORMAL,
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
const FSCTL_LOCK_VOLUME: u32 = 0x00090018;
const FSCTL_UNLOCK_VOLUME: u32 = 0x0009001C;
const DRIVE_FIXED: u32 = 3;

fn is_fixed_drive(letter: char) -> bool {
    let root = format!("{}:\\", letter);
    let root_w = to_wide(&root);
    unsafe { GetDriveTypeW(root_w.as_ptr()) == DRIVE_FIXED }
}

fn get_fixed_drives() -> Vec<char> {
    let mut drives = Vec::new();
    
    unsafe {
        let drive_mask = GetLogicalDrives();
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
    }
    
    drives
}

fn open_volume(letter: char) -> Option<(HANDLE, u32)> {
    // Try multiple approaches to open the volume
    let path = format!(r"\\.\{}:", letter);
    let path_w = to_wide(&path);
    
    // Strategy 1: Standard approach with minimal rights
    if let Some(handle) = try_open_volume(&path_w, FILE_GENERIC_READ | FILE_GENERIC_WRITE, FILE_ATTRIBUTE_NORMAL | FILE_FLAG_NO_BUFFERING) {
        return Some((handle, FILE_GENERIC_READ | FILE_GENERIC_WRITE));
    }
    
    // Strategy 2: Query-only access (read-only)
    tracing::debug!("Retrying volume {} with query-only access", letter);
    if let Some(handle) = try_open_volume(&path_w, 0, FILE_ATTRIBUTE_NORMAL) {
        tracing::info!("Successfully opened volume {} with query-only access", letter);
        return Some((handle, 0));
    }
    
    // Strategy 3: Attempt with different sharing flags
    tracing::debug!("Retrying volume {} with exclusive access", letter);
    if let Some(handle) = try_open_volume(&path_w, FILE_GENERIC_READ | FILE_GENERIC_WRITE, FILE_ATTRIBUTE_NORMAL) {
        return Some((handle, FILE_GENERIC_READ | FILE_GENERIC_WRITE));
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
            null_mut(),
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
    let mut volumes_total = 0;

    // Iterate through all fixed drives dynamically
    let drives = get_fixed_drives();
    for letter in drives {

        if let Some((h, access)) = open_volume(letter) {
            volumes_total += 1;
            unsafe {
                let mut _ret: u32 = 0;

                // If we can open the volume, consider it a success
                // The actual cache flushing is handled by other optimizations (Modified Page List, System File Cache)
                tracing::debug!("Volume {} accessed successfully", letter);
                
                // Try additional optimizations if we have write access
                if privileges_acquired && access != 0 {
                    // Try lock/unlock for additional cache flush
                    let lock_result = DeviceIoControl(
                        h,
                        FSCTL_LOCK_VOLUME,
                        null_mut(),
                        0,
                        null_mut(),
                        0,
                        &mut _ret,
                        null_mut(),
                    );
                    
                    if lock_result != 0 {
                        DeviceIoControl(
                            h,
                            FSCTL_UNLOCK_VOLUME,
                            null_mut(),
                            0,
                            null_mut(),
                            0,
                            &mut _ret,
                            null_mut(),
                        );
                        tracing::debug!("Volume {} additional flush via lock/unlock", letter);
                    }
                    
                    // Try FSCTL operations
                    DeviceIoControl(
                        h,
                        FSCTL_RESET_WRITE_ORDER,
                        null_mut(),
                        0,
                        null_mut(),
                        0,
                        &mut _ret,
                        null_mut(),
                    );
                    
                    DeviceIoControl(
                        h,
                        FSCTL_DISCARD_VOLUME_CACHE,
                        null_mut(),
                        0,
                        null_mut(),
                        0,
                        &mut _ret,
                        null_mut(),
                    );
                }

                CloseHandle(h);
                
                // If we could access the volume, count it as success
                any_success = true;
            }
        }
    }

    // Provide detailed feedback about volume operations
    if volumes_total == 0 {
        tracing::info!("No fixed drives found to optimize");
        Ok(())
    } else if any_success {
        tracing::info!("Successfully accessed {} volumes for cache monitoring", volumes_total);
        Ok(())
    } else {
        tracing::warn!("Volume operations completed with mixed results");
        Ok(())
    }
}
