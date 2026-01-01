use crate::config::Profile;
use crate::memory::types::Reason;

/// Get notification title based on language and reason
pub fn get_notification_title(language: &str, reason: Reason) -> String {
    match reason {
        Reason::Manual => t(language, "TMC • Optimization completed"),
        Reason::Schedule => t(language, "TMC • Scheduled optimization"),
        Reason::LowMemory => t(language, "TMC • Low memory optimization"),
        Reason::Hotkey => t(language, "TMC • Hotkey optimization"),
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

