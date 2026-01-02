/// Internationalization (i18n) support module.
/// 
/// This module provides functionality for managing translations and localization
/// in the application. It includes caching mechanisms for translations received
/// from the frontend and utilities for retrieving translated strings.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

/// Represents a cached translation set for a specific language.
/// 
/// This structure stores the language code and a mapping of translation keys
/// to their corresponding translated strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationCache {
    /// The language code (e.g., "en", "it", "es")
    pub language: String,
    /// Mapping of translation keys to translated strings
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

/// Type alias for the thread-safe translation state.
pub type TranslationState = Arc<RwLock<TranslationCache>>;

/// Caches translations from the frontend for backend use.
/// 
/// This command receives translation data from the frontend and stores it
/// in a thread-safe cache for later use by backend components.
/// 
/// # Arguments
/// 
/// * `app_state` - The application state containing the translation cache
/// * `language` - The language code for the translations
/// * `translations` - HashMap of translation keys to translated strings
/// 
/// # Returns
/// 
/// Returns `Ok(())` if translations are cached successfully,
/// or an error string if the operation fails.
#[tauri::command]
pub fn cmd_set_translations(
    app_state: State<'_, crate::AppState>,
    language: String,
    translations: HashMap<String, String>,
) -> Result<(), String> {
    tracing::info!(
        "Received translations request for language: {} with {} keys",
        language,
        translations.len()
    );

    let mut cache = app_state.translations.write();
    cache.language = language;
    cache.translations = translations;
    tracing::info!(
        "Translations cached successfully for language: {}",
        cache.language
    );
    Ok(())
}

/// Retrieves a cached translation for the given key.
/// 
/// This function looks up the translation for the specified key in the
/// current language's translation cache. If the key is not found, it returns
/// the key itself and logs a warning.
/// 
/// # Arguments
/// 
/// * `state` - The translation state containing the cache
/// * `key` - The translation key to look up
/// 
/// # Returns
/// 
/// Returns the translated string if found, otherwise returns the key itself.
pub fn get_translation(state: &TranslationState, key: &str) -> String {
    let cache = state.read();
    let translation = cache.translations.get(key).cloned().unwrap_or_else(|| {
        tracing::warn!(
            "Translation not found for key: '{}' (language: {})",
            key,
            cache.language
        );
        key.to_string()
    });

    if translation != key {
        tracing::debug!("Found translation for '{}' -> '{}'", key, translation);
    }

    translation
}
