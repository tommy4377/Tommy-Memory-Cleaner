use once_cell::sync::Lazy;
use std::collections::HashMap;
use serde_json::Value;

static TRANSLATIONS: Lazy<HashMap<String, HashMap<String, String>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    
    // Carica traduzioni EN
    let en_json = include_str!("../../ui/src/i18n/en.json");
    if let Ok(en) = serde_json::from_str::<HashMap<String, String>>(en_json) {
        map.insert("en".to_string(), en);
    }
    
    // Carica traduzioni IT
    let it_json = include_str!("../../ui/src/i18n/it.json");
    if let Ok(it) = serde_json::from_str::<HashMap<String, String>>(it_json) {
        map.insert("it".to_string(), it);
    }
    
    map
});

pub fn t(lang: &str, key: &str) -> String {
    TRANSLATIONS
        .get(lang)
        .and_then(|dict| dict.get(key))
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

pub fn t_with_params(lang: &str, key: &str, params: &[(&str, &str)]) -> String {
    let mut text = t(lang, key);
    for (k, v) in params {
        text = text.replace(&format!("%{}%", k), v);
    }
    text
}