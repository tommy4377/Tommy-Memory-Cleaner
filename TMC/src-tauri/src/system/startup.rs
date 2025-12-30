use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::config::get_portable_detector;

// FIX #19: Timeout per comandi di sistema (10 secondi)
const SYSTEM_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);

// FIX #19: Helper per eseguire comandi con timeout
fn run_command_with_timeout(mut cmd: std::process::Command) -> Result<std::process::Output> {
    use std::sync::mpsc;
    
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let result = cmd.output();
        let _ = tx.send(result);
    });
    
    match rx.recv_timeout(SYSTEM_COMMAND_TIMEOUT) {
        Ok(result) => {
            if let Err(e) = handle.join() {
                tracing::warn!("Thread panicked during command execution: {:?}", e);
            }
            result.map_err(|e| anyhow::anyhow!("Command execution failed: {}", e))
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            tracing::warn!("Command timed out after {:?}", SYSTEM_COMMAND_TIMEOUT);
            // Nota: Non possiamo fare join qui perché il thread è ancora in esecuzione
            // Il thread continuerà in background ma terminerà naturalmente quando completa
            bail!("Command timed out after {:?}", SYSTEM_COMMAND_TIMEOUT)
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            if let Err(e) = handle.join() {
                tracing::warn!("Thread panicked during command execution (disconnected): {:?}", e);
            }
            bail!("Command thread disconnected")
        }
    }
}

fn exe_path() -> Result<PathBuf> {
    std::env::current_exe().context("cannot resolve current exe path")
}

fn task_name() -> &'static str {
    "TommyMemoryCleanerAutoStart"
}

fn app_name() -> &'static str {
    "Tommy Memory Cleaner"
}

pub fn set_run_on_startup(enable: bool) -> Result<()> {
    let detector = get_portable_detector();
    
    if detector.is_portable() {
        // Versione portable: usa shortcut nella cartella Startup
        set_portable_startup(enable)
    } else {
        // Versione installata: usa registro e/o Task Scheduler
        set_installed_startup(enable)
    }
}

fn set_portable_startup(enable: bool) -> Result<()> {
    let detector = get_portable_detector();
    let exe_path = detector.exe_path();
    
    // Ottieni cartella Startup di Windows
    let startup_folder = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot find user data directory"))?
        .join(r"Microsoft\Windows\Start Menu\Programs\Startup");
    
    let shortcut_path = startup_folder.join("TommyMemoryCleaner.lnk");
    
    if enable {
        // Crea cartella se non esiste
        std::fs::create_dir_all(&startup_folder)?;
        
        // Crea shortcut usando PowerShell con nome e icona corretti
        // Cerca icon.ico nella stessa cartella dell'exe, altrimenti usa l'exe stesso
        let icon_path = if let Some(parent) = exe_path.parent() {
            // Prova prima icon.ico nella stessa cartella
            let ico_path = parent.join("icon.ico");
            if ico_path.exists() {
                ico_path.to_string_lossy().replace('\\', "\\\\")
            } else {
                // Prova icons/icon.ico
                let icons_ico = parent.join("icons").join("icon.ico");
                if icons_ico.exists() {
                    icons_ico.to_string_lossy().replace('\\', "\\\\")
                } else {
                    // Fallback all'exe stesso come icona (contiene già l'icona embedded)
                    exe_path.to_string_lossy().replace('\\', "\\\\")
                }
            }
        } else {
            exe_path.to_string_lossy().replace('\\', "\\\\")
        };
        
        let ps_script = format!(
            r#"
            $WshShell = New-Object -comObject WScript.Shell
            $Shortcut = $WshShell.CreateShortcut("{}")
            $Shortcut.TargetPath = "{}"
            $Shortcut.WorkingDirectory = "{}"
            $Shortcut.IconLocation = "{}, 0"
            $Shortcut.Description = "Tommy Memory Cleaner - Memory Optimization Tool"
            $Shortcut.WindowStyle = 1
            $Shortcut.Save()
            "#,
            shortcut_path.to_string_lossy().replace('\\', "\\\\"),
            exe_path.to_string_lossy().replace('\\', "\\\\"),
            exe_path.parent()
                .ok_or_else(|| anyhow::anyhow!("Executable path has no parent directory"))?
                .to_string_lossy()
                .replace('\\', "\\\\"),
            icon_path
        );
        
        // FIX #19: Usa timeout per il comando PowerShell
        #[cfg(windows)]
        let mut cmd = std::process::Command::new("powershell");
        #[cfg(windows)]
        cmd.arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(&ps_script)
            .creation_flags(0x08000000); // CREATE_NO_WINDOW
        
        #[cfg(not(windows))]
        let mut cmd = std::process::Command::new("powershell");
        #[cfg(not(windows))]
        cmd.arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(&ps_script);
        
        let result = run_command_with_timeout(cmd)?;
            
        if !result.status.success() {
            let error = String::from_utf8_lossy(&result.stderr);
            bail!("Failed to create startup shortcut: {}", error);
        }
        
        // Verifica che il file sia stato creato
        if !shortcut_path.exists() {
            bail!("Failed to create startup shortcut - file not found");
        }
        
    } else {
        // Rimuovi shortcut se esiste
        if shortcut_path.exists() {
            std::fs::remove_file(shortcut_path)?;
        }
    }
    
    Ok(())
}

fn set_installed_startup(enable: bool) -> Result<()> {
    let exe = exe_path()?;
    let exe_str = exe.to_string_lossy();
    
    // Valida il percorso per sicurezza
    if !exe.exists() {
        bail!("Executable path does not exist");
    }
    
    if enable {
        // Prima prova con il registro (non richiede admin)
        if let Ok(()) = set_registry_startup(&exe_str, true) {
            return Ok(());
        }
        
        // Fallback a Task Scheduler
        set_task_scheduler_startup(&exe_str, true)
    } else {
        // Rimuovi da entrambi
        let _ = set_registry_startup(&exe_str, false);
        let _ = set_task_scheduler_startup(&exe_str, false);
        Ok(())
    }
}

fn set_registry_startup(exe_path: &str, enable: bool) -> Result<()> {
    if enable {
        // FIX: Usa percorso assoluto e verifica esistenza
        let exe_path_abs = if std::path::Path::new(exe_path).is_absolute() {
            exe_path.to_string()
        } else {
            std::env::current_exe()?
                .to_string_lossy()
                .to_string()
        };
        
        // Verifica che l'exe esista
        if !std::path::Path::new(&exe_path_abs).exists() {
            bail!("Executable path does not exist: {}", exe_path_abs);
        }
        
        // Usa PowerShell per evitare problemi di encoding
        let ps_script = format!(
            r#"
            try {{
                $exePath = '{}'
                if (-not (Test-Path $exePath)) {{
                    Write-Error "Executable not found: $exePath"
                    exit 1
                }}
                New-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" `
                    -Name "{}" `
                    -Value $exePath `
                    -PropertyType String `
                    -Force `
                    -ErrorAction Stop | Out-Null
                exit 0
            }} catch {{
                Write-Error $_.Exception.Message
                exit 1
            }}
            "#,
            exe_path_abs.replace('\\', "\\\\").replace('\'', "''"),
            app_name()
        );
        
        // FIX #19: Usa timeout per il comando PowerShell
        #[cfg(windows)]
        let mut cmd = std::process::Command::new("powershell");
        #[cfg(windows)]
        cmd.arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(&ps_script)
            .creation_flags(0x08000000);
        
        #[cfg(not(windows))]
        let mut cmd = std::process::Command::new("powershell");
        #[cfg(not(windows))]
        cmd.arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(&ps_script);
        
        let result = run_command_with_timeout(cmd)?;
        
        if !result.status.success() {
            let error = String::from_utf8_lossy(&result.stderr);
            bail!("Failed to set registry startup: {}", error);
        }
    } else {
        let ps_script = format!(
            r#"
            try {{
                Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" `
                    -Name "{}" `
                    -Force `
                    -ErrorAction Stop
                exit 0
            }} catch {{
                # Se la proprietà non esiste, non è un errore critico
                if ($_.Exception.Message -like "*does not exist*") {{
                    exit 0
                }}
                Write-Error $_.Exception.Message
                exit 1
            }}
            "#,
            app_name()
        );
        
        // Usa timeout anche per la rimozione
        #[cfg(windows)]
        let mut cmd = std::process::Command::new("powershell");
        #[cfg(windows)]
        cmd.arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(&ps_script)
            .creation_flags(0x08000000);
            
        #[cfg(not(windows))]
        let mut cmd = std::process::Command::new("powershell");
        #[cfg(not(windows))]
        cmd.arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(&ps_script);
        
        // Non facciamo fail se la rimozione fallisce (la proprietà potrebbe non esistere)
        if let Ok(result) = run_command_with_timeout(cmd) {
            if !result.status.success() {
                let error = String::from_utf8_lossy(&result.stderr);
                tracing::warn!("Failed to remove registry startup (non-critical): {}", error);
            }
        } else {
            tracing::warn!("Failed to execute removal command (non-critical)");
        }
    }
    
    Ok(())
}

fn set_task_scheduler_startup(exe_path: &str, enable: bool) -> Result<()> {
    if enable {
        // FIX: Usa XML per configurazione più robusta del Task Scheduler
        // Questo evita problemi con delay e privilegi
        let xml_content = format!(
            r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Date>2025-01-01T00:00:00</Date>
    <Author>tommy437</Author>
    <Description>Tommy Memory Cleaner - Auto Start on Login</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <Delay>PT30S</Delay>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>7</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>"{}"</Command>
    </Exec>
  </Actions>
</Task>"#,
            exe_path.replace('\\', "\\\\").replace('"', "&quot;")
        );
        
        // Salva XML temporaneo
        let temp_xml = std::env::temp_dir().join("tmc_startup_task.xml");
        std::fs::write(&temp_xml, xml_content)?;
        
        // FIX #19: Usa timeout per il comando schtasks
        #[cfg(windows)]
        let mut cmd = std::process::Command::new("schtasks");
        #[cfg(windows)]
        cmd.args([
                "/Create",
                "/F", // Force overwrite
                "/TN", task_name(),
                "/XML", &temp_xml.to_string_lossy(),
            ])
            .creation_flags(0x08000000);
        
        #[cfg(not(windows))]
        let mut cmd = std::process::Command::new("schtasks");
        #[cfg(not(windows))]
        cmd.args([
                "/Create",
                "/F",
                "/TN", task_name(),
                "/XML", &temp_xml.to_string_lossy(),
            ]);
        
        let result = run_command_with_timeout(cmd)?;
        
        // Rimuovi file temporaneo
        let _ = std::fs::remove_file(&temp_xml);
            
        if !result.status.success() {
            let error = String::from_utf8_lossy(&result.stderr);
            // Fallback a metodo semplice se XML fallisce
            tracing::warn!("XML method failed, trying simple method: {}", error);
            
            // FIX #19: Usa timeout per il comando schtasks (fallback)
            #[cfg(windows)]
            let mut cmd = std::process::Command::new("schtasks");
            #[cfg(windows)]
            cmd.args([
                    "/Create",
                    "/F",
                    "/SC", "ONLOGON",
                    "/TN", task_name(),
                    "/TR", &format!("\"{}\"", exe_path),
                    "/RL", "HIGHEST",
                    "/DELAY", "0000:30",
                ])
                .creation_flags(0x08000000);
            
            #[cfg(not(windows))]
            let mut cmd = std::process::Command::new("schtasks");
            #[cfg(not(windows))]
            cmd.args([
                    "/Create",
                    "/F",
                    "/SC", "ONLOGON",
                    "/TN", task_name(),
                    "/TR", &format!("\"{}\"", exe_path),
                    "/RL", "HIGHEST",
                    "/DELAY", "0000:30",
                ]);
            
            let result = run_command_with_timeout(cmd)?;
                
            if !result.status.success() {
                let error = String::from_utf8_lossy(&result.stderr);
                bail!("Failed to create scheduled task: {}", error);
            }
        }
    } else {
        #[cfg(windows)]
        let _ = std::process::Command::new("schtasks")
            .args(["/Delete", "/F", "/TN", task_name()])
            .creation_flags(0x08000000)
            .output();
            
        #[cfg(not(windows))]
        let _ = std::process::Command::new("schtasks")
            .args(["/Delete", "/F", "/TN", task_name()])
            .output();
    }
    
    Ok(())
}

pub fn is_startup_enabled() -> bool {
    let detector = get_portable_detector();
    
    if detector.is_portable() {
        // Check for shortcut in Startup folder
        if let Some(data_dir) = dirs::data_dir() {
            let shortcut_path = data_dir
                .join(r"Microsoft\Windows\Start Menu\Programs\Startup")
                .join("TommyMemoryCleaner.lnk");
            return shortcut_path.exists();
        }
    } else {
        // Check registry
        #[cfg(windows)]
        {
            let ps_script = format!(
                r#"
                $value = Get-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" `
                    -Name "{}" `
                    -ErrorAction SilentlyContinue
                if ($value) {{ exit 0 }} else {{ exit 1 }}
                "#,
                app_name()
            );
            
            // FIX #19: Usa timeout per il comando PowerShell
            #[cfg(windows)]
            let mut cmd = std::process::Command::new("powershell");
            #[cfg(windows)]
            cmd.arg("-NoProfile")
                .arg("-NonInteractive")
                .arg("-Command")
                .arg(&ps_script)
                .creation_flags(0x08000000);
            
            #[cfg(not(windows))]
            let mut cmd = std::process::Command::new("powershell");
            #[cfg(not(windows))]
            cmd.arg("-NoProfile")
                .arg("-NonInteractive")
                .arg("-Command")
                .arg(&ps_script);
            
            if let Ok(result) = run_command_with_timeout(cmd) {
                if result.status.success() {
                    return true;
                }
            }
            
            // Check Task Scheduler
            // FIX #19: Usa timeout per il comando schtasks
            #[cfg(windows)]
            let mut cmd = std::process::Command::new("schtasks");
            #[cfg(windows)]
            cmd.args(["/Query", "/TN", task_name()])
                .creation_flags(0x08000000);
            
            #[cfg(not(windows))]
            let mut cmd = std::process::Command::new("schtasks");
            #[cfg(not(windows))]
            cmd.args(["/Query", "/TN", task_name()]);
            
            if let Ok(result) = run_command_with_timeout(cmd) {
                return result.status.success();
            }
        }
    }
    
    false
}