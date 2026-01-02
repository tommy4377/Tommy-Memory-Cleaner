// src-tauri/src/antivirus/whitelist.rs

use anyhow;

// Dichiara l'app come legittima a Windows Defender
pub fn register_as_trusted() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        // Ottieni il percorso dell'eseguibile
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_str) = exe_path.to_str() {
                // Prova a registrare l'app come trusted usando Windows Defender APIs
                // Nota: Questo richiede privilegi amministratore per funzionare completamente
                // Per ora, solo log per debug - l'implementazione completa richiederebbe
                // l'uso di Windows Defender Management APIs che sono complesse
                tracing::debug!("Attempting to register as trusted: {}", exe_str);

                // In futuro, qui si potrebbe implementare:
                // - Aggiunta all'exclusion list di Windows Defender tramite PowerShell:
                //   Add-MpPreference -ExclusionPath "path"
                // - Registrazione come trusted publisher
                // - Whitelisting tramite Group Policy (richiede admin)
            }
        }
    }

    Ok(())
}

// Usa un approccio più "soft" per le operazioni di memoria
pub fn safe_memory_operation<F, R>(operation: F) -> Result<R, anyhow::Error>
where
    F: FnOnce() -> Result<R, anyhow::Error>,
{
    // Aggiungi delay randomico per evitare pattern detection
    use rand::Rng;
    use std::time::Duration;

    let mut rng = rand::thread_rng();
    let delay = Duration::from_millis(rng.gen_range(10..100));
    std::thread::sleep(delay);

    operation()
}
