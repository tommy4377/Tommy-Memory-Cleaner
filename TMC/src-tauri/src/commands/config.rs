/// Configuration management commands.
///
/// This module provides Tauri commands for managing application configuration,
/// including loading, saving, and updating various settings such as profiles,
/// memory areas, themes, and system preferences.
use crate::config::{Config, Priority, Profile};
use crate::memory::types::Areas;
use tauri::{AppHandle, Emitter, Manager, State};

/// Exits the application gracefully.
///
/// This command terminates the application process after logging the exit event.
#[tauri::command]
pub fn cmd_exit(_app: AppHandle) {
    tracing::info!("Exiting application...");
    std::process::exit(0);
}

/// Retrieves the current application configuration.
///
/// # Returns
///
/// Returns a clone of the current configuration or an error if the
/// configuration lock is poisoned.
#[tauri::command]
pub fn cmd_get_config(state: State<'_, crate::AppState>) -> Result<Config, String> {
    state
        .cfg
        .lock()
        .map_err(|_| "Config lock poisoned".to_string())
        .map(|c| c.clone())
}

/// Saves configuration changes from JSON data.
///
/// This command updates the application configuration based on the provided
/// JSON value. It includes rate limiting to prevent excessive updates.
///
/// # Arguments
///
/// * `app` - The application handle for emitting events
/// * `state` - The application state containing the configuration
/// * `cfg_json` - JSON value containing the configuration changes
///
/// # Returns
///
/// Returns `Ok(())` if the configuration is saved successfully,
/// or an error string if the operation fails.
#[tauri::command]
pub fn cmd_save_config(
    app: AppHandle,
    state: State<'_, crate::AppState>,
    cfg_json: serde_json::Value,
) -> Result<(), String> {
    // Rate limiting check to prevent excessive configuration updates
    {
        let mut rl = state
            .rate_limiter
            .lock()
            .map_err(|_| "Rate limiter lock poisoned".to_string())?;
        if !rl.check_rate_limit("save_config") {
            return Err("Too many requests. Please wait before trying again.".to_string());
        }
    }

    let mut current_cfg = state
        .cfg
        .lock()
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
                need_icon_update = true; // Tray icon changes color based on theme
            }
        }

        // Main color - support for separate light/dark colors
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
        update_bool!(request_elevation_on_startup);
        // Setup completed - important to prevent setup from opening multiple times
        if let Some(v) = obj.get("setup_completed") {
            if let Some(b) = v.as_bool() {
                current_cfg.setup_completed = b;
            }
        }
        
        // Platform detection fields
        if let Some(v) = obj.get("platform_detected") {
            if let Some(b) = v.as_bool() {
                current_cfg.platform_detected = b;
            }
        }
        if let Some(v) = obj.get("is_windows_10") {
            if let Some(b) = v.as_bool() {
                current_cfg.is_windows_10 = b;
            }
        }
        // Handle run_on_startup specially - it needs to call the system function
        if let Some(v) = obj.get("run_on_startup") {
            if let Some(b) = v.as_bool() {
                // Execute operation and log any errors
                if let Err(e) = crate::system::startup::set_run_on_startup(b) {
                    tracing::error!("Error enabling automatic startup (settings): {:?}", e);
                }
                // Force the boolean value chosen by user in config,
                // instead of re-reading from system which might be slow to update
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
            if let Ok(list) =
                serde_json::from_value::<std::collections::BTreeSet<String>>(v.clone())
            {
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

    // FIX #2: Release lock as soon as possible - save config with retry then release
    {
        let mut guard = state
            .cfg
            .lock()
            .map_err(|_| "Config lock poisoned".to_string())?;
        *guard = current_cfg.clone();

        // Save with retry for better reliability
        let save_result = guard.save();
        match save_result {
            Ok(_) => {
                tracing::debug!("Config saved successfully");
            }
            Err(e) => {
                tracing::warn!("Failed to save config: {:?}, retrying...", e);
                // Retry once after a short delay
                std::thread::sleep(std::time::Duration::from_millis(100));
                guard.save().map_err(|e2| {
                    tracing::error!("Failed to save config on retry: {:?}", e2);
                    format!("Failed to save config: {}", e2)
                })?;
            }
        }
        // Lock is automatically released here
    }

    // Update UI - all these operations happen AFTER the lock has been released
    // Note: update_menu no longer exists, menu is managed via HTML

    if need_icon_update {
        crate::refresh_tray_icon(&app);
    }

    if need_hotkey_update {
        if let Err(e) =
            crate::register_global_hotkey_v2(&app, &current_cfg.hotkey, state.inner().cfg.clone())
        {
            tracing::error!("Failed to register hotkey: {}", e);
        }
    }

    // Emit config-changed event for tray menu
    let _ = app.emit("config-changed", ());

    Ok(())
}

/// Completes the setup wizard with provided configuration.
///
/// This command applies the initial configuration settings chosen during
/// the first-time setup, including startup preferences, theme, language,
/// and window behavior. It also handles the transition from setup window
/// to the main application window.
///
/// # Arguments
///
/// * `app` - The application handle for window management
/// * `state` - The application state containing the configuration
/// * `setup_data` - JSON value containing the setup configuration
///
/// # Returns
///
/// Returns `Ok(())` if setup is completed successfully,
/// or an error string if the operation fails.
#[tauri::command]
pub fn cmd_complete_setup(
    app: AppHandle,
    state: State<'_, crate::AppState>,
    setup_data: serde_json::Value,
) -> Result<(), String> {
    let mut cfg = state
        .cfg
        .lock()
        .map_err(|_| "Config lock poisoned".to_string())?;

    // Apply settings from setup
    if let Some(obj) = setup_data.as_object() {
        // Handle platform detection
        if let Some(v) = obj.get("platform_detected") {
            if let Some(b) = v.as_bool() {
                cfg.platform_detected = b;
            }
        }
        if let Some(v) = obj.get("is_windows_10") {
            if let Some(b) = v.as_bool() {
                cfg.is_windows_10 = b;
            }
        }
        
        if let Some(v) = obj.get("run_on_startup") {
            if let Some(b) = v.as_bool() {
                // Execute operation and log any errors
                if let Err(e) = crate::system::startup::set_run_on_startup(b) {
                    tracing::error!("Failed to set startup during setup: {:?}", e);
                }
                // Force the boolean value chosen by user in config,
                // instead of re-reading from system which might be slow to update
                cfg.run_on_startup = b;
            }
        }

        if let Some(v) = obj.get("theme") {
            if let Some(s) = v.as_str() {
                cfg.theme = s.to_string();

                // If theme is light and no main color for light is set, set default
                if s == "light" && cfg.main_color_hex_light.is_empty() {
                    cfg.main_color_hex_light = "#9a8a72".to_string();
                }
                // If theme is dark and no main color for dark is set, set default
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

    // Mark setup as completed and save with retry
    cfg.setup_completed = true;

    // Save with retry to ensure setup_completed is saved
    let save_result = cfg.save();
    match save_result {
        Ok(_) => {
            tracing::info!("Config saved successfully after setup completion");
        }
        Err(e) => {
            tracing::error!("Failed to save config after setup: {:?}", e);
            // Retry once after a short delay
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

    // Verify that setup_completed was saved correctly
    let config_path = crate::config::get_portable_detector().config_path();
    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(setup_completed) = json.get("setup_completed").and_then(|v| v.as_bool())
                {
                    if !setup_completed {
                        tracing::warn!("setup_completed not saved correctly, forcing save again");
                        cfg.setup_completed = true;
                        let _ = cfg.save();
                    }
                }
            }
        }
    }

    // Log applied settings for debugging
    tracing::info!("Setup completed - Theme: {}, Language: {}, AlwaysOnTop: {}, ShowNotifications: {}, RunOnStartup: {}, SetupCompleted: {}", 
        cfg.theme, cfg.language, cfg.always_on_top, cfg.show_opt_notifications, cfg.run_on_startup, cfg.setup_completed);

    // Prepare data for synchronization BEFORE creating/showing the window
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

    // Show the main window FIRST, THEN close setup
    // Ensure the main window exists, otherwise create it
    let main_window = if let Some(window) = app.get_webview_window("main") {
        tracing::info!("Main window already exists, showing it...");
        Some(window)
    } else {
        tracing::info!("Main window not found, creating it...");
        // Create the main window if it doesn't exist
        match tauri::WebviewWindowBuilder::new(
            &app,
            "main",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("Tommy Memory Cleaner")
        .inner_size(500.0, 700.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)  // Disabilita shadow per Windows 10
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

    // Show the main window and apply settings
    let main_window_shown = if let Some(main_window) = main_window {
        tracing::info!("Showing main window after setup...");

        // Apply always_on_top (both true and false) - fallback if main window doesn't respond
        let _ = crate::system::window::set_always_on_top(&app, always_on_top);

        // Ensure the window is visible and not hidden
        // Correct order according to best practices: skip_taskbar -> unminimize -> show -> center -> focus
        let _ = main_window.set_skip_taskbar(false);

        // Unminimize before show (if minimized)
        let _ = main_window.unminimize();

        // Show the window
        let show_result = main_window.show();
        if let Err(e) = show_result {
            tracing::error!("Failed to show main window: {:?}", e);
            false
        } else {
            // Center the window
            let _ = main_window.center();

            // Focus the window (after show and center)
            if let Err(e) = main_window.set_focus() {
                tracing::warn!("Failed to focus main window: {:?}", e);
            }

            // Apply rounded corners using centralized function
            #[cfg(windows)]
            {
                let _ = crate::system::window::apply_window_decorations(&main_window);
                // Re-center window after applying rounded corners
                let _ = main_window.center();
            }

            // Apply always_on_top directly to the main window as well
            if let Err(e) = main_window.set_always_on_top(always_on_top) {
                tracing::warn!("Failed to set always_on_top on main window: {:?}", e);
            }

            // Emit event to apply theme and color in the main window
            // The frontend will listen for this event and apply the theme and correct color
            let _ = main_window.eval(&format!(
                r#"
                (function() {{
                    // Apply the theme
                    document.documentElement.setAttribute('data-theme', '{}');
                    localStorage.setItem('tmc_theme', '{}');
                    
                    // Apply the correct main color for the theme
                    const root = document.documentElement;
                    root.style.setProperty('--btn-bg', '{}');
                    root.style.setProperty('--bar-fill', '{}');
                    root.style.setProperty('--input-focus', '{}');
                    
                    // Apply the language if available
                    if (typeof window.setLanguage === 'function') {{
                        window.setLanguage('{}');
                    }}
                    
                    // Notify frontend to reload config
                    if (typeof window.dispatchEvent !== 'undefined') {{
                        window.dispatchEvent(new CustomEvent('config-updated'));
                    }}
                }})();
                "#,
                theme, theme, main_color, main_color, main_color, language
            ));

            // Small delay to ensure main window is fully loaded
            std::thread::sleep(std::time::Duration::from_millis(200));
            true
        }
    } else {
        tracing::error!("Failed to get or create main window");
        false
    };

    // Emit event to notify frontend that setup is completed
    // Frontend will close setup after verifying main window is ready
    tracing::info!(
        "Setup completed, emitting setup-complete event (main window shown: {})...",
        main_window_shown
    );
    let _ = app.emit("setup-complete", ());

    // Emit config-changed event since setup modifies configuration
    let _ = app.emit("config-changed", ());

    // DO NOT close setup here - let frontend close it after verifying
    // that main window is ready. This avoids race conditions and crashes.

    Ok(())
}
