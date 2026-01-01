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
    state: State<'_, TranslationState>,
    language: String,
    translations: HashMap<String, String>,
) -> Result<(), String> {
    let mut cache = state.write();
    cache.language = language;
    cache.translations = translations;
    tracing::info!("Translations cached for language: {}", cache.language);
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
