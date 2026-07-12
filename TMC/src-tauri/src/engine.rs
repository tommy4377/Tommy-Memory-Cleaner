/// Memory optimization engine
///
/// This module contains the core engine responsible for performing
/// memory optimization operations on Windows systems.
use crate::config::Config;
use crate::logging::event_viewer::{log_error_event, log_optimization_event};
use crate::memory::ops::{
    memory_info, optimize_combined_page_list, optimize_modified_page_list_with_stealth, optimize_registry_cache,
    optimize_standby_list_with_stealth, optimize_system_file_cache, optimize_working_set_with_stealth,
};
use crate::memory::advanced::trim_memory_compression_store;
use crate::memory::types::{Areas, MemoryInfo, Reason};
use crate::os;
use serde::{Deserialize, Serialize};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

/// Result of optimizing a specific memory area
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeAreaResult {
    pub name: String,
    pub duration_ms: u128,
    pub error: Option<String>,
}

/// Complete optimization result with all areas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeResult {
    pub reason: Reason,
    pub duration_ms: u128,
    pub freed_physical_bytes: i64,
    pub freed_commit_bytes: i64,
    pub areas: Vec<OptimizeAreaResult>,
}

/// Main memory optimization engine
#[derive(Clone)]
pub struct Engine {
    pub cfg: Arc<Mutex<Config>>,
}

impl Engine {
    /// Create a new engine instance with configuration
    pub fn new(cfg: Arc<Mutex<Config>>) -> Self {
        Self { cfg }
    }

    /// Get current memory information
    pub fn memory(&self) -> anyhow::Result<MemoryInfo> {
        memory_info().map_err(|e| e.into())
    }

    /// Perform memory optimization on specified areas
    ///
    /// This is the main optimization method that:
    /// - Acquires necessary privileges before starting
    /// - Optimizes each specified memory area
    /// - Reports progress through callback
    /// - Returns detailed results
    pub fn optimize<F>(
        &self,
        reason: Reason,
        areas: Areas,
        mut progress: Option<F>,
    ) -> anyhow::Result<OptimizeResult>
    where
        F: FnMut(u8, u8, String),
    {
        // Pre-acquire all necessary privileges BEFORE starting
        tracing::info!(
            "Starting optimization with reason: {:?}, areas: {:?}",
            reason,
            areas
        );

        // Check if we should use indirect syscalls for advanced memory areas
        // These areas benefit from stealth: Combined Page List, Modified Page List, Standby List
        let use_indirect_syscalls = areas.intersects(
            Areas::COMBINED_PAGE_LIST | Areas::MODIFIED_PAGE_LIST | Areas::STANDBY_LIST
        );
        
        tracing::debug!("use_indirect_syscalls = {}", use_indirect_syscalls);
        
        if use_indirect_syscalls {
            tracing::info!("Advanced memory areas detected - using indirect syscalls for stealth");
        }

        // Acquire privileges in advance for all areas with retry
        let mut required_privs = vec![];
        if areas.contains(Areas::WORKING_SET) {
            required_privs.push("SeDebugPrivilege");
        }
        if areas.contains(Areas::SYSTEM_FILE_CACHE) {
            required_privs.push("SeIncreaseQuotaPrivilege");
        }
        if areas.intersects(
            Areas::MODIFIED_PAGE_LIST
                | Areas::STANDBY_LIST
                | Areas::STANDBY_LIST_LOW
                | Areas::COMBINED_PAGE_LIST,
        ) {
            required_privs.push("SeProfileSingleProcessPrivilege");
        }

        // Deduplicate and acquire privileges with retry logic
        required_privs.sort();
        required_privs.dedup();

        let mut acquired_privs = 0;
        for priv_name in &required_privs {
            // Retry up to 3 times for each privilege
            let mut success = false;
            for attempt in 1..=3 {
                match crate::memory::privileges::ensure_privilege(priv_name) {
                    Ok(_) => {
                        tracing::info!("✓ Acquired privilege {} (attempt {})", priv_name, attempt);
                        acquired_privs += 1;
                        success = true;
                        break;
                    }
                    Err(e) => {
                        if attempt < 3 {
                            tracing::warn!(
                                "Failed to acquire {} (attempt {}): {}, retrying...",
                                priv_name,
                                attempt,
                                e
                            );
                            // TODO: Use tokio::time::sleep when optimize() is made async
                            std::thread::sleep(std::time::Duration::from_millis(
                                50 * attempt as u64,
                        ));
                        } else {
                            let error_msg = format!(
                                "Failed to acquire privilege {} after 3 attempts: {}",
                                priv_name, e
                            );
                            tracing::warn!("✗ {}", error_msg);
                            log_error_event(&error_msg);
                        }
                    }
                }
            }

            if !success {
                tracing::warn!("Warning: Continuing without privilege {}", priv_name);
            }
        }

        tracing::info!(
            "Acquired {}/{} required privileges",
            acquired_privs,
            required_privs.len()
        );

        // Validate the areas available for this Windows version
        let mut validated_areas = Areas::empty();
        if areas.contains(Areas::STANDBY_LIST) && os::has_standby_list() {
            validated_areas |= Areas::STANDBY_LIST;
        }
        if areas.contains(Areas::STANDBY_LIST_LOW) && os::has_standby_list_low() {
            validated_areas |= Areas::STANDBY_LIST_LOW;
        }
        if areas.contains(Areas::MODIFIED_PAGE_LIST) && os::has_modified_page_list() {
            validated_areas |= Areas::MODIFIED_PAGE_LIST;
        }
        if areas.contains(Areas::REGISTRY_CACHE) && os::has_registry_cache() {
            validated_areas |= Areas::REGISTRY_CACHE;
        }
        if areas.contains(Areas::SYSTEM_FILE_CACHE) && os::has_system_file_cache() {
            validated_areas |= Areas::SYSTEM_FILE_CACHE;
        }
        if areas.contains(Areas::COMBINED_PAGE_LIST) && os::has_combined_page_list() {
            validated_areas |= Areas::COMBINED_PAGE_LIST;
        }
        if areas.contains(Areas::WORKING_SET) && os::has_working_set() {
            validated_areas |= Areas::WORKING_SET;
        }
        if areas.contains(Areas::MODIFIED_FILE_CACHE) && os::has_modified_file_cache() {
            validated_areas |= Areas::MODIFIED_FILE_CACHE;
        }

        if validated_areas != areas {
            let missing = areas & !validated_areas;
            tracing::warn!(
                "Some memory areas are not available on this Windows version: {:?}",
                missing
            );
        }

        let areas = validated_areas;

        // Brief stabilization delay before optimization
        // TODO: Use tokio::time::sleep when optimize() is made async
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Get memory usage BEFORE optimization
        let before = self.memory()?;

        let mut area_operations = Vec::new();
        let mut area_names = Vec::new();
        let mut successful_areas = 0;

        // Build the list of operations
        // Order operations for optimal chaining:
        // 1. ModifiedFileCache first (flushes disk cache)
        // 2. ModifiedPageList second (needs flushed data)
        // 3. SystemFileCache (limits cache size)
        // 4. Other operations
        if areas.contains(Areas::MODIFIED_FILE_CACHE) {
            area_operations.push(("ModifiedFileCache", "Modified File Cache"));
        }
        if areas.contains(Areas::MODIFIED_PAGE_LIST) {
            area_operations.push(("ModifiedPageList", "Modified Page List"));
        }
        if areas.contains(Areas::SYSTEM_FILE_CACHE) {
            area_operations.push(("SystemFileCache", "System File Cache"));
        }
        if areas.contains(Areas::WORKING_SET) {
            area_operations.push(("WorkingSet", "Working Set"));
        }
        if areas.contains(Areas::STANDBY_LIST) {
            area_operations.push(("StandbyList", "Standby List"));
        }
        // FIX: Add STANDBY_LIST_LOW even if STANDBY_LIST is present
        // These are two distinct, complementary optimizations
        if areas.contains(Areas::STANDBY_LIST_LOW) {
            area_operations.push(("StandbyListLowPriority", "Standby List (Low Priority)"));
        }
        if areas.contains(Areas::COMBINED_PAGE_LIST) {
            area_operations.push(("CombinedPageList", "Combined Page List"));
        }
        if areas.contains(Areas::REGISTRY_CACHE) {
            area_operations.push(("RegistryCache", "Registry Cache"));
        }

        // Validation to avoid overflow: len() could be > 255
        let total = area_operations
            .len()
            .try_into()
            .ok()
            .and_then(|n: u8| n.checked_add(1))
            .unwrap_or(u8::MAX);
        let mut idx: u8 = 0;
        let mut results = Vec::with_capacity(area_operations.len());
        let mut errors = Vec::new();
        let start_all = Instant::now();

        // FIX #10: Timeout for optimization operations (30 seconds per operation)
        const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

        // Run the optimizations
        for (operation_name, display_name) in &area_operations {
            idx = idx.saturating_add(1);
            area_names.push(display_name.to_string());

            if let Some(cb) = progress.as_mut() {
                cb(idx, total, display_name.to_string());
            }

            // FIX: Increase the delay between operations on the first run
            if idx > 1 {
                // Brief delay between operations to avoid overwhelming the system
                // TODO: Use tokio::time::sleep when optimize() is made async
                std::thread::sleep(std::time::Duration::from_millis(30));
            }

            let t0 = Instant::now();

            // FIX #10: Run the operation with a timeout using a separate thread
            let operation_name_clone = operation_name.to_string();
            let cfg_clone = self.cfg.clone();
            let use_indirect_syscalls_clone = use_indirect_syscalls;

            let (tx, rx) = mpsc::channel();
            let handle = std::thread::spawn(move || {
                // Recreate the engine to run the operation
                let engine = Engine { cfg: cfg_clone };
                let result = engine.execute_optimization(&operation_name_clone, use_indirect_syscalls_clone);
                let _ = tx.send(result);
            });

            // Wait for the result with a timeout
            let res = match rx.recv_timeout(OPERATION_TIMEOUT) {
                Ok(result) => {
                    // Wait for the thread to finish (should already be done)
                    if let Err(e) = handle.join() {
                        tracing::warn!(
                            "Thread panicked during operation {}: {:?}",
                            display_name,
                            e
                        );
                    }
                    result
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    tracing::warn!(
                        "Operation {} timed out after {:?}",
                        display_name,
                        OPERATION_TIMEOUT
                    );
                    // The thread might still be running, but we cannot wait for it indefinitely.
                    // Note: we cannot join here because the thread is still running and could block us.
                    // It will keep running in the background and terminate naturally once the operation completes.
                    Err(anyhow::anyhow!(
                        "Operation timed out after {:?}",
                        OPERATION_TIMEOUT
                    ))
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // The thread crashed or was terminated
                    if let Err(e) = handle.join() {
                        tracing::warn!(
                            "Thread panicked during operation {} (disconnected): {:?}",
                            display_name,
                            e
                        );
                    }
                    Err(anyhow::anyhow!("Operation thread disconnected"))
                }
            };

            let dur = t0.elapsed().as_millis();

            match res {
                Ok(_) => {
                    successful_areas += 1;
                    results.push(OptimizeAreaResult {
                        name: display_name.to_string(),
                        duration_ms: dur,
                        error: None,
                    });
                    tracing::debug!("Successfully optimized: {} in {}ms", display_name, dur);
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    tracing::warn!("Area {} optimization warning: {}", display_name, error_msg);

                    results.push(OptimizeAreaResult {
                        name: display_name.to_string(),
                        duration_ms: dur,
                        error: Some(error_msg.clone()),
                    });

                    if *operation_name == "WorkingSet" || *operation_name == "SystemFileCache" {
                        errors.push(format!("{}: {}", display_name, error_msg));
                    }
                }
            }
        }

        // Notifica completamento
        if let Some(cb) = progress.as_mut() {
            cb(total, total, "Completed".to_string());
        }

        // Brief stabilization delay after optimization for memory measurement
        // TODO: Use tokio::time::sleep when optimize() is made async
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Get memory usage AFTER, with retry and validation
        let mut after = self.memory()?;
        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 3;

        // FIX: If there is no significant difference, retry with progressive delays
        loop {
            // FIX #12: Use saturating_sub here too for consistency
            let freed = (after.physical.free.bytes as i64)
                .saturating_sub(before.physical.free.bytes as i64);

            // Exit once at least 1MB was freed or all retries have been exhausted
            if freed.abs() >= 1024 * 1024 || retry_count >= MAX_RETRIES {
                if retry_count > 0 {
                    tracing::info!(
                        "Memory measurement stabilized after {} retries",
                        retry_count
                    );
                }
                break;
            }

            retry_count += 1;
            tracing::debug!(
                "Memory change too small ({} bytes), retrying measurement (attempt {})",
                freed,
                retry_count
            );
            // TODO: Use tokio::time::sleep when optimize() is made async
            std::thread::sleep(std::time::Duration::from_millis(100 * retry_count as u64));
            after = self.memory()?;
        }

        // FIX #16: Use saturating_sub to avoid overflow/underflow issues.
        // Also validate that values are within a safe range before casting, to avoid overflow.
        // i64::MAX is ~9 exabytes, so we clamp to 8 exabytes for safety.
        const MAX_SAFE_BYTES: u64 = 8 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024; // 8 EiB

        let after_phys_safe = after.physical.free.bytes.min(MAX_SAFE_BYTES);
        let before_phys_safe = before.physical.free.bytes.min(MAX_SAFE_BYTES);
        let after_commit_safe = after.commit.free.bytes.min(MAX_SAFE_BYTES);
        let before_commit_safe = before.commit.free.bytes.min(MAX_SAFE_BYTES);

        // If the values are very large, log a warning but continue
        if after.physical.free.bytes > MAX_SAFE_BYTES || before.physical.free.bytes > MAX_SAFE_BYTES
        {
            tracing::warn!(
                "Memory values exceed safe range ({} bytes), clamping for calculation",
                MAX_SAFE_BYTES
            );
        }

        // Safe cast after clamping
        let freed_phys = (after_phys_safe as i64).saturating_sub(before_phys_safe as i64);
        let freed_commit = (after_commit_safe as i64).saturating_sub(before_commit_safe as i64);
        let duration = start_all.elapsed().as_millis();

        // FIX: Validate results to avoid reporting fake optimizations
        let freed_phys_mb = freed_phys as f64 / 1024.0 / 1024.0;
        let freed_commit_mb = freed_commit as f64 / 1024.0 / 1024.0;

        // Verify that at least one area was optimized successfully
        let has_successful_area = results.iter().any(|r| r.error.is_none());

        // If no memory was freed AND no areas succeeded, this might indicate a problem
        if freed_phys.abs() < 1024 * 1024 && !has_successful_area && successful_areas == 0 {
            tracing::warn!("Optimization may have failed: no memory freed and no successful areas");
        }

        tracing::info!(
        "Optimization completed: freed {:.2} MB physical, {:.2} MB commit in {}ms ({} successful areas)",
        freed_phys_mb,
        freed_commit_mb,
        duration,
        successful_areas
    );

        // Log to the Event Viewer only if significant memory was freed or some areas succeeded
        if freed_phys.abs() > 1024 * 1024 || has_successful_area {
            let freed_mb = freed_phys as f64 / 1024.0 / 1024.0;
            let profile_name = self
                .cfg
                .lock()
                .map(|c| format!("{:?}", c.profile))
                .unwrap_or_else(|_| "Unknown".to_string());

            let mode = match reason {
                Reason::Manual => "Manual",
                Reason::Schedule => "Scheduled",
                Reason::LowMemory => "Low Memory Auto",
                Reason::Hotkey => "Hotkey",
            };

            log_optimization_event(
                freed_mb.abs(),
                &profile_name,
                mode,
                &area_names.join(", "),
                duration,
                &errors,
            );
        }

        Ok(OptimizeResult {
            reason,
            duration_ms: duration,
            freed_physical_bytes: freed_phys,
            freed_commit_bytes: freed_commit,
            areas: results,
        })
    }

    fn execute_optimization(&self, operation_name: &str, use_indirect_syscalls: bool) -> anyhow::Result<()> {
        match operation_name {
            "WorkingSet" => {
                let excl = self
                    .cfg
                    .lock()
                    .map(|c| c.process_exclusion_list_lower())
                    .unwrap_or_default();
                
                // Use stealth mode for Working Set when indirect syscalls are enabled
                if use_indirect_syscalls {
                    tracing::debug!("Using stealth mode for Working Set optimization");
                }
                
                optimize_working_set_with_stealth(&excl, use_indirect_syscalls)
            }
            "SystemFileCache" => {
                // System cache optimization
                optimize_system_file_cache()
            }
            "ModifiedPageList" => {
                // Use the optimized modified page list function with stealth support
                optimize_modified_page_list_with_stealth(use_indirect_syscalls)
            }
            "StandbyList" => {
                optimize_standby_list_with_stealth(false, use_indirect_syscalls)
            }
            "StandbyListLowPriority" => optimize_standby_list_with_stealth(true, use_indirect_syscalls),
            "CombinedPageList" => optimize_combined_page_list(),
            "RegistryCache" => optimize_registry_cache(),
            "ModifiedFileCache" => {
                // Always trim memory compression store
                tracing::warn!("Using memory compression store trim");
                let _ = trim_memory_compression_store();
                crate::memory::volumes::flush_modified_file_cache_all()
            }
            _ => {
                tracing::warn!("Unknown optimization operation: {}", operation_name);
                Ok(())
            }
        }
    }
}
