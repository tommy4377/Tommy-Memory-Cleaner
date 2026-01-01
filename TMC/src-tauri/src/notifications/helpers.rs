/// Helper to convert ICO to high-resolution PNG
#[cfg(windows)]
pub fn convert_ico_to_highres_png(ico_data: &[u8]) -> Result<Vec<u8>, String> {
    // Carica l'ICO usando image::load_from_memory che gestisce automaticamente il formato
    let img = image::load_from_memory(ico_data)
        .map_err(|e| format!("Failed to load ICO: {}", e))?;
    
    // Converti in RGBA8
    let rgba_img = img.to_rgba8();
    
    // Resize a 256x256 (alta risoluzione per Windows Toast)
    let resized = image::imageops::resize(
        &rgba_img,
        256,
        256,
        image::imageops::FilterType::Lanczos3,
    );
    
    // Codifica come PNG usando DynamicImage::save (API image 0.25)
    // Converti RgbaImage in DynamicImage per poter usare save
    let dynamic_img = image::DynamicImage::ImageRgba8(resized);
    
    // Salva in un buffer in memoria usando il metodo save_with_format
    let mut png_data = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut png_data);
        dynamic_img.write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| format!("Failed to encode PNG: {}", e))?;
    }
    
    Ok(png_data)
}

/// Helper to get the path to a notification icon (PNG high-res preferred)
/// Windows Toast works better with PNG at high resolution (128x128 or larger) instead of ICO
#[cfg(windows)]
pub fn ensure_notification_icon_available() -> Option<std::path::PathBuf> {
    use std::fs;
    
    // Prova prima a leggere PNG 128x128 dalla directory runtime (se distribuito con l'app)
    // Altrimenti usa ICO embedded e convertilo in PNG usando la libreria image
    let (icon_data, icon_ext) = {
        let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
        
        // Prova a leggere PNG dalla directory runtime (se l'app è distribuita con le icone)
        if let Ok(png_data) = fs::read(exe_dir.join("icons").join("128x128.png")) {
            (png_data, "png")
        } else if let Ok(png_data) = fs::read(exe_dir.join("128x128.png")) {
            (png_data, "png")
        } else if let Ok(png_data) = fs::read(exe_dir.join("icons").join("icon.png")) {
            (png_data, "png")
        } else if let Ok(png_data) = fs::read(exe_dir.join("icon.png")) {
            (png_data, "png")
        } else {
            // Fallback: converti ICO embedded in PNG 256x256 ad alta risoluzione
            // Questo risolve il problema della sgranatura
            // Use the icon from the icons directory at the root of src-tauri
            match convert_ico_to_highres_png(include_bytes!("../../icons/icon.ico")) {
                Ok(png_data) => {
                    tracing::debug!("Converted ICO to high-res PNG (256x256) for better notification quality");
                    (png_data, "png")
                },
                Err(e) => {
                    tracing::warn!("Failed to convert ICO to PNG, using ICO: {}", e);
                    (include_bytes!("../../icons/icon.ico").to_vec(), "ico")
                }
            }
        }
    };
    
    // Prova a salvare l'icona nella directory dati dell'app
    let icon_path = {
        let detector = crate::config::get_portable_detector();
        detector.data_dir().join(format!("icon.{}", icon_ext))
    };
    
    // Crea la directory se non esiste
    if let Some(parent) = icon_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            tracing::warn!("Failed to create icon directory: {}", e);
            return None;
        }
    }
    
    // Copia l'icona solo se non esiste o se è stata modificata
    // Controlla se il file esiste e ha la stessa dimensione
    let needs_copy = match fs::metadata(&icon_path) {
        Ok(meta) => meta.len() != icon_data.len() as u64,
        Err(_) => true, // File non esiste, devi copiarlo
    };
    
    if needs_copy {
        if let Err(e) = fs::write(&icon_path, &icon_data) {
            tracing::warn!("Failed to write notification icon: {}", e);
            return None;
        }
        tracing::debug!("Notification icon (format: {}) copied to: {}", icon_ext, icon_path.display());
    }
    
    Some(icon_path)
}

#[cfg(not(windows))]
pub fn ensure_notification_icon_available() -> Option<std::path::PathBuf> {
    None
}

use crate::config::Profile;
use crate::memory::types::Reason;

/// Get notification title based on language and reason
pub fn get_notification_title(language: &str, reason: Reason) -> String {
    match reason {
        Reason::Manual => t(language, "TMC • Optimization completed"),
        Reason::Schedule => t(language, "TMC • Scheduled optimization"),
        Reason::LowMemory => t(language, "TMC • Low memory optimization"),
    }
}

/// Get profile display name in the specified language
pub fn get_profile_display_name(profile: &Profile, language: &str) -> String {
    match profile {
        Profile::Normal => t(language, "Normal"),
        Profile::Balanced => t(language, "Balanced"),
        Profile::Gaming => t(language, "Gaming"),
    }
}

/// Get notification body with memory information
pub fn get_notification_body(language: &str, _reason: Reason, freed_mb: f64, free_gb: f64, profile: &Profile) -> String {
    let profile_name = get_profile_display_name(profile, language);
    
    // Format based on language
    match language {
        "it" => format!(
            "✅ Liberati: {:.1} MB\n🧠 RAM libera: {:.2} GB\n🎯 Profilo: {}",
            freed_mb.abs(), free_gb, profile_name
        ),
        "es" => format!(
            "✅ Liberado: {:.1} MB\n🧠 RAM libre: {:.2} GB\n🎯 Perfil: {}",
            freed_mb.abs(), free_gb, profile_name
        ),
        "fr" => format!(
            "✅ Libéré: {:.1} MB\n🧠 RAM libre: {:.2} GB\n🎯 Profil: {}",
            freed_mb.abs(), free_gb, profile_name
        ),
        "pt" => format!(
            "✅ Liberado: {:.1} MB\n🧠 RAM livre: {:.2} GB\n🎯 Perfil: {}",
            freed_mb.abs(), free_gb, profile_name
        ),
        "de" => format!(
            "✅ Freigegeben: {:.1} MB\n🧠 Freier RAM: {:.2} GB\n🎯 Profil: {}",
            freed_mb.abs(), free_gb, profile_name
        ),
        "ar" => format!(
            "✅ تم التحرير: {:.1} ميجابايت\n🧠 ذاكرة متاحة: {:.2} جيجابايت\n🎯 الملف الشخصي: {}",
            freed_mb.abs(), free_gb, profile_name
        ),
        "ja" => format!(
            "✅ 解放: {:.1} MB\n🧠 空きRAM: {:.2} GB\n🎯 プロファイル: {}",
            freed_mb.abs(), free_gb, profile_name
        ),
        "zh" => format!(
            "✅ 已释放: {:.1} MB\n🧠 可用RAM: {:.2} GB\n🎯 配置文件: {}",
            freed_mb.abs(), free_gb, profile_name
        ),
        _ => format!(
            "✅ Freed: {:.1} MB\n🧠 Free RAM: {:.2} GB\n🎯 Profile: {}",
            freed_mb.abs(), free_gb, profile_name
        )
    }
}

/// Translation function - moved from main.rs
fn t(lang: &str, key: &str) -> String {
    match (lang, key) {
        // Italiano
        ("it", "TMC • Optimization completed") => "TMC • Ottimizzazione completata",
        ("it", "TMC • Scheduled optimization") => "TMC • Ottimizzazione programmata",
        ("it", "TMC • Low memory optimization") => "TMC • Ottimizzazione per memoria bassa",
        ("it", "Normal") => "Normale",
        ("it", "Balanced") => "Bilanciato",
        ("it", "Gaming") => "Gaming",
        
        // Spagnolo
        ("es", "TMC • Optimization completed") => "TMC • Optimización completada",
        ("es", "TMC • Scheduled optimization") => "TMC • Optimización programada",
        ("es", "TMC • Low memory optimization") => "TMC • Optimización por memoria baja",
        ("es", "Normal") => "Normal",
        ("es", "Balanced") => "Equilibrado",
        ("es", "Gaming") => "Gaming",
        
        // Francese
        ("fr", "TMC • Optimization completed") => "TMC • Optimisation terminée",
        ("fr", "TMC • Scheduled optimization") => "TMC • Optimisation programmée",
        ("fr", "TMC • Low memory optimization") => "TMC • Optimisation mémoire faible",
        ("fr", "Normal") => "Normal",
        ("fr", "Balanced") => "Équilibré",
        ("fr", "Gaming") => "Gaming",
        
        // Portoghese
        ("pt", "TMC • Optimization completed") => "TMC • Otimização concluída",
        ("pt", "TMC • Scheduled optimization") => "TMC • Otimização agendada",
        ("pt", "TMC • Low memory optimization") => "TMC • Otimização por memória baixa",
        ("pt", "Normal") => "Normal",
        ("pt", "Balanced") => "Balanceado",
        ("pt", "Gaming") => "Jogos",
        
        // Tedesco
        ("de", "TMC • Optimization completed") => "TMC • Optimierung abgeschlossen",
        ("de", "TMC • Scheduled optimization") => "TMC • Geplante Optimierung",
        ("de", "TMC • Low memory optimization") => "TMC • Optimierung bei wenig Speicher",
        ("de", "Normal") => "Normal",
        ("de", "Balanced") => "Ausgeglichen",
        ("de", "Gaming") => "Spielen",
        
        // Arabo
        ("ar", "TMC • Optimization completed") => "TMC • اكتمل التحسين",
        ("ar", "TMC • Scheduled optimization") => "TMC • تحسين مجدول",
        ("ar", "TMC • Low memory optimization") => "TMC • تحسين الذاكرة المنخفضة",
        ("ar", "Normal") => "عادي",
        ("ar", "Balanced") => "متوازن",
        ("ar", "Gaming") => "الألعاب",
        
        // Giapponese
        ("ja", "TMC • Optimization completed") => "TMC • 最適化完了",
        ("ja", "TMC • Scheduled optimization") => "TMC • スケジュール最適化",
        ("ja", "TMC • Low memory optimization") => "TMC • メモリ不足最適化",
        ("ja", "Normal") => "ノーマル",
        ("ja", "Balanced") => "バランス",
        ("ja", "Gaming") => "ゲーミング",
        
        // Cinese
        ("zh", "TMC • Optimization completed") => "TMC • 优化完成",
        ("zh", "TMC • Scheduled optimization") => "TMC • 计划优化",
        ("zh", "TMC • Low memory optimization") => "TMC • 低内存优化",
        ("zh", "Normal") => "普通",
        ("zh", "Balanced") => "平衡",
        ("zh", "Gaming") => "游戏",
        
        // Default inglese
        (_, "TMC • Optimization completed") => "TMC • Optimization completed",
        (_, "TMC • Scheduled optimization") => "TMC • Scheduled optimization",
        (_, "TMC • Low memory optimization") => "TMC • Low memory optimization",
        (_, "Normal") => "Normal",
        (_, "Balanced") => "Balanced",
        (_, "Gaming") => "Gaming",
        _ => key,
    }.to_string()
}

