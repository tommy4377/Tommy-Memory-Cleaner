use crate::config::Config;
use crate::logging::event_viewer::{log_error_event, log_optimization_event};
use crate::memory::ops::{
    memory_info, optimize_combined_page_list, optimize_modified_page_list, optimize_registry_cache,
    optimize_standby_list, optimize_system_file_cache, optimize_working_set,
};
use crate::memory::types::{Areas, MemoryInfo, Reason};
use crate::os;
use serde::{Deserialize, Serialize};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeAreaResult {
    pub name: String,
    pub duration_ms: u128,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeResult {
    pub reason: Reason,
    pub duration_ms: u128,
    pub freed_physical_bytes: i64,
    pub freed_commit_bytes: i64,
    pub areas: Vec<OptimizeAreaResult>,
}

#[derive(Clone)]
pub struct Engine {
    pub cfg: Arc<Mutex<Config>>,
}

impl Engine {
    pub fn new(cfg: Arc<Mutex<Config>>) -> Self {
        Self { cfg }
    }

    pub fn memory(&self) -> anyhow::Result<MemoryInfo> {
        memory_info().map_err(|e| e.into())
    }

    pub fn optimize<F>(
        &self,
        reason: Reason,
        areas: Areas,
        mut progress: Option<F>,
    ) -> anyhow::Result<OptimizeResult>
    where
        F: FnMut(u8, u8, String),
    {
        // FIX: Pre-acquisisci tutti i privilegi necessari PRIMA di iniziare
        tracing::info!(
            "Starting optimization with reason: {:?}, areas: {:?}",
            reason,
            areas
        );

        // Acquisisci privilegi in anticipo per tutte le aree con retry
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

        // Deduplica e acquisisci privilegi con retry logic
        required_privs.sort();
        required_privs.dedup();

        let mut acquired_privs = 0;
        for priv_name in &required_privs {
            // Retry fino a 3 volte per ogni privilegio
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
                            std::thread::sleep(std::time::Duration::from_millis(
                                100 * attempt as u64,
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

        // Valida le aree disponibili per questa versione di Windows
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

        // FIX: Aggiungi un delay iniziale più lungo per stabilizzare il sistema
        // Questo è particolarmente importante al primo avvio
        std::thread::sleep(std::time::Duration::from_millis(300));

        // Ottieni memoria PRIMA dell'ottimizzazione
        let before = self.memory()?;

        let mut area_operations = Vec::new();
        let mut area_names = Vec::new();
        let mut successful_areas = 0;

        // Costruisci lista operazioni
        if areas.contains(Areas::WORKING_SET) {
            area_operations.push(("WorkingSet", "Working Set"));
        }
        if areas.contains(Areas::MODIFIED_PAGE_LIST) {
            area_operations.push(("ModifiedPageList", "Modified Page List"));
        }
        if areas.contains(Areas::STANDBY_LIST) {
            area_operations.push(("StandbyList", "Standby List"));
        }
        // FIX: Aggiungi STANDBY_LIST_LOW anche se STANDBY_LIST è presente
        // Sono due ottimizzazioni diverse e complementari
        if areas.contains(Areas::STANDBY_LIST_LOW) {
            area_operations.push(("StandbyListLowPriority", "Standby List (Low Priority)"));
        }
        if areas.contains(Areas::SYSTEM_FILE_CACHE) {
            area_operations.push(("SystemFileCache", "System File Cache"));
        }
        if areas.contains(Areas::COMBINED_PAGE_LIST) {
            area_operations.push(("CombinedPageList", "Combined Page List"));
        }
        if areas.contains(Areas::MODIFIED_FILE_CACHE) {
            area_operations.push(("ModifiedFileCache", "Modified File Cache"));
        }
        if areas.contains(Areas::REGISTRY_CACHE) {
            area_operations.push(("RegistryCache", "Registry Cache"));
        }

        // Validazione per evitare overflow: len() potrebbe essere > 255
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

        // FIX #10: Timeout per operazioni di ottimizzazione (30 secondi per operazione)
        const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

        // Esegui ottimizzazioni
        for (operation_name, display_name) in &area_operations {
            idx = idx.saturating_add(1);
            area_names.push(display_name.to_string());

            if let Some(cb) = progress.as_mut() {
                cb(idx, total, display_name.to_string());
            }

            // FIX: Aumenta il delay tra operazioni per il primo run
            if idx > 1 {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            let t0 = Instant::now();

            // FIX #10: Esegui l'operazione con timeout usando un thread separato
            let operation_name_clone = operation_name.to_string();
            let cfg_clone = self.cfg.clone();

            let (tx, rx) = mpsc::channel();
            let handle = std::thread::spawn(move || {
                // Ricrea l'engine per eseguire l'operazione
                let engine = Engine { cfg: cfg_clone };
                let result = engine.execute_optimization(&operation_name_clone);
                let _ = tx.send(result);
            });

            // Attendi il risultato con timeout
            let res = match rx.recv_timeout(OPERATION_TIMEOUT) {
                Ok(result) => {
                    // Aspetta che il thread finisca (dovrebbe essere già finito)
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
                    // Il thread potrebbe ancora essere in esecuzione, ma non possiamo aspettarlo indefinitamente
                    // Nota: Non possiamo fare join qui perché il thread è ancora in esecuzione e potrebbe bloccarci
                    // Il thread continuerà in background ma terminerà naturalmente quando completa l'operazione
                    Err(anyhow::anyhow!(
                        "Operation timed out after {:?}",
                        OPERATION_TIMEOUT
                    ))
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // Il thread è crashato o è stato terminato
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

        // FIX: Aumenta il delay di stabilizzazione dopo l'ottimizzazione
        std::thread::sleep(std::time::Duration::from_millis(800));

        // Ottieni memoria DOPO con retry e validazione
        let mut after = self.memory()?;
        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 3;

        // FIX: Se non c'è differenza significativa, riprova con delay progressivi
        loop {
            // FIX #12: Usa saturating_sub anche qui per coerenza
            let freed = (after.physical.free.bytes as i64)
                .saturating_sub(before.physical.free.bytes as i64);

            // Se abbiamo liberato almeno 1MB o abbiamo fatto tutti i retry, usciamo
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
            std::thread::sleep(std::time::Duration::from_millis(500 * retry_count as u64));
            after = self.memory()?;
        }

        // FIX #16: Usa saturating_sub per evitare problemi con overflow/underflow
        // Inoltre, valida che i valori siano in un range sicuro prima del cast per evitare overflow
        // i64::MAX è ~9 exabytes, quindi limitiamo a 8 exabytes per sicurezza
        const MAX_SAFE_BYTES: u64 = 8 * 1024 * 1024 * 1024 * 1024 * 1024 * 1024; // 8 EiB

        let after_phys_safe = after.physical.free.bytes.min(MAX_SAFE_BYTES);
        let before_phys_safe = before.physical.free.bytes.min(MAX_SAFE_BYTES);
        let after_commit_safe = after.commit.free.bytes.min(MAX_SAFE_BYTES);
        let before_commit_safe = before.commit.free.bytes.min(MAX_SAFE_BYTES);

        // Se i valori sono molto grandi, logga un warning ma continua
        if after.physical.free.bytes > MAX_SAFE_BYTES || before.physical.free.bytes > MAX_SAFE_BYTES
        {
            tracing::warn!(
                "Memory values exceed safe range ({} bytes), clamping for calculation",
                MAX_SAFE_BYTES
            );
        }

        // Cast sicuro dopo il clamping
        let freed_phys = (after_phys_safe as i64).saturating_sub(before_phys_safe as i64);
        let freed_commit = (after_commit_safe as i64).saturating_sub(before_commit_safe as i64);
        let duration = start_all.elapsed().as_millis();

        // FIX: Validazione risultati per evitare ottimizzazioni fake
        let freed_phys_mb = freed_phys as f64 / 1024.0 / 1024.0;
        let freed_commit_mb = freed_commit as f64 / 1024.0 / 1024.0;

        // Verifica che almeno una area sia stata ottimizzata con successo
        let has_successful_area = results.iter().any(|r| r.error.is_none());

        // Se non abbiamo liberato memoria E non abbiamo aree di successo, potrebbe essere un problema
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

        // Log nell'Event Viewer solo se abbiamo liberato memoria significativa o abbiamo aree di successo
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

    fn execute_optimization(&self, operation_name: &str) -> anyhow::Result<()> {
        match operation_name {
            "WorkingSet" => {
                let excl = self
                    .cfg
                    .lock()
                    .map(|c| c.process_exclusion_list_lower())
                    .unwrap_or_default();
                optimize_working_set(&excl)
            }
            "SystemFileCache" => optimize_system_file_cache(),
            "ModifiedPageList" => optimize_modified_page_list(),
            "StandbyList" => optimize_standby_list(false),
            "StandbyListLowPriority" => optimize_standby_list(true),
            "CombinedPageList" => optimize_combined_page_list(),
            "RegistryCache" => optimize_registry_cache(),
            "ModifiedFileCache" => crate::memory::volumes::flush_modified_file_cache_all(),
            _ => {
                tracing::warn!("Unknown optimization operation: {}", operation_name);
                Ok(())
            }
        }
    }
}
