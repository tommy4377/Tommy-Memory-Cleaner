// Translation function - NOTE: This is a temporary solution
// TODO: Remove this module entirely and use frontend i18n instead (see IDEE_MIGLIORAMENTI.md 1.2)

pub fn t(lang: &str, key: &str) -> String {
    match (lang, key) {
        // Italiano
        ("it", "Open TMC") => "Apri TMC",
        ("it", "Optimize Memory") => "Ottimizza Memoria",
        ("it", "Exit") => "Esci",
        ("it", "TMC • Optimization completed") => "TMC • Ottimizzazione completata",
        ("it", "TMC • Scheduled optimization") => "TMC • Ottimizzazione programmata",
        ("it", "TMC • Low memory optimization") => "TMC • Ottimizzazione per memoria bassa",
        ("it", "Normal") => "Normale",
        ("it", "Balanced") => "Bilanciato",
        ("it", "Gaming") => "Gaming",
        
        // Spagnolo
        ("es", "Open TMC") => "Abrir TMC",
        ("es", "Optimize Memory") => "Optimizar Memoria",
        ("es", "Exit") => "Salir",
        ("es", "TMC • Optimization completed") => "TMC • Optimización completada",
        ("es", "TMC • Scheduled optimization") => "TMC • Optimización programada",
        ("es", "TMC • Low memory optimization") => "TMC • Optimización por memoria baja",
        ("es", "Normal") => "Normal",
        ("es", "Balanced") => "Equilibrado",
        ("es", "Gaming") => "Gaming",
        
        // Francese
        ("fr", "Open TMC") => "Ouvrir TMC",
        ("fr", "Optimize Memory") => "Optimiser la Mémoire",
        ("fr", "Exit") => "Quitter",
        ("fr", "TMC • Optimization completed") => "TMC • Optimisation terminée",
        ("fr", "TMC • Scheduled optimization") => "TMC • Optimisation programmée",
        ("fr", "TMC • Low memory optimization") => "TMC • Optimisation mémoire faible",
        ("fr", "Normal") => "Normal",
        ("fr", "Balanced") => "Équilibré",
        ("fr", "Gaming") => "Gaming",
        
        // Portoghese
        ("pt", "Open TMC") => "Abrir TMC",
        ("pt", "Optimize Memory") => "Otimizar Memória",
        ("pt", "Exit") => "Sair",
        ("pt", "TMC • Optimization completed") => "TMC • Otimização concluída",
        ("pt", "TMC • Scheduled optimization") => "TMC • Otimização agendada",
        ("pt", "TMC • Low memory optimization") => "TMC • Otimização por memória baixa",
        ("pt", "Normal") => "Normal",
        ("pt", "Balanced") => "Balanceado",
        ("pt", "Gaming") => "Jogos",
        
        // Tedesco
        ("de", "Open TMC") => "TMC Öffnen",
        ("de", "Optimize Memory") => "Speicher Optimieren",
        ("de", "Exit") => "Beenden",
        ("de", "TMC • Optimization completed") => "TMC • Optimierung abgeschlossen",
        ("de", "TMC • Scheduled optimization") => "TMC • Geplante Optimierung",
        ("de", "TMC • Low memory optimization") => "TMC • Optimierung bei wenig Speicher",
        ("de", "Normal") => "Normal",
        ("de", "Balanced") => "Ausgeglichen",
        ("de", "Gaming") => "Spielen",
        
        // Arabo
        ("ar", "Open TMC") => "فتح TMC",
        ("ar", "Optimize Memory") => "تحسين الذاكرة",
        ("ar", "Exit") => "خروج",
        ("ar", "TMC • Optimization completed") => "TMC • اكتمل التحسين",
        ("ar", "TMC • Scheduled optimization") => "TMC • تحسين مجدول",
        ("ar", "TMC • Low memory optimization") => "TMC • تحسين الذاكرة المنخفضة",
        ("ar", "Normal") => "عادي",
        ("ar", "Balanced") => "متوازن",
        ("ar", "Gaming") => "الألعاب",
        
        // Giapponese
        ("ja", "Open TMC") => "TMCを開く",
        ("ja", "Optimize Memory") => "メモリを最適化",
        ("ja", "Exit") => "終了",
        ("ja", "TMC • Optimization completed") => "TMC • 最適化完了",
        ("ja", "TMC • Scheduled optimization") => "TMC • スケジュール最適化",
        ("ja", "TMC • Low memory optimization") => "TMC • メモリ不足最適化",
        ("ja", "Normal") => "ノーマル",
        ("ja", "Balanced") => "バランス",
        ("ja", "Gaming") => "ゲーミング",
        
        // Cinese
        ("zh", "Open TMC") => "打开TMC",
        ("zh", "Optimize Memory") => "优化内存",
        ("zh", "Exit") => "退出",
        ("zh", "TMC • Optimization completed") => "TMC • 优化完成",
        ("zh", "TMC • Scheduled optimization") => "TMC • 计划优化",
        ("zh", "TMC • Low memory optimization") => "TMC • 低内存优化",
        ("zh", "Normal") => "普通",
        ("zh", "Balanced") => "平衡",
        ("zh", "Gaming") => "游戏",
        
        // Default inglese
        (_, "Open TMC") => "Open TMC",
        (_, "Optimize Memory") => "Optimize Memory",
        (_, "Exit") => "Exit",
        (_, "TMC • Optimization completed") => "TMC • Optimization completed",
        (_, "TMC • Scheduled optimization") => "TMC • Scheduled optimization",
        (_, "TMC • Low memory optimization") => "TMC • Low memory optimization",
        (_, "Normal") => "Normal",
        (_, "Balanced") => "Balanced",
        (_, "Gaming") => "Gaming",
        _ => key,
    }.to_string()
}

