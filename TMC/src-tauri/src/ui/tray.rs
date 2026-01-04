use crate::engine::Engine;
use image::{ImageBuffer, Rgba, RgbaImage};
use rusttype::{point, Font, Scale};
use tauri::{image::Image, tray::TrayIconBuilder, AppHandle, Manager, Runtime};

use crate::TRAY_ICON_ID;

const ICON_SIZE: u32 = 32;

// Font embedded nel binario
const FONT_DATA: &[u8] = include_bytes!("../../fonts/Roboto-Bold.ttf");

fn hex_to_rgba(hex: &str) -> [u8; 4] {
    // FIX #7: Validare il formato hex prima del parsing e usare un default sensato
    let hex = hex.trim_start_matches('#');

    // Valida che sia esattamente 6 caratteri hex
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
        return [r, g, b, 255];
    }

    // Default grigio invece di verde se il formato è invalido
    tracing::debug!("Invalid hex color format: {}, using gray default", hex);
    [128, 128, 128, 255]
}

pub fn create_tray_icon(
    percentage: u8,
    bg_hex: &str,
    text_hex: &str,
    transparent: bool,
) -> Image<'static> {
    let render_size = ICON_SIZE * 2;

    let bg_color = hex_to_rgba(bg_hex);
    let text_color = hex_to_rgba(text_hex);

    let mut img: RgbaImage = if transparent {
        ImageBuffer::from_fn(render_size, render_size, |_, _| Rgba([0, 0, 0, 0]))
    } else {
        ImageBuffer::from_fn(render_size, render_size, |_, _| Rgba(bg_color))
    };

    if !transparent {
        apply_rounded_corners(&mut img, 12.0, bg_color);
    }

    // Try to load font, but don't crash if it fails - just create icon without text
    if let Some(font) = Font::try_from_bytes(FONT_DATA) {
        let val = percentage.min(99);
        let text = format!("{}", val);

        let scale = Scale::uniform(render_size as f32 * 0.75);
        let v_metrics = font.v_metrics(scale);

        let glyphs_temp: Vec<_> = font
            .layout(&text, scale, point(0.0, v_metrics.ascent))
            .collect();

        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;

        for glyph in &glyphs_temp {
            if let Some(bb) = glyph.pixel_bounding_box() {
                min_x = min_x.min(bb.min.x);
                max_x = max_x.max(bb.max.x);
                min_y = min_y.min(bb.min.y);
                max_y = max_y.max(bb.max.y);
            }
        }

        if min_x == i32::MAX {
            min_x = 0;
            max_x = render_size as i32;
            min_y = 0;
            max_y = render_size as i32;
        }

        let text_width = (max_x - min_x) as f32;
        let text_height = (max_y - min_y) as f32;

        let offset_x = (render_size as f32 - text_width) / 2.0 - min_x as f32;
        let offset_y = (render_size as f32 - text_height) / 2.0 - min_y as f32;

        for glyph in font.layout(&text, scale, point(offset_x, v_metrics.ascent + offset_y)) {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                glyph.draw(|gx, gy, v| {
                    let px = gx as i32 + bounding_box.min.x;
                    let py = gy as i32 + bounding_box.min.y;

                    if px >= 0 && px < render_size as i32 && py >= 0 && py < render_size as i32 {
                        let pixel = img.get_pixel_mut(px as u32, py as u32);
                        let alpha = (v * 255.0) as u8;
                        pixel.0 = blend_colors(pixel.0, text_color, alpha);
                    }
                });
            }
        }
    } else {
        tracing::warn!("Failed to load embedded font, creating icon without text");
    }

    let final_img = image::imageops::resize(
        &img,
        ICON_SIZE,
        ICON_SIZE,
        image::imageops::FilterType::Lanczos3,
    );

    let buffer: Vec<u8> = final_img.into_raw();
    Image::new_owned(buffer, ICON_SIZE, ICON_SIZE)
}

fn apply_rounded_corners(img: &mut RgbaImage, radius: f32, _bg_color: [u8; 4]) {
    let (width, height) = img.dimensions();
    let width_f = width as f32;
    let height_f = height as f32;

    for y in 0..height {
        for x in 0..width {
            let xf = x as f32;
            let yf = y as f32;

            let mut in_corner = false;
            let mut dx = 0.0;
            let mut dy = 0.0;

            if xf < radius && yf < radius {
                dx = radius - xf;
                dy = radius - yf;
                in_corner = true;
            } else if xf > width_f - radius && yf < radius {
                dx = xf - (width_f - radius);
                dy = radius - yf;
                in_corner = true;
            } else if xf < radius && yf > height_f - radius {
                dx = radius - xf;
                dy = yf - (height_f - radius);
                in_corner = true;
            } else if xf > width_f - radius && yf > height_f - radius {
                dx = xf - (width_f - radius);
                dy = yf - (height_f - radius);
                in_corner = true;
            }

            if in_corner {
                let distance = (dx * dx + dy * dy).sqrt();
                if distance > radius {
                    let pixel = img.get_pixel_mut(x, y);
                    pixel.0[3] = 0;
                } else if distance > radius - 1.0 {
                    let alpha = ((radius - distance) * 255.0) as u8;
                    let pixel = img.get_pixel_mut(x, y);
                    pixel.0[3] = (pixel.0[3] as f32 * (alpha as f32 / 255.0)) as u8;
                }
            }
        }
    }
}

fn blend_colors(bg: [u8; 4], fg: [u8; 4], alpha: u8) -> [u8; 4] {
    let alpha_f = alpha as f32 / 255.0;
    let inv_alpha = 1.0 - alpha_f;

    [
        ((fg[0] as f32 * alpha_f) + (bg[0] as f32 * inv_alpha)) as u8,
        ((fg[1] as f32 * alpha_f) + (bg[1] as f32 * inv_alpha)) as u8,
        ((fg[2] as f32 * alpha_f) + (bg[2] as f32 * inv_alpha)) as u8,
        255,
    ]
}

fn load_default_icon() -> Result<Image<'static>, String> {
    let ico_data = include_bytes!("../../icons/icon.ico");

    let img =
        image::load_from_memory(ico_data).map_err(|e| format!("Failed to load ICO: {}", e))?;

    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();

    let final_img = if width != ICON_SIZE || height != ICON_SIZE {
        image::imageops::resize(
            &rgba_img,
            ICON_SIZE,
            ICON_SIZE,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        rgba_img
    };

    let rgba_bytes: Vec<u8> = final_img.into_raw();
    Ok(Image::new_owned(rgba_bytes, ICON_SIZE, ICON_SIZE))
}

// Cache per l'icona di default
use std::sync::OnceLock;
static DEFAULT_ICON: OnceLock<Image<'static>> = OnceLock::new();

fn get_default_icon() -> Image<'static> {
    DEFAULT_ICON
        .get_or_init(|| {
            load_default_icon().unwrap_or_else(|e| {
                tracing::error!("Failed to load default icon: {}", e);
                // Fallback: crea un'icona vuota
                Image::new_owned(
                    vec![0u8; (ICON_SIZE * ICON_SIZE * 4) as usize],
                    ICON_SIZE,
                    ICON_SIZE,
                )
            })
        })
        .clone()
}

/// Update tray icon with current theme
pub fn update_tray_icon_with_theme<R: Runtime>(app: &AppHandle<R>, theme: &str) -> tauri::Result<()> {
    // For now, just log the theme change
    // TODO: Implement theme-specific tray icons when icons are available
    tracing::info!("Theme changed to: {}, tray icon update requested", theme);
    Ok(())
}

pub fn build<R: Runtime>(_app: &AppHandle<R>) -> tauri::Result<TrayIconBuilder<R>> {
    let icon = get_default_icon();

    Ok(TrayIconBuilder::new().icon(icon).tooltip("Memory Cleaner"))
}

// CORREZIONE 1: Ritorna Option<String> invece di Option<TrayIconId>
fn get_tray_id() -> Option<String> {
    TRAY_ICON_ID.lock().ok().and_then(|g| g.clone())
}

fn set_tray_icon(app: &AppHandle, icon: Image<'static>, tooltip: &str) {
    let tray_id = get_tray_id();

    let tray = tray_id
        .as_ref()
        .and_then(|id| app.tray_by_id(id))
        .or_else(|| app.tray_by_id("main"));

    if let Some(tray) = tray {
        let _ = tray.set_icon(Some(icon));
        let _ = tray.set_tooltip(Some(tooltip.to_string()));
    }
}

pub fn update_tray_icon(app: &AppHandle, mem_percent: u8) {
    // CORREZIONE 2: Risolve errore lifetime 'state does not live long enough'
    let state = app.state::<crate::AppState>();

    let tray_cfg = match state.cfg.try_lock() {
        Ok(cfg) => cfg.tray.clone(),
        Err(_) => {
            // Lock occupato, riprova dopo
            tracing::debug!("Config lock busy, skipping update");
            return;
        }
    };

    if !tray_cfg.show_mem_usage {
        set_tray_icon(app, get_default_icon(), "Memory Cleaner");
        return;
    }

    let bg = if mem_percent >= tray_cfg.danger_level {
        &tray_cfg.danger_color_hex
    } else if mem_percent >= tray_cfg.warning_level {
        &tray_cfg.warning_color_hex
    } else {
        &tray_cfg.background_color_hex
    };

    let icon = create_tray_icon(
        mem_percent,
        bg,
        &tray_cfg.text_color_hex,
        tray_cfg.transparent_bg,
    );

    // Try to get translated tooltip
    let tooltip = {
        let translated = crate::commands::get_translation(&state.translations, "RAM: %d%");

        // If translation is empty, use English format
        if translated.is_empty() {
            format!("RAM: {}%", mem_percent)
        } else {
            // Replace placeholder with actual percentage
            translated.replace("%d%", &mem_percent.to_string())
        }
    };

    set_tray_icon(app, icon, &tooltip);
}

/// Forza refresh dell'icona (chiamato quando cambia la config)
#[allow(dead_code)]
pub fn refresh_tray_icon(app: &AppHandle) {
    let (show_mem, mem_percent) = {
        let state = app.state::<crate::AppState>();
        // Percentuale 0 come placeholder, verrà aggiornata dal loop se necessario
        let show = state
            .cfg
            .lock()
            .map(|c| c.tray.show_mem_usage)
            .unwrap_or(true);
        (show, 0u8)
    };

    if !show_mem {
        tracing::debug!("refresh_tray_icon: setting default icon");
        set_tray_icon(app, get_default_icon(), "Memory Cleaner");
    } else {
        update_tray_icon(app, mem_percent);
    }
}

pub fn start_tray_updater(app: AppHandle, engine: Engine) {
    tauri::async_runtime::spawn(async move {
        let mut last_percent: f32 = -1.0; // Inizializza a valore impossibile

        loop {
            // FIX #12: Clona la configurazione del tray PRIMA di chiamare memory() per evitare race conditions
            // Questo assicura che anche se la config cambia durante l'esecuzione, usiamo valori consistenti
            let tray_cfg_opt = {
                let state = app.state::<crate::AppState>();
                let cfg_result = match state.cfg.try_lock() {
                    Ok(cfg) => Some(cfg.tray.clone()),
                    Err(_) => {
                        // Lock occupato, salta questo ciclo
                        tracing::debug!("Config lock busy in start_tray_updater, skipping cycle");
                        None
                    }
                };
                // Se il lock è occupato, aspetta e continua
                if cfg_result.is_none() {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    continue;
                }
                cfg_result
            };

            // Se la configurazione non mostra l'uso della memoria, usa l'icona di default
            if let Some(ref tray_cfg) = tray_cfg_opt {
                if !tray_cfg.show_mem_usage {
                    set_tray_icon(&app, get_default_icon(), "Memory Cleaner");
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    continue;
                }
            }

            // Ora ottieni la memoria e aggiorna l'icona solo se cambia significativamente
            if let Ok(mem) = engine.memory() {
                // Clamp percentage tra 0-100 (dovrebbe essere già nel range, ma per sicurezza)
                let current_percent = mem.physical.used.percentage.min(100) as f32;

                // Aggiorna solo se la variazione è > 0.5% o è il primo ciclo
                if last_percent < 0.0 || (current_percent - last_percent).abs() > 0.5 {
                    update_tray_icon(&app, current_percent as u8);
                    last_percent = current_percent;
                    #[cfg(debug_assertions)]
                    tracing::debug!("Tray icon updated: {:.1}% (change > 0.5%)", current_percent);
                } else {
                    // No update needed - change too small
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
}
