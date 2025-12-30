use crate::memory::types::Areas;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, io, path::PathBuf};
use once_cell::sync::Lazy;
use parking_lot::RwLock;

// ========== PORTABLE DETECTION ==========
#[derive(Debug, Clone)]
pub struct PortableDetector {
    is_portable: bool,
    exe_path: PathBuf,
    data_dir: PathBuf,
}

impl PortableDetector {
    pub fn new() -> io::Result<Self> {
        let exe_path = std::env::current_exe()?;
        
        // Il programma è sempre "portable" (può essere spostato ovunque)
        // ma i dati vengono salvati SEMPRE in AppData per centralizzazione
        let is_portable = true; // Il programma è portable (può essere spostato)
        
        // SEMPRE usa AppData per i dati, indipendentemente da dove si trova l'exe
        let data_dir = {
            #[cfg(windows)]
            {
                use std::env;
                // Prova prima LOCALAPPDATA poi APPDATA
                env::var("LOCALAPPDATA")
                    .or_else(|_| env::var("APPDATA"))
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| {
                        // Fallback a directory utente
                        dirs::config_dir()
                            .unwrap_or_else(|| PathBuf::from("."))
                    })
                    .join("TommyMemoryCleaner")
            }
            
            #[cfg(not(windows))]
            {
                dirs::config_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("TommyMemoryCleaner")
            }
        };
        
        // Crea directory se non esiste
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)?;
        }
        
        // Log dove salviamo i dati
        tracing::info!("Data directory: {}", data_dir.display());
        tracing::info!("Portable executable: {} (can be moved anywhere, data saved in AppData)", is_portable);
        
        Ok(Self {
            is_portable,
            exe_path,
            data_dir,
        })
    }
    
    pub fn is_portable(&self) -> bool {
        self.is_portable
    }
    
    pub fn config_path(&self) -> PathBuf {
        self.data_dir.join("config.json")
    }
    
    pub fn exe_path(&self) -> &PathBuf {
        &self.exe_path
    }
    
    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }
}

static PORTABLE: Lazy<RwLock<PortableDetector>> = Lazy::new(|| {
    match PortableDetector::new() {
        Ok(detector) => RwLock::new(detector),
        Err(e) => {
            eprintln!("Failed to initialize portable detector: {}", e);
            RwLock::new(PortableDetector {
                is_portable: false,
                exe_path: std::env::current_exe().unwrap_or_else(|err| {
                    tracing::error!("Failed to get exe path: {}, using fallback", err);
                    PathBuf::from(".")
                }),
                data_dir: PathBuf::from(".").join("TommyMemoryCleaner"),
            })
        }
    }
});

pub fn get_portable_detector() -> PortableDetector {
    PORTABLE.read().clone()
}

fn config_path() -> PathBuf {
    PORTABLE.read().config_path()
}

// ========== ENUMS ==========
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum Priority {
    Low,
    Normal,
    High,
}

impl Default for Priority {
    fn default() -> Self {
        Self::High
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum Profile {
    Normal,
    Balanced,
    Gaming,
}

impl Default for Profile {
    fn default() -> Self {
        Self::Balanced
    }
}

impl Profile {
    pub fn get_memory_areas(&self) -> Areas {
        match self {
            Profile::Normal => {
                // Profilo leggero: aree essenziali + registry cache (molto leggero e efficace)
                Areas::WORKING_SET 
                | Areas::MODIFIED_PAGE_LIST
                | Areas::REGISTRY_CACHE
            },
            Profile::Balanced => {
                // Profilo bilanciato: aree principali + modified file cache + registry cache per efficienza
                let mut areas = Areas::WORKING_SET
                    | Areas::MODIFIED_PAGE_LIST 
                    | Areas::STANDBY_LIST
                    | Areas::SYSTEM_FILE_CACHE
                    | Areas::REGISTRY_CACHE;
                
                // Aggiungi Modified File Cache se disponibile (utile per performance)
                if crate::os::has_modified_file_cache() {
                    areas |= Areas::MODIFIED_FILE_CACHE;
                    tracing::debug!("Balanced profile: MODIFIED_FILE_CACHE available");
                }
                
                areas
            },
            Profile::Gaming => {
                // FIX: Gaming profile usa TUTTE le aree disponibili per massime prestazioni
                let mut areas = Areas::empty();
                
                // Aree base sempre disponibili
                areas |= Areas::WORKING_SET;
                areas |= Areas::MODIFIED_PAGE_LIST;
                areas |= Areas::STANDBY_LIST;
                areas |= Areas::SYSTEM_FILE_CACHE;
                areas |= Areas::REGISTRY_CACHE;
                
                // Aree avanzate (solo se disponibili su questa versione di Windows)
                // La validazione finale in engine.rs rimuoverà quelle non disponibili
                if crate::os::has_standby_list_low() {
                    areas |= Areas::STANDBY_LIST_LOW;
                    tracing::debug!("Gaming profile: STANDBY_LIST_LOW available");
                } else {
                    tracing::debug!("Gaming profile: STANDBY_LIST_LOW NOT available");
                }
                if crate::os::has_combined_page_list() {
                    areas |= Areas::COMBINED_PAGE_LIST;
                    tracing::debug!("Gaming profile: COMBINED_PAGE_LIST available");
                } else {
                    tracing::debug!("Gaming profile: COMBINED_PAGE_LIST NOT available");
                }
                if crate::os::has_modified_file_cache() {
                    areas |= Areas::MODIFIED_FILE_CACHE;
                    tracing::debug!("Gaming profile: MODIFIED_FILE_CACHE available");
                } else {
                    tracing::debug!("Gaming profile: MODIFIED_FILE_CACHE NOT available");
                }
                
                tracing::info!("Gaming profile areas: {:?} ({} areas)", areas, areas.bits().count_ones());
                areas
            }
        }
    }

    pub fn get_priority(&self) -> Priority {
        match self {
            Profile::Normal => Priority::Low,
            Profile::Balanced => Priority::Normal,
            Profile::Gaming => Priority::High,
        }
    }
}


// ========== TRAY CONFIG ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrayConfig {
    pub show_mem_usage: bool,
    pub text_color_hex: String,
    pub background_color_hex: String,
    pub transparent_bg: bool,
    pub warning_level: u8,
    pub warning_color_hex: String,
    pub danger_level: u8,
    pub danger_color_hex: String,
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            show_mem_usage: true,
            text_color_hex: "#FFFFFF".to_string(),
            background_color_hex: "#2d8a3d".to_string(), // Verde originale ma leggermente meno acceso
            transparent_bg: false,
            warning_level: 80,
            warning_color_hex: "#d97706".to_string(), // Arancione originale ma leggermente meno acceso
            danger_level: 90,
            danger_color_hex: "#b91c1c".to_string(), // Rosso originale ma leggermente meno acceso
        }
    }
}

impl TrayConfig {
    fn validate(&mut self) {
        // Se i colori sono ancora i vecchi default (inclusi quelli "freddi"), aggiornali ai nuovi bilanciati
        // Lista completa di tutti i vecchi colori da aggiornare
        let old_defaults = [
            "#1C8C2D", "#15803d", "#34c759", "#28a745", "#2d5a3d", "#3d6b4d", 
            "#1c8c2d", "#15803D", "#34C759", "#28A745", "#2D5A3D", "#3D6B4D",
            // Colori "freddi" che potrebbero essere stati usati
            "#2e7d32", "#388e3c", "#43a047", "#4caf50", "#66bb6a", "#81c784",
            "#2E7D32", "#388E3C", "#43A047", "#4CAF50", "#66BB6A", "#81C784"
        ];
        let old_warning = [
            "#FF9900", "#ff9500", "#8b6f47", "#b8864d",
            "#ff9900", "#FF9500", "#8B6F47", "#B8864D",
            // Colori warning "freddi"
            "#f57c00", "#fb8c00", "#ff9800", "#ffa726", "#ffb74d",
            "#F57C00", "#FB8C00", "#FF9800", "#FFA726", "#FFB74D"
        ];
        let old_danger = [
            "#CC3300", "#ff3b30", "#dc3545", "#6b2d2d", "#8b3d3d",
            "#cc3300", "#FF3B30", "#DC3545", "#6B2D2D", "#8B3D3D",
            // Colori danger "freddi"
            "#c62828", "#d32f2f", "#e53935", "#ef5350", "#e57373",
            "#C62828", "#D32F2F", "#E53935", "#EF5350", "#E57373"
        ];
        
        // Normalizza i colori per il confronto (uppercase senza spazi)
        let bg_normalized = self.background_color_hex.trim().to_uppercase();
        let warn_normalized = self.warning_color_hex.trim().to_uppercase();
        let danger_normalized = self.danger_color_hex.trim().to_uppercase();
        
        // Aggiorna solo se sono vecchi colori
        if old_defaults.iter().any(|&c| c.to_uppercase() == bg_normalized) {
            self.background_color_hex = "#2d8a3d".to_string();
        } else {
            // Normalizza il formato se non è un vecchio colore
            self.background_color_hex = Self::normalize_hex_color(&self.background_color_hex, "#2d8a3d");
        }
        
        if old_warning.iter().any(|&c| c.to_uppercase() == warn_normalized) {
            self.warning_color_hex = "#d97706".to_string();
        } else {
            // Normalizza il formato se non è un vecchio colore
            self.warning_color_hex = Self::normalize_hex_color(&self.warning_color_hex, "#d97706");
        }
        
        if old_danger.iter().any(|&c| c.to_uppercase() == danger_normalized) {
            self.danger_color_hex = "#b91c1c".to_string();
        } else {
            // Normalizza il formato se non è un vecchio colore
            self.danger_color_hex = Self::normalize_hex_color(&self.danger_color_hex, "#b91c1c");
        }
        
        // Normalizza sempre il colore del testo
        self.text_color_hex = Self::normalize_hex_color(&self.text_color_hex, "#FFFFFF");
        
        if self.warning_level >= self.danger_level {
            self.warning_level = 80;
            self.danger_level = 90;
        }
        
        self.warning_level = self.warning_level.clamp(50, 95);
        self.danger_level = self.danger_level.clamp(60, 100);
    }
    
    fn normalize_hex_color(color: &str, default: &str) -> String {
        let clean = color.trim().trim_start_matches('#');
        
        if clean.len() == 6 && clean.chars().all(|c| c.is_ascii_hexdigit()) {
            format!("#{}", clean.to_uppercase())
        } else {
            default.to_string()
        }
    }
}

// ========== MAIN CONFIG ==========
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub always_on_top: bool,
    pub minimize_to_tray: bool,
    pub close_after_opt: bool,
    pub compact_mode: bool,
    pub auto_opt_interval_hours: u32,
    pub auto_opt_free_threshold: u8,
    pub auto_update: bool,
    pub font_size: f32,
    pub language: String,
    pub theme: String,
    #[serde(default = "default_main_color")]
    pub main_color_hex: String, // Deprecated, mantenuto per compatibilità
    #[serde(default = "default_main_color_light")]
    pub main_color_hex_light: String,
    #[serde(default = "default_main_color_dark")]
    pub main_color_hex_dark: String,
    pub profile: Profile,
    pub memory_areas: Areas,
    pub hotkey: String,
    pub process_exclusion_list: BTreeSet<String>,
    pub run_priority: Priority,
    pub run_on_startup: bool,
    pub show_opt_notifications: bool,
    pub tray: TrayConfig,
    
    #[serde(default)]
    pub is_portable_install: bool,
    
    #[serde(default = "default_config_version")]
    pub config_version: u32,
    
    #[serde(default = "default_setup_completed")]
    pub setup_completed: bool,
}

fn default_setup_completed() -> bool {
    false
}

fn default_config_version() -> u32 {
    2
}

fn default_main_color_light() -> String {
    "#9a8a72".to_string() // Default sepia per light theme
}

fn default_main_color_dark() -> String {
    "#0a84ff".to_string() // Default blu per dark theme
}

fn default_main_color() -> String {
    "#9a8a72".to_string() // Default sepia per light theme, sarà sovrascritto in dark
}

impl Default for Config {
    fn default() -> Self {
        let default_profile = Profile::Balanced;
        let default_areas = default_profile.get_memory_areas();
        let default_priority = Priority::High; // Default priority is High, not from profile
        
        // NESSUNA ESCLUSIONE DI DEFAULT!
        let exclusions = BTreeSet::new();
        
        Self {
            always_on_top: false,
            minimize_to_tray: true,
            close_after_opt: false,
            compact_mode: false,
            auto_opt_interval_hours: 1,
            auto_opt_free_threshold: 30,
            auto_update: true,
            font_size: 13.0,
            language: "en".to_string(),
            theme: "dark".to_string(),
            main_color_hex: "#0a84ff".to_string(), // Deprecated, mantenuto per compatibilità
            main_color_hex_light: default_main_color_light(),
            main_color_hex_dark: default_main_color_dark(),
            profile: default_profile,
            memory_areas: default_areas,
            hotkey: "Ctrl+Alt+N".to_string(),
            process_exclusion_list: exclusions,
            run_priority: default_priority,
            run_on_startup: true,
            show_opt_notifications: true,
            tray: TrayConfig::default(),
            is_portable_install: false,
            config_version: default_config_version(),
            setup_completed: false,
        }
    }
}

impl Config {
    pub fn validate(&mut self) {
        // FIX #11: Valida auto_opt_interval_hours - 0 significa "disabilitato" ed è valido
        // Limita solo se > 0, altrimenti 0 è un valore valido per disabilitare
        if self.auto_opt_interval_hours > 24 {
            self.auto_opt_interval_hours = 24;
        }
        // 0 è valido (disabilita auto-opt programmata)
        
        // Valida e normalizza main_color_hex
        if self.main_color_hex.is_empty() {
            self.main_color_hex = if self.theme == "dark" {
                "#0a84ff".to_string()
            } else {
                "#007aff".to_string()
            };
        } else {
            // Normalizza il formato del colore
            let clean = self.main_color_hex.trim().trim_start_matches('#');
            if clean.len() == 6 && clean.chars().all(|c| c.is_ascii_hexdigit()) {
                self.main_color_hex = format!("#{}", clean.to_uppercase());
            } else {
                self.main_color_hex = if self.theme == "dark" {
                    "#0a84ff".to_string()
                } else {
                    "#007aff".to_string()
                };
            }
        }
        // FIX #11: Valida auto_opt_free_threshold - 0 significa "disabilitato" ed è valido
        // Limita solo se > 0, altrimenti 0 è un valore valido per disabilitare
        if self.auto_opt_free_threshold > 100 {
            self.auto_opt_free_threshold = 100;
        }
        // 0 è valido (disabilita auto-opt per memoria bassa)
        self.font_size = self.font_size.clamp(8.0, 24.0);
        
        const VALID_LANGUAGES: &[&str] = &["en", "it", "es", "fr", "pt", "de", "ar", "ja", "zh"];
        if !VALID_LANGUAGES.contains(&self.language.as_str()) {
            self.language = "en".to_string();
        }
        
        if !["light", "dark"].contains(&self.theme.as_str()) {
            self.theme = "dark".to_string();
        }
        
        if self.hotkey.trim().is_empty() {
            self.hotkey = "Ctrl+Alt+N".to_string();
        }
        
        self.tray.validate();
        
        let mut seen = BTreeSet::new();
        self.process_exclusion_list = self.process_exclusion_list
            .iter()
            .filter_map(|s| {
                let trimmed = s.trim();
                if !trimmed.is_empty() {
                    let lower = trimmed.to_lowercase();
                    if seen.insert(lower.clone()) {
                        Some(trimmed.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        
        self.is_portable_install = PORTABLE.read().is_portable();
        
        if self.memory_areas.is_empty() {
            self.memory_areas = self.profile.get_memory_areas();
        }
        
        // NOTE: run_priority is now independent from profile, so don't override it
        // The user can set it manually and it won't be changed by profile changes
    }
    
    fn load_installer_settings() -> Option<serde_json::Value> {
        // Prova a leggere tutte le impostazioni dal file di configurazione creato dall'installer
        // L'installer salva in {userappdata}\TommyMemoryCleaner\config.json
        #[cfg(windows)]
        {
            use std::env;
            if let Ok(appdata) = env::var("APPDATA") {
                let installer_config = std::path::PathBuf::from(appdata)
                    .join("TommyMemoryCleaner")
                    .join("config.json");
                if let Ok(content) = fs::read_to_string(&installer_config) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        return Some(json);
                    }
                }
            }
        }
        None
    }
    
    pub fn load() -> io::Result<Self> {
        let path = config_path();
        
        // Prova a migrare da vecchia location se necessario
        if !path.exists() {
            // Controlla se c'è un config nella directory dell'exe (vecchia location)
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let old_config = exe_dir.join("config.json");
                    if old_config.exists() && old_config != path {
                        tracing::info!("Migrating config from {} to {}", 
                                     old_config.display(), path.display());
                        
                        // Copia il vecchio config nella nuova location
                        if let Err(e) = fs::copy(&old_config, &path) {
                            tracing::warn!("Failed to migrate config: {}", e);
                        } else {
                            // Rinomina il vecchio per backup
                            if let Err(e) = fs::rename(&old_config, exe_dir.join("config.json.old")) {
                                tracing::debug!("Failed to rename old config for backup: {}", e);
                            }
                        }
                    }
                }
            }
        }
        
        let mut cfg = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str::<Self>(&content) {
                        Ok(mut c) => {
                            c.migrate_if_needed();
                            c
                        }
                        Err(e) => {
                            eprintln!("Failed to parse config: {}. Using defaults.", e);
                            let backup_path = path.with_extension("json.bak");
                            let _ = fs::copy(&path, backup_path);
                            Self::default()
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read config: {}. Using defaults.", e);
                    Self::default()
                }
            }
        } else {
            let mut default = Self::default();
            // FIX: Prova a caricare tutte le impostazioni dall'installer se esiste
            if let Some(installer_json) = Self::load_installer_settings() {
                if let Some(lang) = installer_json.get("language").and_then(|v| v.as_str()) {
                    default.language = lang.to_string();
                }
                if let Some(theme) = installer_json.get("theme").and_then(|v| v.as_str()) {
                    default.theme = theme.to_string();
                }
                if let Some(always_on_top) = installer_json.get("always_on_top").and_then(|v| v.as_bool()) {
                    default.always_on_top = always_on_top;
                }
                if let Some(notifications) = installer_json.get("show_opt_notifications").and_then(|v| v.as_bool()) {
                    default.show_opt_notifications = notifications;
                }
            }
            default
        };
        
        // FIX: Applica sempre le impostazioni dall'installer se presente (non solo se sono default)
        if let Some(installer_json) = Self::load_installer_settings() {
            // Applica sempre la lingua dall'installer se presente
            if let Some(lang) = installer_json.get("language").and_then(|v| v.as_str()) {
                cfg.language = lang.to_string();
            }
            // Applica sempre il tema dall'installer se presente
            if let Some(theme) = installer_json.get("theme").and_then(|v| v.as_str()) {
                cfg.theme = theme.to_string();
            }
            // Applica sempre always_on_top dall'installer se presente
            if let Some(always_on_top) = installer_json.get("always_on_top").and_then(|v| v.as_bool()) {
                cfg.always_on_top = always_on_top;
            }
            // Applica sempre le notifiche dall'installer se presente
            if let Some(notifications) = installer_json.get("show_opt_notifications").and_then(|v| v.as_bool()) {
                cfg.show_opt_notifications = notifications;
            }
        }
        
        cfg.validate();
        
        if let Err(e) = cfg.save() {
            eprintln!("Warning: Failed to save validated config: {}", e);
        }
        
        Ok(cfg)
    }

    pub fn save(&self) -> io::Result<()> {
        let path = config_path();
        
        // Usa data_dir per assicurarsi che la directory esista
        {
            let portable = PORTABLE.read();
            let data_dir = portable.data_dir();
            if !data_dir.exists() {
                fs::create_dir_all(data_dir)?;
            }
        }
        
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = serde_json::to_string_pretty(self)?;
        
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, content)?;
        fs::rename(temp_path, path)?;
        
        Ok(())
    }

    pub fn process_exclusion_list_lower(&self) -> Vec<String> {
        self.process_exclusion_list
            .iter()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    }
    
    fn migrate_if_needed(&mut self) {
        if self.config_version < 2 {
            self.migrate_v1_to_v2();
        }
    }
    
    fn migrate_v1_to_v2(&mut self) {
        // NON aggiungere esclusioni di default nella migrazione
        
        if self.memory_areas.is_empty() {
            self.memory_areas = self.profile.get_memory_areas();
        }
        
        self.config_version = 2;
    }
}