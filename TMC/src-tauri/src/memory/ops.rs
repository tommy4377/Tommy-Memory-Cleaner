#![allow(non_camel_case_types)]

// NOTE: This module uses Windows system APIs that may be flagged by antivirus as "suspicious",
// but these are LEGITIMATE and necessary APIs for a professional memory cleaner:
//
// - K32EmptyWorkingSet: Microsoft documented API to empty a process's working set
//   (https://docs.microsoft.com/en-us/windows/win32/api/psapi/nf-psapi-emptyworkingset)
//   Used by many legitimate commercial memory cleaners (MemReduct, CleanMem, etc.)
//
// - NtSetSystemInformation: Documented NT API for system optimizations
//   Used with SYSTEM_MEMORY_LIST_INFORMATION to empty standby/modified memory lists
//   Requires administrator privileges (normal behavior for memory cleaners)
//
// - SetSystemFileCacheSize: Microsoft documented API to manage file system cache
//   (https://docs.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-setsystemfilecachesize)
//   Allows limiting file system cache to free physical memory
//
// All these APIs are officially documented by Microsoft and used by legitimate software.
// Antivirus false positives are common for unsigned software that uses system APIs.

use crate::memory::privileges::ensure_privileges;
use crate::memory::types::{mk_stats, MemoryInfo};
use anyhow::{bail, Result};
use std::{ffi::OsString, mem, os::windows::ffi::OsStringExt, ptr};
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::ProcessStatus::K32EmptyWorkingSet;
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA,
};

use ntapi::ntexapi::NtSetSystemInformation;
use windows_sys::Win32::System::Memory::SetSystemFileCacheSize;

use crate::memory::critical_processes::is_critical_process;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use parking_lot::RwLock;
use std::time::{Duration, Instant};

pub const SYS_MEMORY_LIST_INFORMATION: u32 = 80;
const SYS_COMBINE_PHYSICAL_MEMORY_INFORMATION: u32 = 101;

const MEM_EMPTY_WORKING_SETS: u32 = 2;

const SE_DEBUG_NAME: &str = "SeDebugPrivilege";
const SE_INC_QUOTA_NAME: &str = "SeIncreaseQuotaPrivilege";
const SE_PROFILE_SINGLE_PROCESS_NAME: &str = "SeProfileSingleProcessPrivilege";

#[repr(C)]
struct MEMORY_COMBINE_INFORMATION_EX {
    handle: usize,
    pages_combined: usize,
    flags: u64,
}

// Cache for process list
struct ProcessCache {
    list: Vec<(u32, String)>,
    last_update: Instant,
}

static PROCESS_CACHE: Lazy<RwLock<ProcessCache>> = Lazy::new(|| {
    RwLock::new(ProcessCache {
        list: Vec::new(),
        last_update: Instant::now() - Duration::from_secs(60),
    })
});

/// Get Global Memory Status Extended
fn gmse() -> Result<MEMORYSTATUSEX> {
    unsafe {
        let mut st: MEMORYSTATUSEX = std::mem::zeroed();
        st.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut st) == 0 {
            bail!("GlobalMemoryStatusEx failed");
        }
        Ok(st)
    }
}

/// Get current memory information
/// Returns physical and commit memory statistics
pub fn memory_info() -> Result<MemoryInfo> {
    let st = gmse()?;
    let phys_free = st.ullAvailPhys;
    let phys_total = st.ullTotalPhys;
    let load = st.dwMemoryLoad as u32;
    let commit_free = st.ullAvailPageFile;
    let commit_total = st.ullTotalPageFile;

    Ok(MemoryInfo {
        physical: mk_stats(phys_free as u64, phys_total as u64, Some(load as u8)),
        commit: mk_stats(commit_free as u64, commit_total as u64, None),
        load_percent: load,
    })
}

/// Make NT system call with u32 command
pub fn nt_call_u32(class: u32, command: u32) -> Result<()> {
    // FIX: Retry logic for antivirus compatibility
    const MAX_RETRIES: u32 = 3;
    let mut last_error = 0i32;

    for attempt in 1..=MAX_RETRIES {
        unsafe {
            let mut cmd = command;
            let status =
                NtSetSystemInformation(class, (&mut cmd as *mut u32) as _, size_of::<u32>() as u32);

            if status >= 0 {
                if attempt > 1 {
                    tracing::info!("NtSetSystemInformation succeeded on attempt {}", attempt);
                }
                return Ok(());
            }

            last_error = status;

            // Alcuni errori comuni che indicano blocco antivirus
            match status {
                -1073741823i32 => {
                    // STATUS_UNSUCCESSFUL (0xC0000001)
                    if attempt < MAX_RETRIES {
                        tracing::debug!("NtSetSystemInformation blocked (possible antivirus), retrying (attempt {})...", attempt);
                        std::thread::sleep(std::time::Duration::from_millis(100 * attempt as u64));
                        continue;
                    }
                }
                -1073741790i32 => {
                    // STATUS_ACCESS_DENIED (0xC0000022)
                    if attempt < MAX_RETRIES {
                        tracing::debug!(
                            "NtSetSystemInformation access denied, retrying (attempt {})...",
                            attempt
                        );
                        std::thread::sleep(std::time::Duration::from_millis(100 * attempt as u64));
                        continue;
                    }
                }
                _ => {
                    // Altri errori, non retry
                    break;
                }
            }
        }
    }

    // FIX #4: Ritorna un errore invece di sempre Ok(())
    // Le funzioni chiamanti gestiranno l'errore come warning e continueranno
    let error_msg = format!(
        "NtSetSystemInformation(class={}, cmd={}) failed after {} attempts: 0x{:x}",
        class, command, MAX_RETRIES, last_error
    );
    tracing::warn!("{}", error_msg);
    Err(anyhow::anyhow!("{}", error_msg))
}

pub fn optimize_standby_list(low_priority: bool) -> Result<()> {
    optimize_standby_list_with_stealth(low_priority, false)
}

/// Optimize standby list with optional stealth mode (indirect syscalls)
pub fn optimize_standby_list_with_stealth(low_priority: bool, use_stealth: bool) -> Result<()> {
    ensure_privileges(&[SE_PROFILE_SINGLE_PROCESS_NAME])?;
    
    // Use the original implementation to avoid recursion
    crate::antivirus::whitelist::safe_memory_operation(|| {
        // Try advanced function first, then fallback to standard
        if low_priority {
            if use_stealth {
                // Try stealth optimization for low priority standby
                match crate::memory::advanced::purge_standby_list_low_priority_stealth() {
                    Ok(_) => {
                        tracing::info!("✓ Advanced low priority standby list purge successful (stealth mode)");
                        Ok(())
                    }
                    Err(e) => {
                        tracing::warn!("⚠ Advanced low priority standby purge failed ({}), using standard API", e);
                        let cmd = MEM_EMPTY_WORKING_SETS + 1; // Different command for low priority
                        let result = nt_call_u32(SYS_MEMORY_LIST_INFORMATION, cmd);
                        if result.is_ok() {
                            tracing::info!("✓ Low priority standby list purged (standard API)");
                        }
                        Ok(())
                    }
                }
            } else {
                match crate::memory::advanced::purge_standby_list_low_priority() {
                    Ok(_) => {
                        tracing::info!("✓ Advanced low priority standby list purge successful");
                        Ok(())
                    }
                    Err(e) => {
                        tracing::warn!("⚠ Advanced low priority standby purge failed ({}), using standard API", e);
                        let cmd = MEM_EMPTY_WORKING_SETS + 1; // Different command for low priority
                        let result = nt_call_u32(SYS_MEMORY_LIST_INFORMATION, cmd);
                        if result.is_ok() {
                            tracing::info!("✓ Low priority standby list purged (standard API)");
                        }
                        Ok(())
                    }
                }
            }
        } else {
            if use_stealth {
                // Try stealth optimization for standby list
                match crate::memory::advanced::purge_standby_list_stealth() {
                    Ok(_) => {
                        tracing::info!("✓ Advanced standby list purge successful (stealth mode)");
                        Ok(())
                    }
                    Err(e) => {
                        tracing::warn!("⚠ Advanced standby purge failed ({}), using standard API", e);
                        let result = nt_call_u32(SYS_MEMORY_LIST_INFORMATION, MEM_EMPTY_WORKING_SETS + 1);
                        if result.is_ok() {
                            tracing::info!("✓ Standby list purged (standard API)");
                        }
                        Ok(())
                    }
                }
            } else {
                match crate::memory::advanced::purge_standby_list() {
                    Ok(_) => {
                        tracing::info!("✓ Advanced standby list purge successful");
                        Ok(())
                    }
                    Err(e) => {
                        tracing::warn!("⚠ Advanced standby purge failed ({}), using standard API", e);
                        let result = nt_call_u32(SYS_MEMORY_LIST_INFORMATION, MEM_EMPTY_WORKING_SETS + 1);
                        if result.is_ok() {
                            tracing::info!("✓ Standby list purged (standard API)");
                        }
                        Ok(())
                    }
                }
            }
        }
    })
}

/// Optimize modified page list with optional stealth mode
pub fn optimize_modified_page_list_with_stealth(use_stealth: bool) -> Result<()> {
    ensure_privileges(&[SE_PROFILE_SINGLE_PROCESS_NAME])?;
    
    // Use the original implementation to avoid recursion
    crate::antivirus::whitelist::safe_memory_operation(|| {
        if use_stealth {
            // Try stealth optimization for modified page list
            match crate::memory::advanced::aggressive_modified_page_flush_stealth() {
                Ok(_) => {
                    tracing::info!("✓ Advanced modified page list flush successful (stealth mode)");
                    Ok(())
                }
                Err(e) => {
                    tracing::warn!("⚠ Advanced modified page flush failed ({}), using standard API", e);
                    nt_call_u32(SYS_MEMORY_LIST_INFORMATION, 3) // MEM_FLUSH_MODIFIED_LIST equivalent
                }
            }
        } else {
            // Try advanced aggressive flush first
            match crate::memory::advanced::aggressive_modified_page_flush() {
                Ok(_) => {
                    tracing::info!("✓ Advanced modified page list flush successful");
                    Ok(())
                }
                Err(e) => {
                    tracing::warn!("⚠ Advanced modified page flush failed ({}), using standard API", e);
                    nt_call_u32(SYS_MEMORY_LIST_INFORMATION, 3) // MEM_FLUSH_MODIFIED_LIST equivalent
                }
            }
        }
    })
}

pub fn optimize_registry_cache() -> Result<()> {
    // Use the original implementation to avoid recursion
    crate::antivirus::whitelist::safe_memory_operation(|| {
        // Try advanced registry optimization first
        match crate::memory::advanced::optimize_registry_cache() {
            Ok(_) => {
                tracing::info!("✓ Advanced registry optimization successful");
                Ok(())
            }
            Err(e) => {
                tracing::warn!("⚠ Advanced registry optimization failed ({}), using standard API", e);
                unsafe {
                    let status = ntapi::ntexapi::NtSetSystemInformation(
                        155, // SYS_REGISTRY_RECONCILIATION_INFORMATION
                        ptr::null_mut(),
                        0,
                    );
                    if status < 0 {
                        tracing::warn!("Registry cache optimization not available: 0x{:x}", status);
                    }
                    Ok(())
                }
            }
        }
    })
}

pub fn optimize_system_file_cache() -> Result<()> {
    ensure_privileges(&[SE_INC_QUOTA_NAME])?;
    crate::antivirus::whitelist::safe_memory_operation(|| -> Result<(), anyhow::Error> {
        unsafe {
            // Get total memory to determine optimal cache limits
            let st = gmse()?;
            let total_gb = st.ullTotalPhys / (1024 * 1024 * 1024);
            let available_gb = st.ullAvailPhys / (1024 * 1024 * 1024);
            
            // Dynamic limits based on BOTH total and available RAM
            let (min_size, max_size) = if total_gb <= 8 {
                // Systems with 8GB or less RAM - more conservative
                (8 * 1024 * 1024, 128 * 1024 * 1024) // 8MB - 128MB
            } else if total_gb <= 16 {
                // Systems with 16GB RAM - balanced
                (16 * 1024 * 1024, 256 * 1024 * 1024) // 16MB - 256MB
            } else if available_gb >= 8 {
                // High-end systems with plenty of available RAM
                (32 * 1024 * 1024, 512 * 1024 * 1024) // 32MB - 512MB
            } else {
                // High-end systems but low available RAM - be conservative
                (16 * 1024 * 1024, 256 * 1024 * 1024) // 16MB - 256MB
            };
            
            tracing::debug!(
                "Setting file cache limits: min={}MB, max={}MB (total RAM: {}GB, available: {}GB)",
                min_size / (1024 * 1024),
                max_size / (1024 * 1024),
                total_gb,
                available_gb
            );
            
            // First try to flush completely, then set limits
            let minus_one = usize::MAX;
            if SetSystemFileCacheSize(minus_one, minus_one, 0) == 0 {
                tracing::warn!("Complete cache flush failed, trying with limits...");
            }
            
            // Set optimal limits based on available RAM
            if SetSystemFileCacheSize(min_size, max_size, 0) == 0 {
                tracing::warn!("SetSystemFileCacheSize with limits failed, continuing...");
                // Non far crashare
                return Ok(());
            }
        }
        Ok(())
    })
}

#[cfg(target_os = "windows")]
pub fn process_list() -> Vec<(u32, String)> {
    const CACHE_DURATION: Duration = Duration::from_secs(5);

    // Double-checked locking pattern to avoid race conditions
    {
        let cache = PROCESS_CACHE.read();
        if cache.last_update.elapsed() < CACHE_DURATION {
            return cache.list.clone();
        }
    } // Read lock released here

    // Update cache - acquire write lock only if needed
    // Check again after acquiring write lock
    let mut cache = PROCESS_CACHE.write();
    if cache.last_update.elapsed() < CACHE_DURATION {
        // Another thread updated while we waited for write lock
        return cache.list.clone();
    }
    
    // Now update the cache
    let processes = fetch_process_list();
    cache.list = processes.clone();
    cache.last_update = Instant::now();

    processes
}

/// Helper function to fetch process list from system
#[cfg(target_os = "windows")]
fn fetch_process_list() -> Vec<(u32, String)> {
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let mut out = Vec::with_capacity(256);

    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return out;
        }

        struct HandleGuard(HANDLE);
        impl Drop for HandleGuard {
            fn drop(&mut self) {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
        let _guard = HandleGuard(snap);

        let mut pe: PROCESSENTRY32W = std::mem::zeroed();
        pe.dwSize = size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snap, &mut pe) != 0 {
            loop {
                let len = pe
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(pe.szExeFile.len());

                if len > 0 {
                    let name = OsString::from_wide(&pe.szExeFile[..len])
                        .to_string_lossy()
                        .to_lowercase()
                        .replace(".exe", "");
                    out.push((pe.th32ProcessID, name));
                }

                if Process32NextW(snap, &mut pe) == 0 {
                    break;
                }
            }
        }
    }

    out
}

/// Empty working set for a specific process
fn empty_ws_process(pid: u32) -> bool {
    // IMPORTANT: This function requires SE_DEBUG_NAME to work correctly
    // On system processes. Ensure it has been acquired BEFORE calling this function.
    const MAX_RETRIES: u32 = 2;

    for attempt in 1..=MAX_RETRIES {
        unsafe {
            // Use PROCESS_ALL_ACCESS if available, otherwise minimum required permissions
            let h: HANDLE = OpenProcess(PROCESS_SET_QUOTA | PROCESS_QUERY_INFORMATION, 0, pid);

            // HANDLE in windows-sys is isize, so compare with 0
            if h.is_null() {
                let error = GetLastError();
                // ERROR_ACCESS_DENIED (0x5) is common if SE_DEBUG_NAME is not acquired
                if error == 5 {
                    tracing::debug!(
                        "Access denied for process {} - SE_DEBUG_NAME privilege may be missing",
                        pid
                    );
                }

                if attempt < MAX_RETRIES {
                    tracing::debug!(
                        "Failed to open process {} (attempt {}): 0x{:x}, retrying...",
                        pid,
                        attempt,
                        error
                    );
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    continue;
                } else {
                    tracing::debug!("Failed to open process {} after {} attempts: 0x{:x} (ACCESS_DENIED=0x5 means SE_DEBUG_NAME missing)", pid, MAX_RETRIES, error);
                    return false;
                }
            }

            let result = K32EmptyWorkingSet(h) != 0;
            CloseHandle(h);

            // If successful, return immediately
            if result {
                return true;
            }

            // If it's the last attempt, return false
            if attempt >= MAX_RETRIES {
                return false;
            }

            // Retry if it fails
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    false
}

/// Optimize working set with optional stealth mode
pub fn optimize_working_set_with_stealth(exclusions: &[String], use_stealth: bool) -> Result<()> {
    ensure_privileges(&[SE_DEBUG_NAME])?;
    
    crate::antivirus::whitelist::safe_memory_operation(|| {
        if use_stealth {
            // Try stealth optimization for working set
            match crate::memory::advanced::empty_working_set_stealth(exclusions) {
                Ok(_) => {
                    tracing::info!("✓ Working Set optimization successful (stealth mode with indirect syscalls)");
                    Ok(())
                }
                Err(e) => {
                    tracing::warn!("⚠ Stealth Working Set optimization failed ({}), using standard API", e);
                    // Fallback to standard implementation
                    optimize_working_set_standard(exclusions)
                }
            }
        } else {
            // Use standard implementation
            optimize_working_set_standard(exclusions)
        }
    })
}

/// Standard working set optimization without stealth
fn optimize_working_set_standard(exclusions: &[String]) -> Result<()> {
    // IMPORTANT: Always acquire SE_DEBUG_NAME to allow access to all processes
    // Even if we use the global method, SE_DEBUG_NAME ensures it works on all processes
    ensure_privileges(&[SE_DEBUG_NAME, SE_PROFILE_SINGLE_PROCESS_NAME])?;

    // Get foreground window PID to exclude it (prevents FPS drops in games)
    let foreground_pid = get_foreground_process_pid();
    
    // Convert exclusions to lowercase for comparison
    let exclusions_lower: Vec<String> = exclusions.iter().map(|s| s.to_lowercase()).collect();

    // If there are no custom exclusions, use fast global optimization
    // This method requires SE_DEBUG_NAME to work correctly on system processes
    if exclusions_lower.is_empty() {
        return crate::antivirus::whitelist::safe_memory_operation(|| {
            nt_call_u32(SYS_MEMORY_LIST_INFORMATION, MEM_EMPTY_WORKING_SETS)
        });
    }

    // Create HashSet for user exclusions
    let user_exclusions: HashSet<&str> = exclusions_lower.iter().map(|s| s.as_str()).collect();

    let processes = process_list();
    let mut success_count = 0;
    let mut skip_count = 0;
    let mut critical_skip = 0;
    let mut foreground_skip = 0;

    for (pid, name) in processes {
        // FIRST check if it's the foreground process
        if Some(pid) == foreground_pid {
            tracing::debug!("Skipping foreground process {} (PID: {})", name, pid);
            foreground_skip += 1;
            continue;
        }

        // THEN check if it's a critical process
        if is_critical_process(&name) {
            critical_skip += 1;
            continue;
        }

        // THEN check user exclusions
        if user_exclusions.contains(name.as_str()) {
            skip_count += 1;
            continue;
        }

        if empty_ws_process(pid) {
            success_count += 1;
        }
    }

    tracing::debug!(
        "Working set optimization: {} cleaned, {} user excluded, {} critical protected, {} foreground protected",
        success_count,
        skip_count,
        critical_skip,
        foreground_skip
    );

    Ok(())
}

pub fn optimize_combined_page_list() -> Result<()> {
    // First ensure privileges are correct
    ensure_privileges(&[
        SE_PROFILE_SINGLE_PROCESS_NAME,
        SE_DEBUG_NAME, // Also add this for Gaming mode
    ])?;

    // FIX: Use has_combined_page_list() function instead of checking manually
    // This uses RtlGetVersion which is more reliable
    if !crate::os::has_combined_page_list() {
        tracing::info!("Combined page list not available on this Windows version, skipping");
        return Ok(());
    }

    // Use safe_memory_operation to avoid antivirus detections
    crate::antivirus::whitelist::safe_memory_operation(|| -> Result<(), anyhow::Error> {
        ensure_privileges(&[SE_PROFILE_SINGLE_PROCESS_NAME])?;

        unsafe {
            let mut info = MEMORY_COMBINE_INFORMATION_EX {
                handle: 0,
                pages_combined: 0,
                flags: 0,
            };

            let status = NtSetSystemInformation(
                SYS_COMBINE_PHYSICAL_MEMORY_INFORMATION,
                &mut info as *mut _ as _,
                mem::size_of::<MEMORY_COMBINE_INFORMATION_EX>() as u32,
            );

            if status < 0 {
                // Check for Windows 11 24H2+ compatibility issue
                if status as u32 == 0xC0000003 {
                    // STATUS_INVALID_INFO_CLASS - Windows 11 24H2+ changed the API
                    tracing::debug!(
                        "Combined page list not supported on Windows 11 24H2+ (STATUS_INVALID_INFO_CLASS). \
                        This is expected and not an error."
                    );
                    return Ok(());
                }
                
                tracing::warn!(
                    "Combined page list optimization failed: 0x{:x} (this may be normal on newer Windows versions)",
                    status
                );
                return Ok(()); // Don't fail the entire optimization
            }

            tracing::info!("Combined {} pages", info.pages_combined);
        }

        Ok(())
    })
}

/// Get the PID of the foreground window process
#[cfg(target_os = "windows")]
fn get_foreground_process_pid() -> Option<u32> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        Some(pid)
    }
}

#[cfg(not(target_os = "windows"))]
fn get_foreground_process_pid() -> Option<u32> {
    None
}

pub fn list_process_names() -> Vec<String> {
    let mut names: Vec<String> = process_list().into_iter().map(|(_, n)| n).collect();
    names.sort();
    names.dedup();
    names
}
