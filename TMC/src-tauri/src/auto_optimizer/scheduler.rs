use crate::config::Config;
use crate::engine::Engine;
use crate::memory::types::Reason;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::AppHandle;

/// Start the auto-optimizer background task
/// This function spawns an async task that periodically checks for:
/// - Scheduled optimizations (time-based)
/// - Low memory conditions (threshold-based)
pub fn start_auto_optimizer(app: AppHandle, engine: Engine, cfg: Arc<Mutex<Config>>) {
    tauri::async_runtime::spawn(async move {
        let mut last_scheduled_opt = Instant::now();
        let mut last_low_mem_opt = Instant::now();
        let mut check_interval = Duration::from_secs(30);

        // Wait before starting checks
        tokio::time::sleep(Duration::from_secs(10)).await;

        loop {
            tokio::time::sleep(check_interval).await;

            let conf = match cfg.lock() {
                Ok(c) => c.clone(),
                Err(_) => continue,
            };

            let mut action_taken = false;

            // SCHEDULED OPTIMIZATION
            if conf.auto_opt_interval_hours > 0 {
                let hours_passed = last_scheduled_opt.elapsed().as_secs() / 3600;
                if hours_passed >= conf.auto_opt_interval_hours as u64 {
                    tracing::info!(
                        "Triggering scheduled optimization after {} hours",
                        hours_passed
                    );

                    // Log evento automatico
                    crate::logging::event_viewer::log_auto_optimization_event(
                        "Scheduled",
                        conf.auto_opt_interval_hours as u8,
                    );

                    let app_clone = app.clone();
                    let engine_clone = engine.clone();
                    let cfg_clone = cfg.clone();

                    tauri::async_runtime::spawn(async move {
                        // FIX: Use with_progress: true to update the UI during automatic optimizations
                        // This prevents overlaps and correctly shows the status
                        crate::perform_optimization(
                            app_clone,
                            engine_clone,
                            cfg_clone,
                            Reason::Schedule,
                            true,
                            None,
                        )
                        .await;
                    });

                    last_scheduled_opt = Instant::now();
                    action_taken = true;
                }
            }

            // LOW MEMORY OPTIMIZATION (bug fix)
            if conf.auto_opt_free_threshold > 0 && !action_taken {
                // Check memory status
                if let Ok(mem) = engine.memory() {
                    let free_percent = mem.physical.free.percentage;

                    // FIX: Correctly compare with threshold
                    if free_percent < conf.auto_opt_free_threshold {
                        // Verify 5-minute cooldown
                        if last_low_mem_opt.elapsed() >= Duration::from_secs(300) {
                            tracing::info!(
                                "Triggering low memory optimization: {}% free < {}% threshold",
                                free_percent,
                                conf.auto_opt_free_threshold
                            );

                            // Log automatic event
                            crate::logging::event_viewer::log_auto_optimization_event(
                                "Low Memory",
                                conf.auto_opt_free_threshold,
                            );

                            let app_clone = app.clone();
                            let engine_clone = engine.clone();
                            let cfg_clone = cfg.clone();

                            tauri::async_runtime::spawn(async move {
                                // FIX: Use with_progress: true to update UI during automatic optimizations
                                // This prevents overlaps and correctly shows status
                                crate::perform_optimization(
                                    app_clone,
                                    engine_clone,
                                    cfg_clone,
                                    Reason::LowMemory,
                                    true,
                                    None,
                                )
                                .await;
                            });

                            last_low_mem_opt = Instant::now();
                            action_taken = true;
                        } else {
                            let remaining = 300 - last_low_mem_opt.elapsed().as_secs();
                            tracing::debug!(
                                "Low memory detected ({}% free) but cooldown active ({}s remaining)",
                                free_percent, remaining
                            );
                        }

                        // Increase check frequency when memory is low
                        check_interval = Duration::from_secs(30);
                    } else {
                        // Memory OK, reduce check frequency
                        check_interval = Duration::from_secs(60);
                    }
                }
            }

            // Adaptive interval
            if !action_taken {
                check_interval =
                    (check_interval + Duration::from_secs(10)).min(Duration::from_secs(120));
            } else {
                check_interval = Duration::from_secs(30);
            }
        }
    });
}
