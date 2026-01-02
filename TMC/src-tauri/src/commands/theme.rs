#[tauri::command]
pub fn cmd_get_system_theme() -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::System::Registry::*;

        let key_path: Vec<u16> =
            OsStr::new(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

        let mut hkey: HKEY = 0;
        let value_name: Vec<u16> = OsStr::new("AppsUseLightTheme")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let result =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, key_path.as_ptr(), 0, KEY_READ, &mut hkey) };

        // HKEY in windows-sys is isize, so compare with 0
        if result == 0 && hkey != 0 {
            let mut value_data: u32 = 0;
            let mut value_type: u32 = 0;
            let mut data_size: u32 = std::mem::size_of::<u32>() as u32;

            let read_result = unsafe {
                RegQueryValueExW(
                    hkey,
                    value_name.as_ptr(),
                    std::ptr::null_mut(),
                    &mut value_type,
                    &mut value_data as *mut _ as *mut u8,
                    &mut data_size,
                )
            };

            unsafe {
                RegCloseKey(hkey);
            }

            if read_result == 0 && value_type == REG_DWORD {
                // 0 = dark, 1 = light
                return Ok(if value_data == 0 {
                    "dark".to_string()
                } else {
                    "light".to_string()
                });
            }
        }
    }

    // Default a dark se non riusciamo a rilevare
    Ok("dark".to_string())
}

#[tauri::command]
pub fn cmd_get_system_language() -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::System::Registry::*;

        // Leggi la lingua dal registro Windows
        let key_path: Vec<u16> = OsStr::new(r"Control Panel\International")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut hkey: HKEY = 0;
        let value_name: Vec<u16> = OsStr::new("LocaleName")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let result =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, key_path.as_ptr(), 0, KEY_READ, &mut hkey) };

        // HKEY in windows-sys is isize, so compare with 0
        if result == 0 && hkey != 0 {
            let mut value_data = [0u16; 85];
            let mut value_type: u32 = 0;
            let mut data_size: u32 = (value_data.len() * 2) as u32;

            let read_result = unsafe {
                RegQueryValueExW(
                    hkey,
                    value_name.as_ptr(),
                    std::ptr::null_mut(),
                    &mut value_type,
                    value_data.as_mut_ptr() as *mut u8,
                    &mut data_size,
                )
            };

            unsafe {
                RegCloseKey(hkey);
            }

            if read_result == 0 && value_type == REG_SZ {
                // Trova la fine della stringa (primo null)
                let len = value_data
                    .iter()
                    .position(|&x| x == 0)
                    .unwrap_or(value_data.len());
                let locale_str = String::from_utf16_lossy(&value_data[..len]);

                // Estrai il codice lingua (es. "it-IT" -> "it", "en-US" -> "en")
                let lang_code = locale_str.split('-').next().unwrap_or("en").to_lowercase();

                // Mappa i codici lingua supportati
                match lang_code.as_str() {
                    "it" => return Ok("it".to_string()),
                    "es" => return Ok("es".to_string()),
                    "fr" => return Ok("fr".to_string()),
                    "pt" => return Ok("pt".to_string()),
                    "de" => return Ok("de".to_string()),
                    "ar" => return Ok("ar".to_string()),
                    "ja" => return Ok("ja".to_string()),
                    "zh" => return Ok("zh".to_string()),
                    _ => return Ok("en".to_string()),
                }
            }
        }
    }

    // Default a inglese se non riusciamo a rilevare
    Ok("en".to_string())
}
