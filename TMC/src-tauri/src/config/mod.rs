/// Configuration management module
///
/// Handles loading, saving, and validating application configuration
/// with support for portable installations and proper data directory handling.
use crate::memory::types::Areas;
use crate::security::{
    contains_injection_patterns, is_valid_hex_color, sanitize_hotkey, sanitize_process_name,
};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, io, path::PathBuf};

// ========== PORTABLE DETECTION ==========
/// Detects portable installation and manages data directories
#[derive(Debug, Clone)]
pub struct PortableDetector {
    is_portable: bool,
    exe_path: PathBuf,
    data_dir: PathBuf,
}

impl PortableDetector {
    /// Create new portable detector instance
    pub fn new() -> io::Result<Self> {
        let exe_path = std::env::current_exe()?;

        // The program is always "portable" (can be moved anywhere)
        // but data is ALWAYS saved in AppData for centralization
        let is_portable = true; // The program is portable (can be moved)

        // ALWAYS use AppData for data, regardless of exe location
        let data_dir = {
            #[cfg(windows)]
            {
                use std::env;
                // Try LOCALAPPDATA first, then APPDATA
                env::var("LOCALAPPDATA")
                    .or_else(|_| env::var("APPDATA"))
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| {
                        // Fallback to user directory
                        dirs::config_dir().unwrap_or_else(|| PathBuf::from("."))
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

        // Create directory if it doesn't exist
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)?;
        }

        // Log where we save the data
        tracing::info!("Data directory: {}", data_dir.display());
        tracing::info!(
            "Portable executable: {} (can be moved anywhere, data saved in AppData)",
            is_portable
        );

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

static PORTABLE: Lazy<RwLock<PortableDetector>> = Lazy::new(|| match PortableDetector::new() {
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
                // Profilo Normal: Working Set + Registry Cache + Standby List (Low Priority)
                // - Liberazione immediata senza latenza percepibile
                // ~540MB Working Set + ~1.86MB Registry Cache
                // NOTA: MODIFIED_PAGE_LIST non è incluso nel profilo Normal (come da specifiche utente)
                let mut areas = Areas::WORKING_SET | Areas::REGISTRY_CACHE;
                
                // Aggiunge Standby List Low Priority se disponibile
                if crate::os::has_standby_list_low() {
                    areas |= Areas::STANDBY_LIST_LOW;
                }
                
                areas
            }
            Profile::Balanced => {
                // Profilo Balanced: Include Normal + System File Cache + File Cache + Standby List (Full)
                // - Refresh profondo del sistema dopo uso intenso
                let mut areas = Areas::WORKING_SET | Areas::REGISTRY_CACHE;
                
                // Aggiunge aree aggiuntive
                areas |= Areas::SYSTEM_FILE_CACHE;
                areas |= Areas::STANDBY_LIST;
                
                // Standby List Low Priority se disponibile
                if crate::os::has_standby_list_low() {
                    areas |= Areas::STANDBY_LIST_LOW;
                }
                
                // Modified File Cache se disponibile
                if crate::os::has_modified_file_cache() {
                    areas |= Areas::MODIFIED_FILE_CACHE;
                }
                
                areas
            }
            Profile::Gaming => {
                // Profilo Gaming: Include Balanced + Modified Page List + Combined Page List
                // - Reset totale per gaming, tabula rasa della RAM
                // - Uses undocumented techniques for maximum performance
                let mut areas = Areas::WORKING_SET | Areas::REGISTRY_CACHE;
                
                // Tutte le aree del profilo Balanced
                areas |= Areas::SYSTEM_FILE_CACHE;
                areas |= Areas::STANDBY_LIST;
                
                // Aree aggiuntive per gaming
                areas |= Areas::MODIFIED_PAGE_LIST;
                
                // Aree dipendenti dalla versione Windows
                if crate::os::has_standby_list_low() {
                    areas |= Areas::STANDBY_LIST_LOW;
                }
                if crate::os::has_combined_page_list() {
                    areas |= Areas::COMBINED_PAGE_LIST;
                }
                if crate::os::has_modified_file_cache() {
                    areas |= Areas::MODIFIED_FILE_CACHE;
                }
                
                tracing::info!(
                    "Gaming profile areas: {:?} ({} areas)",
                    areas,
                    areas.bits().count_ones()
                );
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
            background_color_hex: "#2d8a3d".to_string(), // Original green but slightly less bright
            transparent_bg: false,
            warning_level: 80,
            warning_color_hex: "#d97706".to_string(), // Original orange but slightly less bright
            danger_level: 90,
            danger_color_hex: "#b91c1c".to_string(), // Original red but slightly less bright
        }
    }
}

impl TrayConfig {
    fn validate(&mut self) {
        // If colors are still old defaults (including "cold" ones), update to new balanced ones
        // Complete list of all old colors to update
        let old_defaults = [
            "#1C8C2D", "#15803d", "#34c759", "#28a745", "#2d5a3d", "#3d6b4d", "#1c8c2d", "#15803D",
            "#34C759", "#28A745", "#2D5A3D", "#3D6B4D",
            // "Cold" colors that might have been used
            "#2e7d32", "#388e3c", "#43a047", "#4caf50", "#66bb6a", "#81c784", "#2E7D32", "#388E3C",
            "#43A047", "#4CAF50", "#66BB6A", "#81C784",
        ];
        let old_warning = [
            "#FF9900", "#ff9500", "#8b6f47", "#b8864d", "#ff9900", "#FF9500", "#8B6F47", "#B8864D",
            // "Cold" warning colors
            "#f57c00", "#fb8c00", "#ff9800", "#ffa726", "#ffb74d", "#F57C00", "#FB8C00", "#FF9800",
            "#FFA726", "#FFB74D",
        ];
        let old_danger = [
            "#CC3300", "#ff3b30", "#dc3545", "#6b2d2d", "#8b3d3d", "#cc3300", "#FF3B30", "#DC3545",
            "#6B2D2D", "#8B3D3D", // "Cold" danger colors
            "#c62828", "#d32f2f", "#e53935", "#ef5350", "#e57373", "#C62828", "#D32F2F", "#E53935",
            "#EF5350", "#E57373",
        ];

        // Normalize colors for comparison (uppercase without spaces)
        let bg_normalized = self.background_color_hex.trim().to_uppercase();
        let warn_normalized = self.warning_color_hex.trim().to_uppercase();
        let danger_normalized = self.danger_color_hex.trim().to_uppercase();

        // Update only if they are old colors
        if old_defaults
            .iter()
            .any(|&c| c.to_uppercase() == bg_normalized)
        {
            self.background_color_hex = "#2d8a3d".to_string();
        } else {
            // Normalize format if not an old color
            self.background_color_hex =
                Self::normalize_hex_color(&self.background_color_hex, "#2d8a3d");
        }

        if old_warning
            .iter()
            .any(|&c| c.to_uppercase() == warn_normalized)
        {
            self.warning_color_hex = "#d97706".to_string();
        } else {
            // Normalize format if not an old color
            self.warning_color_hex = Self::normalize_hex_color(&self.warning_color_hex, "#d97706");
        }

        if old_danger
            .iter()
            .any(|&c| c.to_uppercase() == danger_normalized)
        {
            self.danger_color_hex = "#b91c1c".to_string();
        } else {
            // Normalize format if not an old color
            self.danger_color_hex = Self::normalize_hex_color(&self.danger_color_hex, "#b91c1c");
        }

        // Always normalize text color
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
    pub main_color_hex: String, // Deprecated, kept for compatibility
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
    "#9a8a72".to_string() // Default sepia for light theme
}

fn default_main_color_dark() -> String {
    "#0a84ff".to_string() // Default blue for dark theme
}

fn default_main_color() -> String {
    "#9a8a72".to_string() // Default sepia for light theme, will be overridden in dark
}

impl Default for Config {
    fn default() -> Self {
        let default_profile = Profile::Balanced;
        let default_areas = default_profile.get_memory_areas();
        let default_priority = Priority::High; // Default priority is High, not from profile

        // NO DEFAULT EXCLUSIONS!
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
            main_color_hex: "#0a84ff".to_string(), // Deprecated, kept for compatibility
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
        // FIX #11: Validate auto_opt_interval_hours - 0 means "disabled" and is valid
        // Limit only if > 0, otherwise 0 is a valid value to disable
        if self.auto_opt_interval_hours > 24 {
            self.auto_opt_interval_hours = 24;
        }
        // 0 is valid (disables scheduled auto-opt)

        // Validate and normalize main_color_hex
        if self.main_color_hex.is_empty() {
            self.main_color_hex = if self.theme == "dark" {
                "#0a84ff".to_string()
            } else {
                "#007aff".to_string()
            };
        } else {
            // Use validation function from security module
            if is_valid_hex_color(&self.main_color_hex) {
                let clean = self.main_color_hex.trim().trim_start_matches('#');
                self.main_color_hex = format!("#{}", clean.to_uppercase());
            } else {
                self.main_color_hex = if self.theme == "dark" {
                    "#0a84ff".to_string()
                } else {
                    "#007aff".to_string()
                };
            }
        }
        // FIX #11: Validate auto_opt_free_threshold - 0 means "disabled" and is valid
        // Limit only if > 0, otherwise 0 is a valid value to disable
        if self.auto_opt_free_threshold > 100 {
            self.auto_opt_free_threshold = 100;
        }
        // 0 is valid (disables auto-opt for low memory)
        self.font_size = self.font_size.clamp(8.0, 24.0);

        const VALID_LANGUAGES: &[&str] = &["en", "it", "es", "fr", "pt", "de", "ar", "ja", "zh"];
        if !VALID_LANGUAGES.contains(&self.language.as_str()) {
            self.language = "en".to_string();
        }

        if !["light", "dark"].contains(&self.theme.as_str()) {
            self.theme = "dark".to_string();
        }

        // Security: Validate and sanitize hotkey
        if contains_injection_patterns(&self.hotkey) {
            tracing::warn!("Potential injection in hotkey, resetting to default");
            self.hotkey = "Ctrl+Alt+N".to_string();
        } else {
            self.hotkey = sanitize_hotkey(&self.hotkey);
            if self.hotkey.trim().is_empty() {
                self.hotkey = "Ctrl+Alt+N".to_string();
            }
        }

        self.tray.validate();

        // Security: Sanitize process exclusion list
        let mut seen = BTreeSet::new();
        self.process_exclusion_list = self
            .process_exclusion_list
            .iter()
            .filter_map(|s| {
                let sanitized = sanitize_process_name(s);
                let trimmed = sanitized.trim();
                if !trimmed.is_empty() {
                    // Check for injection patterns
                    if contains_injection_patterns(trimmed) {
                        tracing::warn!("Potential injection in process exclusion: {}", trimmed);
                        None
                    } else {
                        let lower = trimmed.to_lowercase();
                        if seen.insert(lower.clone()) {
                            Some(trimmed.to_string())
                        } else {
                            None
                        }
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
        // Try to read all settings from the configuration file created by the installer
        // The installer saves in {userappdata}\TommyMemoryCleaner\config.json
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

        // Try to migrate from old location if needed
        if !path.exists() {
            // Check if there's a config in exe directory (old location)
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let old_config = exe_dir.join("config.json");
                    if old_config.exists() && old_config != path {
                        tracing::info!(
                            "Migrating config from {} to {}",
                            old_config.display(),
                            path.display()
                        );

                        // Copy old config to new location
                        if let Err(e) = fs::copy(&old_config, &path) {
                            tracing::warn!("Failed to migrate config: {}", e);
                        } else {
                            // Rename old one for backup
                            if let Err(e) = fs::rename(&old_config, exe_dir.join("config.json.old"))
                            {
                                tracing::debug!("Failed to rename old config for backup: {}", e);
                            }
                        }
                    }
                }
            }
        }

        let mut cfg = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<Self>(&content) {
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
                },
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
                if let Some(always_on_top) = installer_json
                    .get("always_on_top")
                    .and_then(|v| v.as_bool())
                {
                    default.always_on_top = always_on_top;
                }
                if let Some(notifications) = installer_json
                    .get("show_opt_notifications")
                    .and_then(|v| v.as_bool())
                {
                    default.show_opt_notifications = notifications;
                }
            }
            default
        };

        // FIX: Always apply settings from installer if present (not only if they are default)
        if let Some(installer_json) = Self::load_installer_settings() {
            // Always apply language from installer if present
            if let Some(lang) = installer_json.get("language").and_then(|v| v.as_str()) {
                cfg.language = lang.to_string();
            }
            // Always apply theme from installer if present
            if let Some(theme) = installer_json.get("theme").and_then(|v| v.as_str()) {
                cfg.theme = theme.to_string();
            }
            // Always apply always_on_top from installer if present
            if let Some(always_on_top) = installer_json
                .get("always_on_top")
                .and_then(|v| v.as_bool())
            {
                cfg.always_on_top = always_on_top;
            }
            // Always apply notifications from installer if present
            if let Some(notifications) = installer_json
                .get("show_opt_notifications")
                .and_then(|v| v.as_bool())
            {
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

        // Fallback 1: Ensure directory exists with retry
        {
            let portable = PORTABLE.read();
            let data_dir = portable.data_dir();
            if !data_dir.exists() {
                // Retry up to 3 times to create directory
                let mut last_error = None;
                for attempt in 1..=3 {
                    match fs::create_dir_all(&data_dir) {
                        Ok(_) => {
                            tracing::info!("Created data directory: {}", data_dir.display());
                            break;
                        }
                        Err(e) => {
                            let error_msg = format!("{}", e);
                            last_error = Some(e);
                            tracing::warn!(
                                "Failed to create data directory (attempt {}): {}",
                                attempt,
                                error_msg
                            );
                            if attempt < 3 {
                                std::thread::sleep(std::time::Duration::from_millis(
                                    100 * attempt as u64,
                                ));
                            }
                        }
                    }
                }
                if let Some(e) = last_error {
                    return Err(e);
                }
            }
        }

        // Fallback 2: Also create parent directory if necessary
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        // Fallback 3: Serialize with retry
        let content = match serde_json::to_string_pretty(self) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to serialize config: {:?}", e);
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Serialization error: {}", e),
                ));
            }
        };

        // Fallback 4: Atomic save with retry and backup
        let temp_path = path.with_extension("tmp");
        let backup_path = path.with_extension("json.bak");

        // Create backup of existing file if present
        if path.exists() {
            if let Err(e) = fs::copy(&path, &backup_path) {
                tracing::warn!("Failed to create backup: {:?}", e);
                // Don't block save if backup fails
            }
        }

        // Retry up to 3 times to write temporary file
        let mut last_error = None;
        for attempt in 1..=3 {
            match fs::write(&temp_path, &content) {
                Ok(_) => break,
                Err(e) => {
                    let error_msg = format!("{}", e);
                    last_error = Some(e);
                    tracing::warn!(
                        "Failed to write temp config (attempt {}): {}",
                        attempt,
                        error_msg
                    );
                    if attempt < 3 {
                        std::thread::sleep(std::time::Duration::from_millis(50 * attempt as u64));
                    }
                }
            }
        }

        if let Some(e) = last_error.take() {
            tracing::error!(
                "Failed to write config after retries, restoring from backup if available"
            );
            // Restore backup if available
            if backup_path.exists() && path.exists() {
                let _ = fs::copy(&backup_path, &path);
            }
            return Err(e);
        }

        // Fallback 5: Atomic rename with retry
        for attempt in 1..=3 {
            match fs::rename(&temp_path, &path) {
                Ok(_) => {
                    tracing::debug!("Config saved successfully to: {}", path.display());
                    // Remove old backup if everything is ok
                    if backup_path.exists() {
                        let _ = fs::remove_file(&backup_path);
                    }
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to rename temp config (attempt {}): {:?}",
                        attempt,
                        e
                    );
                    if attempt < 3 {
                        std::thread::sleep(std::time::Duration::from_millis(50 * attempt as u64));
                    } else {
                        // Ultimo tentativo fallito, ripristina backup
                        if backup_path.exists() && path.exists() {
                            let _ = fs::copy(&backup_path, &path);
                        }
                        return Err(e);
                    }
                }
            }
        }

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
        // DO NOT add default exclusions in migration

        if self.memory_areas.is_empty() {
            self.memory_areas = self.profile.get_memory_areas();
        }

        self.config_version = 2;
    }
}
