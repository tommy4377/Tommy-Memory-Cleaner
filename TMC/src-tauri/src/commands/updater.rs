use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, thiserror::Error)]
pub enum UpdaterError {
    #[error(transparent)]
    Updater(#[from] tauri_plugin_updater::Error),
}

impl Serialize for UpdaterError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

#[derive(Clone, Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub notes: Option<String>,
    pub date: Option<String>,
}

/// Check for an available update.
/// Returns `Some(UpdateInfo)` if an update is available, `None` otherwise.
#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<Option<UpdateInfo>, UpdaterError> {
    let update = app.updater()?.check().await?;

    Ok(update.map(|u| UpdateInfo {
        version: u.version.clone(),
        current_version: u.current_version.clone(),
        notes: u.body.clone(),
        date: u.date.map(|d| d.to_string()),
    }))
}

/// Download and install the pending update, then restart the app.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), UpdaterError> {
    if let Some(update) = app.updater()?.check().await? {
        update
            .download_and_install(
                |_chunk_length, _content_length| {},
                || {},
            )
            .await?;
        app.restart();
    }
    Ok(())
}
