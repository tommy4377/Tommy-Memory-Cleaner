use once_cell::sync::Lazy;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, thiserror::Error)]
pub enum UpdaterError {
    #[error("Updater plugin error: {0}")]
    Updater(#[from] tauri_plugin_updater::Error),

    #[error("No update available")]
    NoUpdateAvailable,

    #[error("No ready update to install")]
    NoReadyUpdate,

    #[error("Update download failed: {0}")]
    DownloadFailed(String),

    #[error("Update install failed: {0}")]
    InstallFailed(String),
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

#[derive(Clone, Serialize)]
pub struct UpdateCheckResult {
    pub available: bool,
    pub current_version: String,
    pub available_version: Option<String>,
}

// ---------------------------------------------------------------------------
// Global state for a downloaded-but-not-yet-installed update.
// ---------------------------------------------------------------------------

/// The `Update` struct from the updater plugin. We keep it alive so we can call
/// `install()` on the same object that performed the download.
static PENDING_UPDATE: Lazy<Mutex<Option<tauri_plugin_updater::Update>>> =
    Lazy::new(|| Mutex::new(None));

/// The raw bytes returned by `Update::download()`.  These are needed because
/// `Update::install(&self, bytes)` requires the caller to pass the bytes.
static PENDING_UPDATE_BYTES: Lazy<Mutex<Option<Vec<u8>>>> =
    Lazy::new(|| Mutex::new(None));

/// A fast flag so we can check readiness without locking the Mutex.
static UPDATE_READY_FLAG: AtomicBool = AtomicBool::new(false);

// ---------------------------------------------------------------------------
// Public helpers (used by main.rs close handler, cmd_exit, etc.)
// ---------------------------------------------------------------------------

pub fn is_update_ready() -> bool {
    UPDATE_READY_FLAG.load(Ordering::SeqCst)
}

/// Consume the ready update and its bytes.  Returns `(Update, Vec<u8>)` if
/// both are present, or `None` if nothing is ready.
pub fn take_ready_update() -> Option<(tauri_plugin_updater::Update, Vec<u8>)> {
    let mut update_guard = PENDING_UPDATE.lock().unwrap();
    let mut bytes_guard = PENDING_UPDATE_BYTES.lock().unwrap();

    if let (Some(update), Some(bytes)) = (update_guard.take(), bytes_guard.take()) {
        UPDATE_READY_FLAG.store(false, Ordering::SeqCst);
        tracing::info!("Consumed ready update v{}", update.version);
        Some((update, bytes))
    } else {
        // One or both are missing — invalid state
        *update_guard = None;
        *bytes_guard = None;
        UPDATE_READY_FLAG.store(false, Ordering::SeqCst);
        None
    }
}

fn store_ready_update(update: tauri_plugin_updater::Update, bytes: Vec<u8>) {
    *PENDING_UPDATE.lock().unwrap() = Some(update);
    *PENDING_UPDATE_BYTES.lock().unwrap() = Some(bytes);
    UPDATE_READY_FLAG.store(true, Ordering::SeqCst);
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Check for an available update.
/// Returns structured information about whether an update exists.
#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<UpdateCheckResult, UpdaterError> {
    let current_version = crate::config::app_info::get_version().to_string();

    match app.updater() {
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => Ok(UpdateCheckResult {
                available: true,
                current_version,
                available_version: Some(update.version.clone()),
            }),
            Ok(None) => Ok(UpdateCheckResult {
                available: false,
                current_version,
                available_version: None,
            }),
            Err(e) => Err(UpdaterError::Updater(e)),
        },
        Err(e) => Err(UpdaterError::Updater(e)),
    }
}

/// Download an available update silently in the background.
/// Returns the new version string on success.
///
/// The downloaded bytes are stored in a global static so they can be installed
/// later (when the user closes the application).
#[tauri::command]
pub async fn download_update(app: AppHandle) -> Result<String, UpdaterError> {
    // Prevent simultaneous downloads
    if is_update_ready() {
        return Err(UpdaterError::DownloadFailed(
            "An update is already downloaded and ready".into(),
        ));
    }

    let updater = app.updater().map_err(UpdaterError::Updater)?;
    let update = updater
        .check()
        .await
        .map_err(UpdaterError::Updater)?
        .ok_or(UpdaterError::NoUpdateAvailable)?;

    let version = update.version.clone();

    tracing::info!("Downloading update v{}", version);

    let bytes = update
        .download(
            |chunk, total| {
                #[cfg(debug_assertions)]
                if let Some(total) = total {
                    tracing::debug!("Update download: {}/{} bytes", chunk, total);
                }
            },
            || {},
        )
        .await
        .map_err(|e| UpdaterError::DownloadFailed(e.to_string()))?;

    tracing::info!(
        "Update v{} downloaded successfully ({} bytes)",
        version,
        bytes.len()
    );

    store_ready_update(update, bytes);

    Ok(version)
}

/// Install the previously downloaded update and restart the application.
///
/// This command consumes the stored update and its bytes, runs the installer,
/// and then calls `app.restart()`.
#[tauri::command]
pub async fn install_ready_update(app: AppHandle) -> Result<(), UpdaterError> {
    let (update, bytes) = take_ready_update().ok_or(UpdaterError::NoReadyUpdate)?;

    tracing::info!("Installing ready update v{}", update.version);

    update
        .install(&bytes)
        .map_err(|e| UpdaterError::InstallFailed(e.to_string()))?;

    tracing::info!("Update installed, restarting application");

    app.restart();

    #[allow(unreachable_code)]
    Ok(())
}

/// Returns whether a downloaded update is ready to install.
#[tauri::command]
pub fn cmd_is_update_ready() -> bool {
    is_update_ready()
}

/// Returns the version string of a ready update, if any.
#[tauri::command]
pub fn cmd_get_ready_version() -> Option<String> {
    PENDING_UPDATE
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|u| u.version.clone()))
}
