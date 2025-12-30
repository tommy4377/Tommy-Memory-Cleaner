use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OsVersion { 
    pub major: u32, 
    pub minor: u32, 
    pub build: u32 
}

fn get_windows_version() -> OsVersion {
    // FIX: GetVersionExW è deprecato e può restituire informazioni errate su Windows 8+
    // Usa RtlGetVersion che è più affidabile
    unsafe {
        #[repr(C)]
        struct RTL_OSVERSIONINFOEXW {
            dw_os_version_info_size: u32,
            dw_major_version: u32,
            dw_minor_version: u32,
            dw_build_number: u32,
            dw_platform_id: u32,
            sz_csd_version: [u16; 128],
            w_service_pack_major: u16,
            w_service_pack_minor: u16,
            w_suite_mask: u16,
            w_product_type: u8,
            w_reserved: u8,
        }
        
        type RtlGetVersionFn = unsafe extern "system" fn(*mut RTL_OSVERSIONINFOEXW) -> i32;
        
        // Carica ntdll.dll e ottieni RtlGetVersion
        use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
        use std::ptr::null_mut;
        
        let ntdll_name = b"ntdll.dll\0";
        let ntdll = GetModuleHandleA(ntdll_name.as_ptr());
        // GetModuleHandleA restituisce un HMODULE che è un alias per *mut c_void
        // Confronta direttamente con null_mut()
        if ntdll != null_mut() {
            let rtl_get_version_name = b"RtlGetVersion\0";
            if let Some(rtl_get_version) = GetProcAddress(ntdll, rtl_get_version_name.as_ptr()) {
                let rtl_get_version_fn: RtlGetVersionFn = std::mem::transmute(rtl_get_version);
                let mut os_info: RTL_OSVERSIONINFOEXW = std::mem::zeroed();
                os_info.dw_os_version_info_size = std::mem::size_of::<RTL_OSVERSIONINFOEXW>() as u32;
                
                if rtl_get_version_fn(&mut os_info) == 0 {
                    tracing::info!("RtlGetVersion: Windows {}.{}.{}", 
                        os_info.dw_major_version, os_info.dw_minor_version, os_info.dw_build_number);
                    return OsVersion {
                        major: os_info.dw_major_version,
                        minor: os_info.dw_minor_version,
                        build: os_info.dw_build_number,
                    };
                }
            }
        }
        
        // Fallback a GetVersionExW se RtlGetVersion non è disponibile
        use windows_sys::Win32::System::SystemInformation::{GetVersionExW, OSVERSIONINFOEXW};
        let mut os_info: OSVERSIONINFOEXW = std::mem::zeroed();
        os_info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOEXW>() as u32;
        
        if GetVersionExW(&mut os_info as *mut _ as *mut _) != 0 {
            tracing::warn!("GetVersionExW (may be inaccurate): Windows {}.{}.{}", 
                os_info.dwMajorVersion, os_info.dwMinorVersion, os_info.dwBuildNumber);
            // Se GetVersionExW restituisce 6.2, probabilmente è Windows 10/11
            // Assumiamo Windows 10 come default sicuro
            if os_info.dwMajorVersion == 6 && os_info.dwMinorVersion == 2 {
                tracing::warn!("GetVersionExW returned 6.2 (Windows 8), assuming Windows 10+");
                OsVersion { major: 10, minor: 0, build: 19041 }
            } else {
                OsVersion {
                    major: os_info.dwMajorVersion,
                    minor: os_info.dwMinorVersion,
                    build: os_info.dwBuildNumber,
                }
            }
        } else {
            // Default a Windows 10 se non riusciamo a rilevare
            tracing::warn!("Failed to detect Windows version, defaulting to Windows 10");
            OsVersion { major: 10, minor: 0, build: 19041 }
        }
    }
}

pub fn has_standby_list() -> bool { 
    true // Disponibile su tutte le versioni Windows moderne
}

pub fn has_standby_list_low() -> bool { 
    let ver = get_windows_version();
    let result = ver.major >= 10; // Windows 10+
    tracing::debug!("has_standby_list_low: {} (Windows {}.{}.{})", result, ver.major, ver.minor, ver.build);
    result
}

pub fn has_modified_page_list() -> bool { 
    true // Sempre disponibile
}

pub fn has_registry_cache() -> bool { 
    true // Sempre disponibile
}

pub fn has_system_file_cache() -> bool { 
    true // Sempre disponibile
}

pub fn has_combined_page_list() -> bool { 
    let ver = get_windows_version();
    // Windows 10 1803+ (build 17134)
    let result = ver.major > 10 || (ver.major == 10 && ver.build >= 17134);
    tracing::debug!("has_combined_page_list: {} (Windows {}.{}.{})", result, ver.major, ver.minor, ver.build);
    result
}

pub fn has_working_set() -> bool { 
    true // Sempre disponibile
}

pub fn has_hotkey_manager() -> bool { 
    true // Sempre disponibile
}

pub fn has_modified_file_cache() -> bool {
    // MODIFIED_FILE_CACHE è disponibile solo su Windows 10 1803+ (build 17134)
    let ver = get_windows_version();
    let result = ver.major > 10 || (ver.major == 10 && ver.build >= 17134);
    tracing::debug!("has_modified_file_cache: {} (Windows {}.{}.{})", result, ver.major, ver.minor, ver.build);
    result
}