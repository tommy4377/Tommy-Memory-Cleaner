use crate::config::{Config, Priority, Profile};
use crate::memory::types::Areas;
use tauri::{AppHandle, State, Manager, Emitter};

/// Exit the application
#[tauri::command]
pub fn cmd_exit(_app: AppHandle) {
    tracing::info!("Exiting application...");
    std::process::exit(0);
}

/// Get current configuration
#[tauri::command]
pub fn cmd_get_config(state: State<'_, crate::AppState>) -> Result<Config, String> {
    state.cfg.lock()
        .map_err(|_| "Config lock poisoned".to_string())
        .map(|c| c.clone())
}

/// Save configuration from JSON
#[tauri::command]
pub fn cmd_save_config(app: AppHandle, state: State<'_, crate::AppState>, cfg_json: serde_json::Value) -> Result<(), String> {
    // Rate limiting check
    {
        let mut rl = state.rate_limiter.lock()
            .map_err(|_| "Rate limiter lock poisoned".to_string())?;
        if !rl.check_rate_limit("save_config") {
            return Err("Too many requests. Please wait before trying again.".to_string());
        }
    }
    
    let mut current_cfg = state.cfg.lock()
        .map_err(|_| "Config lock poisoned".to_string())?
        .clone();
    
    let mut _need_menu_update = false;
    let mut need_icon_update = false;
    let mut need_hotkey_update = false;
    
    if let Some(obj) = cfg_json.as_object() {
        // Profile handling
        if let Some(v) = obj.get("profile") {
            if let Ok(profile) = serde_json::from_value::<Profile>(v.clone()) {
                current_cfg.profile = profile.clone();
                current_cfg.memory_areas = profile.get_memory_areas();
                current_cfg.run_priority = profile.get_priority();
                need_icon_update = true;
            }
        }
        
        // Memory areas
        if let Some(v) = obj.get("memory_areas") {
            if let Some(areas_num) = v.as_u64() {
                current_cfg.memory_areas = Areas::from_bits_truncate(areas_num as u32);
            } else if let Some(areas_str) = v.as_str() {
                current_cfg.memory_areas = crate::parse_areas_string(areas_str);
            }
        }
        
        // Hotkey
        if let Some(v) = obj.get("hotkey") {
            if let Some(s) = v.as_str() {
                current_cfg.hotkey = s.to_string();
                need_hotkey_update = true;
            }
        }
        
        // Language
        if let Some(v) = obj.get("language") {
            if let Some(s) = v.as_str() {
                let old_language = current_cfg.language.clone();
                current_cfg.language = s.to_string();
                _need_menu_update = true;
                
                // Emit event if language actually changed
                if old_language != s.to_string() {
                    let _ = app.emit("language-changed", s.to_string());
                }
            }
        }
        
        // Theme
        if let Some(v) = obj.get("theme") {
            if let Some(s) = v.as_str() {
                current_cfg.theme = s.to_string();
                need_icon_update = true; // Tray icon cambia colore in base al tema
            }
        }
        
        // Main color - supporto per light/dark separati
        if let Some(v) = obj.get("main_color_hex_light") {
            if let Some(s) = v.as_str() {
                current_cfg.main_color_hex_light = s.to_string();
            }
        }
        
        if let Some(v) = obj.get("main_color_hex_dark") {
            if let Some(s) = v.as_str() {
                current_cfg.main_color_hex_dark = s.to_string();
            }
        }
        
        // Backward compatibility
        if let Some(v) = obj.get("main_color_hex") {
            if let Some(s) = v.as_str() {
                current_cfg.main_color_hex = s.to_string();
            }
        }
        
        // Tray
        if let Some(v) = obj.get("tray") {
            if let Ok(tray) = serde_json::from_value::<crate::config::TrayConfig>(v.clone()) {
                current_cfg.tray = tray;
                need_icon_update = true;
            }
        }
        
        // Boolean fields
        macro_rules! update_bool {
            ($field:ident) => {
                if let Some(v) = obj.get(stringify!($field)) {
                    if let Some(b) = v.as_bool() {
                        current_cfg.$field = b;
                    }
                }
            };
        }
        
        update_bool!(always_on_top);
        update_bool!(minimize_to_tray);
        update_bool!(show_opt_notifications);
        update_bool!(auto_update);
        update_bool!(close_after_opt);
        // ⭐ Setup completed - importante per evitare che il setup si apra più volte
        if let Some(v) = obj.get("setup_completed") {
            if let Some(b) = v.as_bool() {
                current_cfg.setup_completed = b;
            }
        }
        // Handle run_on_startup specially - it needs to call the system function
        if let Some(v) = obj.get("run_on_startup") {
            if let Some(b) = v.as_bool() {
                // Esegui l'operazione e logga eventuali errori
                if let Err(e) = crate::system::startup::set_run_on_startup(b) {
                    tracing::error!("Errore attivazione avvio automatico (settings): {:?}", e);
                }
                // Forziamo il valore booleano scelto dall'utente nel config,
                // invece di ri-leggerlo dal sistema che potrebbe essere lento ad aggiornarsi
                current_cfg.run_on_startup = b;
            }
        }
        update_bool!(compact_mode);
        
        // Numeric fields
        if let Some(v) = obj.get("auto_opt_interval_hours") {
            if let Some(n) = v.as_u64() {
                if n == 0 {
                    tracing::warn!("auto_opt_interval_hours cannot be 0, using default value 1");
                    current_cfg.auto_opt_interval_hours = 1;
                } else {
                    current_cfg.auto_opt_interval_hours = n.min(24) as u32;
                }
            }
        }
        
        if let Some(v) = obj.get("auto_opt_free_threshold") {
            if let Some(n) = v.as_u64() {
                if n == 0 {
                    tracing::warn!("auto_opt_free_threshold cannot be 0, using default value 1");
                    current_cfg.auto_opt_free_threshold = 1;
                } else {
                    current_cfg.auto_opt_free_threshold = n.min(100) as u8;
                }
            }
        }
        
        if let Some(v) = obj.get("font_size") {
            if let Some(n) = v.as_f64() {
                current_cfg.font_size = (n as f32).clamp(8.0, 24.0);
            }
        }
        
        // Process exclusions
        if let Some(v) = obj.get("process_exclusion_list") {
            if let Ok(list) = serde_json::from_value::<std::collections::BTreeSet<String>>(v.clone()) {
                current_cfg.process_exclusion_list = list;
            }
        }
        
        // Priority
        if let Some(v) = obj.get("run_priority") {
            if let Ok(priority) = serde_json::from_value::<Priority>(v.clone()) {
                current_cfg.run_priority = priority;
            }
        }
    }
    
    // Validate and save
    current_cfg.validate();
    
    // ⭐ FIX #2: Rilascia il lock il prima possibile - salva la config con retry e poi rilascia
    {
        let mut guard = state.cfg.lock()
            .map_err(|_| "Config lock poisoned".to_string())?;
        *guard = current_cfg.clone();
        
        // ⭐ Salvataggio con retry per maggiore affidabilità
        let save_result = guard.save();
        match save_result {
            Ok(_) => {
                tracing::debug!("Config saved successfully");
            }
            Err(e) => {
                tracing::warn!("Failed to save config: {:?}, retrying...", e);
                // Retry una volta dopo un breve delay
                std::thread::sleep(std::time::Duration::from_millis(100));
                guard.save().map_err(|e2| {
                    tracing::error!("Failed to save config on retry: {:?}", e2);
                    format!("Failed to save config: {}", e2)
                })?;
            }
        }
        // Lock viene rilasciato qui automaticamente
    }
    
    // Update UI - tutte queste operazioni avvengono DOPO che il lock è stato rilasciato
    // Nota: update_menu non esiste più, il menu è gestito via HTML
    
    if need_icon_update {
        crate::refresh_tray_icon(&app);
    }
    
    if need_hotkey_update {
        if let Err(e) = crate::register_global_hotkey_v2(&app, &current_cfg.hotkey, state.inner().cfg.clone()) {
            tracing::error!("Failed to register hotkey: {}", e);
        }
    }
    
    Ok(())
}

/// Complete setup wizard
#[tauri::command]
pub fn cmd_complete_setup(
    app: AppHandle,
    state: State<'_, crate::AppState>,
    setup_data: serde_json::Value,
) -> Result<(), String> {
    let mut cfg = state.cfg.lock()
        .map_err(|_| "Config lock poisoned".to_string())?;
    
    // Applica le impostazioni dal setup
    if let Some(obj) = setup_data.as_object() {
        if let Some(v) = obj.get("run_on_startup") {
            if let Some(b) = v.as_bool() {
                // Esegui l'operazione e logga eventuali errori
                if let Err(e) = crate::system::startup::set_run_on_startup(b) {
                    tracing::error!("Failed to set startup during setup: {:?}", e);
                }
                // Forziamo il valore booleano scelto dall'utente nel config,
                // invece di ri-leggerlo dal sistema che potrebbe essere lento ad aggiornarsi
                cfg.run_on_startup = b;
            }
        }
        
        if let Some(v) = obj.get("theme") {
            if let Some(s) = v.as_str() {
                cfg.theme = s.to_string();
                
                // Se il tema è light e non c'è un colore principale per light, imposta il default
                if s == "light" && cfg.main_color_hex_light.is_empty() {
                    cfg.main_color_hex_light = "#9a8a72".to_string();
                }
                // Se il tema è dark e non c'è un colore principale per dark, imposta il default
                if s == "dark" && cfg.main_color_hex_dark.is_empty() {
                    cfg.main_color_hex_dark = "#0a84ff".to_string();
                }
            }
        }
        
        if let Some(v) = obj.get("always_on_top") {
            if let Some(b) = v.as_bool() {
                cfg.always_on_top = b;
                let _ = crate::system::window::set_always_on_top(&app, b);
            }
        }
        
        if let Some(v) = obj.get("show_opt_notifications") {
            if let Some(b) = v.as_bool() {
                cfg.show_opt_notifications = b;
            }
        }
        
        if let Some(v) = obj.get("language") {
            if let Some(s) = v.as_str() {
                cfg.language = s.to_string();
            }
        }
    }
    
    // ⭐ Segna il setup come completato e salva con retry
    cfg.setup_completed = true;
    
    // ⭐ Salvataggio con retry per assicurarsi che setup_completed venga salvato
    let save_result = cfg.save();
    match save_result {
        Ok(_) => {
            tracing::info!("Config saved successfully after setup completion");
        }
        Err(e) => {
            tracing::error!("Failed to save config after setup: {:?}", e);
            // ⭐ Retry una volta dopo un breve delay
            std::thread::sleep(std::time::Duration::from_millis(200));
            match cfg.save() {
                Ok(_) => {
                    tracing::info!("Config saved successfully on retry");
                }
                Err(e2) => {
                    tracing::error!("Failed to save config on retry: {:?}", e2);
                    return Err(format!("Failed to save config: {}", e2));
                }
            }
        }
    }
    
    // ⭐ Verifica che setup_completed sia stato salvato correttamente
    let config_path = crate::config::get_portable_detector().config_path();
    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(setup_completed) = json.get("setup_completed").and_then(|v| v.as_bool()) {
                    if !setup_completed {
                        tracing::warn!("setup_completed not saved correctly, forcing save again");
                        cfg.setup_completed = true;
                        let _ = cfg.save();
                    }
                }
            }
        }
    }
    
    // Log delle impostazioni applicate per debug
    tracing::info!("Setup completed - Theme: {}, Language: {}, AlwaysOnTop: {}, ShowNotifications: {}, RunOnStartup: {}, SetupCompleted: {}", 
        cfg.theme, cfg.language, cfg.always_on_top, cfg.show_opt_notifications, cfg.run_on_startup, cfg.setup_completed);
    
    // Prepara i dati per la sincronizzazione PRIMA di creare/mostrare la finestra
    let theme = cfg.theme.clone();
    let main_color_light = cfg.main_color_hex_light.clone();
    let main_color_dark = cfg.main_color_hex_dark.clone();
    let main_color = if theme == "light" {
        if !main_color_light.is_empty() {
            main_color_light
        } else {
            "#9a8a72".to_string()
        }
    } else {
        if !main_color_dark.is_empty() {
            main_color_dark
        } else {
            "#0a84ff".to_string()
        }
    };
    let language = cfg.language.clone();
    let always_on_top = cfg.always_on_top;
    
    // Mostra PRIMA la finestra principale, POI chiudi il setup
    // Assicurati che la finestra principale esista, altrimenti creala
    let main_window = if let Some(window) = app.get_webview_window("main") {
        tracing::info!("Main window already exists, showing it...");
        Some(window)
    } else {
        tracing::info!("Main window not found, creating it...");
        // Crea la finestra principale se non esiste
        match tauri::WebviewWindowBuilder::new(
            &app,
            "main",
            tauri::WebviewUrl::App("index.html".into())
        )
        .title("Tommy Memory Cleaner")
        .inner_size(480.0, 680.0)
        .resizable(false)
        .center()
        .skip_taskbar(false)
        .visible(true)
        .build()
        {
            Ok(window) => {
                tracing::info!("Main window created successfully after setup");
                Some(window)
            }
            Err(e) => {
                tracing::error!("Failed to create main window: {:?}", e);
                None
            }
        }
    };
    
    // Mostra la finestra principale e applica le impostazioni
    let main_window_shown = if let Some(main_window) = main_window {
        tracing::info!("Showing main window after setup...");
        
        // Applica always_on_top (sia true che false) - fallback se la finestra principale non risponde
        let _ = crate::system::window::set_always_on_top(&app, always_on_top);
        
        // Assicurati che la finestra sia visibile e non nascosta
        // Ordine corretto secondo best practices: skip_taskbar -> unminimize -> show -> center -> focus
        let _ = main_window.set_skip_taskbar(false);
        
        // Unminimize prima di show (se minimizzata)
        let _ = main_window.unminimize();
        
        // Mostra la finestra
        let show_result = main_window.show();
        if let Err(e) = show_result {
            tracing::error!("Failed to show main window: {:?}", e);
            false
        } else {
            // Centra la finestra
            let _ = main_window.center();
            
            // Focalizza la finestra (dopo show e center)
            if let Err(e) = main_window.set_focus() {
                tracing::warn!("Failed to focus main window: {:?}", e);
            }
            
            // Applica always_on_top anche alla finestra principale direttamente
            if let Err(e) = main_window.set_always_on_top(always_on_top) {
                tracing::warn!("Failed to set always_on_top on main window: {:?}", e);
            }
            
            // Emetti evento per applicare il tema e il colore nella finestra principale
            // Il frontend ascolterà questo evento e applicherà il tema e il colore corretto
            let _ = main_window.eval(&format!(
                r#"
                (function() {{
                    // Applica il tema
                    document.documentElement.setAttribute('data-theme', '{}');
                    localStorage.setItem('tmc_theme', '{}');
                    
                    // Applica il colore principale corretto per il tema
                    const root = document.documentElement;
                    root.style.setProperty('--btn-bg', '{}');
                    root.style.setProperty('--bar-fill', '{}');
                    root.style.setProperty('--input-focus', '{}');
                    
                    // Applica la lingua se disponibile
                    if (typeof window.setLanguage === 'function') {{
                        window.setLanguage('{}');
                    }}
                    
                    // Notifica il frontend di ricaricare la config
                    if (typeof window.dispatchEvent !== 'undefined') {{
                        window.dispatchEvent(new CustomEvent('config-updated'));
                    }}
                }})();
                "#,
                theme, theme, main_color, main_color, main_color, language
            ));
            
            // Piccolo delay per assicurarsi che la finestra principale sia completamente caricata
            std::thread::sleep(std::time::Duration::from_millis(200));
            true
        }
    } else {
        tracing::error!("Failed to get or create main window");
        false
    };
    
    // Emetti evento per notificare il frontend che il setup è completato
    // Il frontend chiuderà il setup dopo aver verificato che la finestra principale è pronta
    tracing::info!("Setup completed, emitting setup-complete event (main window shown: {})...", main_window_shown);
    let _ = app.emit("setup-complete", ());
    
    // NON chiudere il setup qui - lascia che il frontend lo chiuda dopo aver verificato
    // che la finestra principale è pronta. Questo evita race conditions e crash.
    
    Ok(())
}

