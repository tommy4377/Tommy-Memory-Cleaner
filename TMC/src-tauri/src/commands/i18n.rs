use tauri::State;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationCache {
    pub language: String,
    pub translations: HashMap<String, String>,
}

impl Default for TranslationCache {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            translations: HashMap::new(),
        }
    }
}

pub type TranslationState = Arc<RwLock<TranslationCache>>;

/// Cache translations from frontend for backend use
#[tauri::command]
pub fn cmd_set_translations(
    app_state: State<'_, crate::AppState>,
    language: String,
    translations: HashMap<String, String>,
) -> Result<(), String> {
    tracing::info!("Received translations request for language: {} with {} keys", language, translations.len());
    
    let mut cache = app_state.translations.write();
    cache.language = language;
    cache.translations = translations;
    tracing::info!("Translations cached successfully for language: {}", cache.language);
    Ok(())
}

/// Get a cached translation
pub fn get_translation(state: &TranslationState, key: &str) -> String {
    let cache = state.read();
    let translation = cache.translations.get(key).cloned().unwrap_or_else(|| {
        tracing::warn!("Translation not found for key: '{}' (language: {})", key, cache.language);
        key.to_string()
    });
    
    if translation != key {
        tracing::debug!("Found translation for '{}' -> '{}'", key, translation);
    }
    
    translation
}
