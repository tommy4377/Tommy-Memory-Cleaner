pub mod event_viewer;

use std::sync::Once;
use tracing_subscriber::prelude::*;

static INIT: Once = Once::new();

pub fn init() {
    INIT.call_once(|| {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_ansi(cfg!(debug_assertions));
        
        let subscriber = tracing_subscriber::registry()
            .with(fmt_layer);
        
        let _ = tracing::subscriber::set_global_default(subscriber);
        
        tracing::info!("TMC logging initialized");
        
        // Log startup nell'Event Viewer (se possibile) - in modo sicuro
        // FIX: Non crashare se il logging degli eventi fallisce
        std::panic::catch_unwind(|| {
            event_viewer::log_startup_event(
                env!("CARGO_PKG_VERSION"),
                true
            );
        }).unwrap_or_else(|_| {
            tracing::debug!("Event viewer logging failed (non-critical)");
        });
    });
}

pub fn shutdown() {
    // FIX: Non crashare se il logging degli eventi fallisce
    std::panic::catch_unwind(|| {
        event_viewer::log_shutdown_event();
    }).unwrap_or_else(|_| {
        tracing::debug!("Event viewer shutdown logging failed (non-critical)");
    });
}