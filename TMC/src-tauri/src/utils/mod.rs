/// Verifica se il processo corrente è eseguito con privilegi amministratore
#[cfg(windows)]
pub fn is_app_elevated() -> bool {
    unsafe {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, HANDLE},
            Security::TOKEN_QUERY,
            System::Threading::GetCurrentProcess,
        };
        
        #[repr(C)]
        struct TokenElevation {
            TokenIsElevated: u32,
        }
        
        const TokenElevation: u32 = 20; // TOKEN_INFORMATION_CLASS::TokenElevation
        
        extern "system" {
            fn OpenProcessToken(
                ProcessHandle: HANDLE,
                DesiredAccess: u32,
                TokenHandle: *mut HANDLE,
            ) -> i32;
            
            fn GetTokenInformation(
                TokenHandle: HANDLE,
                TokenInformationClass: u32,
                TokenInformation: *mut std::ffi::c_void,
                TokenInformationLength: u32,
                ReturnLength: *mut u32,
            ) -> i32;
        }
        
        let process = GetCurrentProcess();
        let mut token: HANDLE = std::ptr::null_mut();
        
        if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        
        // Usa scopeguard per garantire la chiusura del token
        let _guard = scopeguard::guard(token, |t| {
            if !t.is_null() {
                CloseHandle(t);
            }
        });
        
        let mut elevation = TokenElevation { TokenIsElevated: 0 };
        let mut ret_len = 0u32;
        
        let success = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut std::ffi::c_void,
            std::mem::size_of::<TokenElevation>() as u32,
            &mut ret_len,
        ) != 0;
        
        success && elevation.TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
pub fn is_app_elevated() -> bool {
    false
}

