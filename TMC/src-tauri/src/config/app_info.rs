// Application metadata constants - centralized for consistency
pub const APP_NAME: &str = "Tommy Memory Cleaner";
pub const APP_ID: &str = "TommyMemoryCleaner";
pub const COMPANY_NAME: &str = "Tommy437";
pub const VERSION: &str = "2.5.0";
pub const VERSION_FULL: &str = "2.5.0.0";
pub const FILE_DESCRIPTION: &str = "Advanced Memory Optimization Tool for Windows";
pub const COPYRIGHT: &str = "© 2025 Tommy437. All rights reserved.";

// Get application version in different formats
pub fn get_version() -> &'static str {
    VERSION
}

pub fn get_version_full() -> &'static str {
    VERSION_FULL
}

pub fn get_app_name() -> &'static str {
    APP_NAME
}

pub fn get_company_name() -> &'static str {
    COMPANY_NAME
}

pub fn get_app_id() -> &'static str {
    APP_ID
}

pub fn get_copyright() -> &'static str {
    COPYRIGHT
}
