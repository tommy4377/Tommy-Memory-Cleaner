use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn cmd_show_or_create_window(app: AppHandle) {
    crate::show_or_create_window(&app);
}

#[tauri::command]
pub fn cmd_show_notification(
    app: AppHandle,
    title: String,
    message: String,
    state: State<'_, crate::AppState>,
) -> Result<(), String> {
    // Ottieni il tema corrente dalla configurazione
    let theme = {
        match state.cfg.try_lock() {
            Ok(cfg_guard) => cfg_guard.theme.clone(),
            Err(_) => {
                tracing::debug!("Config lock busy in cmd_show_notification, using default theme");
                "dark".to_string()
            }
        }
    };
    // Usa la funzione del modulo notifications
    crate::notifications::show_windows_notification(&app, &title, &message, &theme)
}

// Helper functions that need to be accessible from main.rs
pub fn show_or_create_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _: Result<(), _> = window.set_skip_taskbar(false); // Mostra nella taskbar
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = window.center();
    } else {
        tracing::info!("Creating new main window...");
        let result = tauri::WebviewWindowBuilder::new(
            app,
            "main",
            tauri::WebviewUrl::App("index.html".into())
        )
        .title("Tommy Memory Cleaner")
        .inner_size(480.0, 680.0)
        .resizable(false)
        .shadow(true)  // Abilita shadow per bordi arrotondati
        .center()
        .skip_taskbar(false)  // Mostra nella taskbar
        .visible(true)  // Assicurati che sia visibile
        .build();

        match result {
            Ok(window) => {
                tracing::info!("Window created successfully");
                let _ = window.set_skip_taskbar(false);
                if let Err(e) = window.show() {
                    tracing::error!("Failed to show newly created window: {:?}", e);
                }
                let _ = window.set_focus();
            }
            Err(e) => {
                tracing::error!("Failed to create window: {:?}", e);
                eprintln!("FATAL ERROR: Failed to create window: {:?}", e);
            }
        }
    }
}

pub fn position_tray_menu(window: &tauri::WebviewWindow) {
    // Ottieni le dimensioni del menu
    let menu_size = match window.outer_size() {
        Ok(size) => size,
        Err(e) => {
            tracing::error!("Failed to get menu size: {:?}", e);
            return;
        }
    };

    let menu_width = menu_size.width as i32;
    let menu_height = menu_size.height as i32;

    // ⭐ FIX: Ottieni PRIMA la posizione del cursore (vicino alla tray icon)
    let cursor_pos = match window.cursor_position() {
        Ok(pos) => pos,
        Err(_) => {
            tracing::error!("Failed to get cursor position");
            // Fallback: usa il monitor primario
            if let Ok(Some(monitor)) = window.primary_monitor() {
                let monitor_size = monitor.size();
                let monitor_pos = monitor.position();
                let fallback_pos = tauri::PhysicalPosition {
                    x: (monitor_pos.x + monitor_size.width as i32 - 50) as f64,
                    y: (monitor_pos.y + monitor_size.height as i32 - 50) as f64,
                };
                tracing::warn!("Using fallback cursor position: {:?}", fallback_pos);
                fallback_pos
            } else {
                tracing::error!("Failed to get primary monitor for fallback");
                return;
            }
        }
    };

    // ⭐ FIX: Trova il monitor che contiene il cursore (non quello della finestra)
    let cursor_x = cursor_pos.x as i32;
    let cursor_y = cursor_pos.y as i32;

    // Ottieni tutti i monitor disponibili e trova quello che contiene il cursore
    let monitor = match window.available_monitors() {
        Ok(monitors) => {
            // Trova il monitor che contiene il cursore
            let mut found_monitor = None;
            for m in monitors {
                let m_pos = m.position();
                let m_size = m.size();

                let m_left = m_pos.x;
                let m_top = m_pos.y;
                let m_right = m_pos.x + m_size.width as i32;
                let m_bottom = m_pos.y + m_size.height as i32;

                // Verifica se il cursore è dentro questo monitor
                if cursor_x >= m_left
                    && cursor_x < m_right
                    && cursor_y >= m_top
                    && cursor_y < m_bottom
                {
                    // Log prima di spostare m
                    tracing::debug!(
                        "Found monitor containing cursor: {}x{} at {:?}",
                        m_size.width,
                        m_size.height,
                        m_pos
                    );
                    found_monitor = Some(m);
                    break;
                }
            }

            // Se non trovato, usa il monitor primario come fallback
            found_monitor.unwrap_or_else(|| {
                tracing::warn!("Cursor not found in any monitor, using primary monitor");
                window
                    .primary_monitor()
                    .ok()
                    .flatten()
                    .expect("No primary monitor available")
            })
        }
        Err(e) => {
            tracing::error!("Failed to get available monitors: {:?}", e);
            // Fallback: usa current_monitor o primary_monitor direttamente
            match window
                .current_monitor()
                .ok()
                .flatten()
                .or_else(|| window.primary_monitor().ok().flatten())
            {
                Some(m) => {
                    tracing::warn!("Using fallback monitor (current or primary)");
                    m
                }
                None => {
                    tracing::error!("No monitors available");
                    return;
                }
            }
        }
    };

    let monitor_size = monitor.size();
    let monitor_pos = monitor.position();

    tracing::debug!(
        "Cursor position: {:?}, Using monitor: {}x{} at {:?}",
        cursor_pos,
        monitor_size.width,
        monitor_size.height,
        monitor_pos
    );

    // Determina la posizione della taskbar
    let (final_x, final_y) = if let Some((
        taskbar_left,
        taskbar_top,
        taskbar_right,
        taskbar_bottom,
    )) = get_taskbar_rect()
    {
        let taskbar_height = taskbar_bottom - taskbar_top;
        let taskbar_width = taskbar_right - taskbar_left;
        let is_taskbar_vertical = taskbar_width < taskbar_height;

        tracing::debug!(
            "Taskbar rect: ({}, {}, {}, {}), vertical: {}",
            taskbar_left,
            taskbar_top,
            taskbar_right,
            taskbar_bottom,
            is_taskbar_vertical
        );

        let cursor_x = cursor_pos.x as i32;
        let cursor_y = cursor_pos.y as i32;

        if is_taskbar_vertical {
            // Taskbar verticale (sinistra o destra)
            if taskbar_left < monitor_pos.x + 100 {
                // Taskbar a SINISTRA - menu a destra della tray
                let x = taskbar_right + 5;
                let y = (cursor_y - menu_height / 2).max(monitor_pos.y + 5);
                (x, y)
            } else {
                // Taskbar a DESTRA - menu a sinistra della tray
                let x = (taskbar_left - menu_width - 5).max(monitor_pos.x + 5);
                let y = (cursor_y - menu_height / 2).max(monitor_pos.y + 5);
                (x, y)
            }
        } else {
            // Taskbar orizzontale (alto o basso)
            // Centra il menu orizzontalmente rispetto al cursore
            let x = (cursor_x - menu_width / 2)
                .max(monitor_pos.x + 5)  // Non troppo a sinistra
                .min(monitor_pos.x + monitor_size.width as i32 - menu_width - 5); // Non troppo a destra

            if taskbar_top < monitor_pos.y + 100 {
                // Taskbar in ALTO - menu SOTTO la taskbar
                let y = taskbar_bottom + 5;
                (x, y)
            } else {
                // Taskbar in BASSO - menu SOPRA la taskbar
                let y = taskbar_top - menu_height - 5;
                (x, y)
            }
        }
    } else {
        // Fallback: nessuna info taskbar, usa posizione sicura
        tracing::warn!("Could not get taskbar rect, using fallback positioning");
        let x = (cursor_pos.x as i32 - menu_width / 2)
            .max(monitor_pos.x + 5)
            .min(monitor_pos.x + monitor_size.width as i32 - menu_width - 5);
        let y =
            (monitor_pos.y + monitor_size.height as i32 - menu_height - 80).max(monitor_pos.y + 5);
        (x, y)
    };

    tracing::info!("Positioning tray menu at: ({}, {})", final_x, final_y);

    // Applica la posizione
    if let Err(e) = window.set_position(tauri::PhysicalPosition {
        x: final_x,
        y: final_y,
    }) {
        tracing::error!("Failed to set menu position: {:?}", e);
    }
}

#[cfg(windows)]
pub fn get_taskbar_rect() -> Option<(i32, i32, i32, i32)> {
    use std::mem::zeroed;
    use windows_sys::Win32::UI::Shell::{SHAppBarMessage, ABM_GETTASKBARPOS, APPBARDATA};

    unsafe {
        let mut app_bar_data: APPBARDATA = zeroed();
        app_bar_data.cbSize = std::mem::size_of::<APPBARDATA>() as u32;

        let result = SHAppBarMessage(ABM_GETTASKBARPOS, &mut app_bar_data);
        if result != 0 {
            let rc = app_bar_data.rc;
            Some((rc.left, rc.top, rc.right, rc.bottom))
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
fn get_taskbar_rect() -> Option<(i32, i32, i32, i32)> {
    None
}
