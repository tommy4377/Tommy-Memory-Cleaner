use anyhow::Result;
use windows_sys::Win32::{
    Foundation::GetLastError,
    System::Threading::{
        GetCurrentProcess, GetCurrentThread, SetPriorityClass, SetThreadPriority,
        SetThreadPriorityBoost, HIGH_PRIORITY_CLASS, IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
        THREAD_PRIORITY_HIGHEST, THREAD_PRIORITY_LOWEST, THREAD_PRIORITY_NORMAL,
    },
};

use crate::config::Priority;

pub fn set_priority(p: Priority) -> Result<()> {
    unsafe {
        let proc = GetCurrentProcess();
        let thr = GetCurrentThread();

        match p {
            Priority::Low => {
                if SetThreadPriorityBoost(thr, 1) == 0 {
                    tracing::debug!("SetThreadPriorityBoost failed: {}", GetLastError());
                }
                if SetThreadPriority(thr, THREAD_PRIORITY_LOWEST as i32) == 0 {
                    tracing::warn!("SetThreadPriority(LOWEST) failed: {}", GetLastError());
                }
                if SetPriorityClass(proc, IDLE_PRIORITY_CLASS) == 0 {
                    tracing::warn!("SetPriorityClass(IDLE) failed: {}", GetLastError());
                }
            }
            Priority::Normal => {
                if SetThreadPriorityBoost(thr, 0) == 0 {
                    tracing::debug!("SetThreadPriorityBoost failed: {}", GetLastError());
                }
                if SetThreadPriority(thr, THREAD_PRIORITY_NORMAL as i32) == 0 {
                    tracing::warn!("SetThreadPriority(NORMAL) failed: {}", GetLastError());
                }
                if SetPriorityClass(proc, NORMAL_PRIORITY_CLASS) == 0 {
                    tracing::warn!("SetPriorityClass(NORMAL) failed: {}", GetLastError());
                }
            }
            Priority::High => {
                if SetThreadPriorityBoost(thr, 0) == 0 {
                    tracing::debug!("SetThreadPriorityBoost failed: {}", GetLastError());
                }
                if SetThreadPriority(thr, THREAD_PRIORITY_HIGHEST as i32) == 0 {
                    tracing::warn!("SetThreadPriority(HIGHEST) failed: {}", GetLastError());
                }
                if SetPriorityClass(proc, HIGH_PRIORITY_CLASS) == 0 {
                    tracing::warn!("SetPriorityClass(HIGH) failed: {}", GetLastError());
                }
            }
        }
    }
    Ok(())
}
