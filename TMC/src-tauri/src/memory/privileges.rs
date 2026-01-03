use anyhow::{bail, Context, Result};
use std::ptr::null_mut;
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, HANDLE, LUID},
    Security::{
        AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
        TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
    },
    System::Threading::GetCurrentProcess,
};

extern "system" {
    fn OpenProcessToken(ProcessHandle: HANDLE, DesiredAccess: u32, TokenHandle: *mut HANDLE)
        -> i32;
}

fn to_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub fn ensure_privilege(name: &str) -> Result<()> {
    unsafe {
        let process: HANDLE = GetCurrentProcess();
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(process, TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token) == 0 {
            bail!("OpenProcessToken failed: {}", GetLastError());
        }
        let mut luid: LUID = LUID {
            LowPart: 0,
            HighPart: 0,
        };
        let name_w = to_wide(name);
        if LookupPrivilegeValueW(null_mut(), name_w.as_ptr(), &mut luid) == 0 {
            let err = GetLastError();
            CloseHandle(token);
            bail!("LookupPrivilegeValueW({name}) failed: {}", err);
        }
        let mut tp = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            }],
        };
        let ok = AdjustTokenPrivileges(token, 0, &mut tp, 0, null_mut(), null_mut());
        let last = GetLastError();
        CloseHandle(token);
        if ok == 0 || last != 0 {
            bail!("AdjustTokenPrivileges({name}) failed: {}", last);
        }
    }
    Ok(())
}

pub fn ensure_privileges(names: &[&str]) -> Result<()> {
    for n in names {
        ensure_privilege(n).with_context(|| format!("ensuring privilege {}", n))?;
    }
    Ok(())
}
