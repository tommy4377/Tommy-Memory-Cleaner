use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
pub struct ProgressEvent {
    pub value: u8,
    pub total: u8,
    pub step: String,
}

pub const EV_PROGRESS: &str = "tmc://opt_progress";
pub const EV_DONE: &str = "tmc://opt_done";

pub fn emit_progress(app: &AppHandle, value: u8, total: u8, step: &str) {
    let _ = app.emit(
        EV_PROGRESS,
        ProgressEvent {
            value,
            total,
            step: step.to_string(),
        },
    );
}
