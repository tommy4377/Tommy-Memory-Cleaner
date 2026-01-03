use crate::config::app_info;
use serde_json::json;

#[tauri::command]
pub fn get_app_info() -> serde_json::Value {
    json!({
        "name": app_info::get_app_name(),
        "version": app_info::get_version(),
        "versionFull": app_info::get_version_full(),
        "company": app_info::get_company_name(),
        "copyright": app_info::get_copyright(),
        "description": app_info::FILE_DESCRIPTION
    })
}

#[tauri::command]
pub fn get_app_version() -> String {
    app_info::get_version().to_string()
}

#[tauri::command]
pub fn get_company_name() -> String {
    app_info::get_company_name().to_string()
}
