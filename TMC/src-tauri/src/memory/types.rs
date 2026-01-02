use serde::{Deserialize, Serialize};
use std::fmt;

// ========== MEMORY AREAS ==========
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct Areas: u32 {
        const NONE                = 0;
        const COMBINED_PAGE_LIST  = 1 << 0;
        const MODIFIED_FILE_CACHE = 1 << 1;
        const MODIFIED_PAGE_LIST  = 1 << 2;
        const REGISTRY_CACHE      = 1 << 3;
        const STANDBY_LIST        = 1 << 4;
        const STANDBY_LIST_LOW    = 1 << 5;
        const SYSTEM_FILE_CACHE   = 1 << 6;
        const WORKING_SET         = 1 << 7;

        // Presets
        const BASIC = Self::WORKING_SET.bits()
                    | Self::MODIFIED_PAGE_LIST.bits();

        const STANDARD = Self::BASIC.bits()
                       | Self::STANDBY_LIST.bits()
                       | Self::SYSTEM_FILE_CACHE.bits();

        const FULL = Self::STANDARD.bits()
                   | Self::COMBINED_PAGE_LIST.bits()
                   | Self::MODIFIED_FILE_CACHE.bits()
                   | Self::REGISTRY_CACHE.bits()
                   | Self::STANDBY_LIST_LOW.bits();
    }
}

impl Areas {
    /// Get human-readable names for the areas
    pub fn get_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();

        if self.contains(Areas::WORKING_SET) {
            names.push("Working Set");
        }
        if self.contains(Areas::MODIFIED_PAGE_LIST) {
            names.push("Modified Page List");
        }
        if self.contains(Areas::STANDBY_LIST) {
            names.push("Standby List");
        }
        if self.contains(Areas::STANDBY_LIST_LOW) {
            names.push("Low Priority Standby");
        }
        if self.contains(Areas::SYSTEM_FILE_CACHE) {
            names.push("System File Cache");
        }
        if self.contains(Areas::COMBINED_PAGE_LIST) {
            names.push("Combined Page List");
        }
        if self.contains(Areas::MODIFIED_FILE_CACHE) {
            names.push("Modified File Cache");
        }
        if self.contains(Areas::REGISTRY_CACHE) {
            names.push("Registry Cache");
        }

        names
    }
}

impl fmt::Display for Areas {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = self.get_names();
        if names.is_empty() {
            write!(f, "None")
        } else {
            write!(f, "{}", names.join(", "))
        }
    }
}

// ========== OPTIMIZATION REASON ==========
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Reason {
    LowMemory,
    Manual,
    Schedule,
    Hotkey,
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reason::LowMemory => write!(f, "Low Memory"),
            Reason::Manual => write!(f, "Manual"),
            Reason::Schedule => write!(f, "Scheduled"),
            Reason::Hotkey => write!(f, "Hotkey"),
        }
    }
}

// ========== MEMORY UNITS ==========
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Unit {
    B,
    KB,
    MB,
    GB,
    TB,
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unit::B => write!(f, "B"),
            Unit::KB => write!(f, "KB"),
            Unit::MB => write!(f, "MB"),
            Unit::GB => write!(f, "GB"),
            Unit::TB => write!(f, "TB"),
        }
    }
}

// ========== MEMORY SIZE ==========
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MemorySize {
    pub value: f64,
    pub unit: Unit,
    pub percentage: u8,
    pub bytes: u64,
}

impl MemorySize {
    pub fn new(bytes: u64, percentage: u8) -> Self {
        let (value, unit) = Self::bytes_to_unit(bytes);
        Self {
            value,
            unit,
            percentage,
            bytes,
        }
    }

    fn bytes_to_unit(bytes: u64) -> (f64, Unit) {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;
        const TB: f64 = GB * 1024.0;

        let b = bytes as f64;

        if b >= TB {
            (b / TB, Unit::TB)
        } else if b >= GB {
            (b / GB, Unit::GB)
        } else if b >= MB {
            (b / MB, Unit::MB)
        } else if b >= KB {
            (b / KB, Unit::KB)
        } else {
            (b, Unit::B)
        }
    }
}

impl fmt::Display for MemorySize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} {} ({}%)", self.value, self.unit, self.percentage)
    }
}

// ========== MEMORY STATS ==========
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MemoryStats {
    pub free: MemorySize,
    pub used: MemorySize,
    pub total: MemorySize,
}

impl MemoryStats {
    pub fn new(free_bytes: u64, total_bytes: u64) -> Self {
        let used_bytes = total_bytes.saturating_sub(free_bytes);

        let total_f = total_bytes as f64;
        let used_pct = if total_bytes > 0 {
            ((used_bytes as f64 / total_f) * 100.0).round() as u8
        } else {
            0
        };
        let free_pct = 100u8.saturating_sub(used_pct);

        Self {
            free: MemorySize::new(free_bytes, free_pct),
            used: MemorySize::new(used_bytes, used_pct),
            total: MemorySize::new(total_bytes, 100),
        }
    }
}

// ========== MEMORY INFO ==========
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub physical: MemoryStats,
    pub commit: MemoryStats,
    pub load_percent: u32,
}

// ========== HELPER FUNCTIONS (STILL USED) ==========
#[inline]
pub fn mk_stats(free: u64, total: u64, used_percent_opt: Option<u8>) -> MemoryStats {
    if let Some(used_pct) = used_percent_opt {
        // If used percent is provided, use it
        let free_pct = 100u8.saturating_sub(used_pct);
        let used = total.saturating_sub(free);

        MemoryStats {
            free: MemorySize::new(free, free_pct),
            used: MemorySize::new(used, used_pct),
            total: MemorySize::new(total, 100),
        }
    } else {
        // Calculate from bytes
        MemoryStats::new(free, total)
    }
}

// ========== TESTS ==========
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_areas_display() {
        let areas = Areas::WORKING_SET | Areas::STANDBY_LIST;
        let display = format!("{}", areas);
        assert!(display.contains("Working Set"));
        assert!(display.contains("Standby List"));
    }

    #[test]
    fn test_memory_size() {
        let size = MemorySize::new(1024 * 1024 * 1024, 50); // 1 GB
        assert_eq!(size.unit, Unit::GB);
        assert_eq!(size.value, 1.0);
    }

    #[test]
    fn test_memory_stats() {
        let stats = MemoryStats::new(512 * 1024 * 1024, 1024 * 1024 * 1024); // 512MB free of 1GB
        assert_eq!(stats.used.percentage, 50);
        assert_eq!(stats.free.percentage, 50);
    }
}
