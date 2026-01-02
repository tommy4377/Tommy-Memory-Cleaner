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
use std::{ffi::OsString, mem::size_of, os::windows::ffi::OsStringExt};
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
use std::sync::RwLock;
use std::time::{Duration, Instant};

const SYS_MEMORY_LIST_INFORMATION: u32 = 80;
const SYS_PROCESS_INFORMATION: u32 = 5;
const SYS_FILE_CACHE_INFORMATION: u32 = 21;
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
fn nt_call_u32(class: u32, command: u32) -> Result<()> {
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
    ensure_privileges(&[SE_PROFILE_SINGLE_PROCESS_NAME])?;
    
    // Usa la funzione avanzata per purge standby list
    crate::antivirus::whitelist::safe_memory_operation(|| {
        if low_priority {
            crate::memory::advanced::purge_standby_list_low_priority()
        } else {
            crate::memory::advanced::purge_standby_list()
        }
    })
}

pub fn optimize_modified_page_list() -> Result<()> {
    ensure_privileges(&[SE_PROFILE_SINGLE_PROCESS_NAME])?;
    
    // Usa la funzione avanzata per modified page list
    crate::antivirus::whitelist::safe_memory_operation(|| {
        crate::memory::advanced::aggressive_modified_page_flush()
    })
}

pub fn optimize_registry_cache() -> Result<()> {
    // Usa la funzione avanzata per registry cache
    crate::antivirus::whitelist::safe_memory_operation(|| {
        crate::memory::advanced::optimize_registry_cache()
    })
}

pub fn optimize_system_file_cache() -> Result<()> {
    ensure_privileges(&[SE_INC_QUOTA_NAME])?;
    crate::antivirus::whitelist::safe_memory_operation(|| -> Result<(), anyhow::Error> {
        unsafe {
            let minus_one = usize::MAX;
            if SetSystemFileCacheSize(minus_one, minus_one, 0) == 0 {
                tracing::warn!("SetSystemFileCacheSize failed, continuing...");
                // Non far crashare
                return Ok(());
            }
        }
        Ok(())
    })
}

#[cfg(target_os = "windows")]
fn process_list() -> Vec<(u32, String)> {
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    const CACHE_DURATION: Duration = Duration::from_secs(5);

    // Try read cache first
    {
        let cache = match PROCESS_CACHE.read() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        if cache.last_update.elapsed() < CACHE_DURATION {
            return cache.list.clone();
        }
    }

    // Update cache
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

    // Update cache
    if let Ok(mut cache) = PROCESS_CACHE.write() {
        cache.list = out.clone();
        cache.last_update = Instant::now();
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

/// Optimize working set for all non-critical processes
pub fn optimize_working_set(exclusions_lower: &[String]) -> Result<()> {
    // IMPORTANT: Always acquire SE_DEBUG_NAME to allow access to all processes
    // Even if we use the global method, SE_DEBUG_NAME ensures it works on all processes
    ensure_privileges(&[SE_DEBUG_NAME, SE_PROFILE_SINGLE_PROCESS_NAME])?;

    // Get foreground window PID to exclude it (prevents FPS drops in games)
    let foreground_pid = get_foreground_process_pid();

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
                (&mut info as *mut MEMORY_COMBINE_INFORMATION_EX) as _,
                std::mem::size_of::<MEMORY_COMBINE_INFORMATION_EX>() as u32,
            );

            if status < 0 {
                // Non far crashare, solo log warning e continua
                tracing::warn!(
                    "Combined page list optimization not available on this system (0x{:x})",
                    status
                );
                return Ok(());
            }
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
