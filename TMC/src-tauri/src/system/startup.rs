use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::config::get_portable_detector;

// FIX #19: Timeout for system commands (10 seconds)
const SYSTEM_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);

// FIX #19: Helper to run commands with a timeout
// FIX #13: Use spawn() + wait_with_output() so the zombie process can be killed on timeout
fn run_command_with_timeout(mut cmd: std::process::Command) -> Result<std::process::Output> {
    use std::sync::mpsc;

    // Spawn with piped stdout/stderr so we can get Output later
    let child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn command")?;

    // Record PID before moving child into thread
    let pid = child.id();

    let (tx, rx) = mpsc::channel();

    let handle = std::thread::spawn(move || {
        let result = child.wait_with_output();
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
            // Kill the entire process tree on Windows to prevent zombie processes
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("taskkill")
                    .args(["/F", "/T", "/PID", &pid.to_string()])
                    .creation_flags(0x08000000)
                    .output();
            }
            #[cfg(not(windows))]
            {
                let _ = std::process::Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .output();
            }
            // Join the thread (wait_with_output will return after kill)
            if let Err(e) = handle.join() {
                tracing::warn!("Thread panicked after kill (timeout): {:?}", e);
            }
            bail!("Command timed out after {:?}", SYSTEM_COMMAND_TIMEOUT)
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            // Try to kill on disconnect too
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("taskkill")
                    .args(["/F", "/T", "/PID", &pid.to_string()])
                    .creation_flags(0x08000000)
                    .output();
            }
            #[cfg(not(windows))]
            {
                let _ = std::process::Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .output();
            }
            if let Err(e) = handle.join() {
                tracing::warn!(
                    "Thread panicked during command execution (disconnected): {:?}",
                    e
                );
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

/// Properly escape a string for safe inclusion in XML content.
/// Handles all five XML predefined entities: & < > " '
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&apos;")
}

pub fn set_run_on_startup(enable: bool) -> Result<()> {
    let detector = get_portable_detector();

    if detector.is_portable() {
        // Portable version: use a shortcut in the Startup folder
        set_portable_startup(enable)
    } else {
        // Installed version: use registry and/or Task Scheduler
        set_installed_startup(enable)
    }
}

fn set_portable_startup(enable: bool) -> Result<()> {
    let detector = get_portable_detector();
    let exe_path = detector.exe_path();

    // Get the Windows Startup folder
    let startup_folder = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot find user data directory"))?
        .join(r"Microsoft\Windows\Start Menu\Programs\Startup");

    let shortcut_path = startup_folder.join("TommyMemoryCleaner.lnk");

    if enable {
        // Create the folder if it doesn't exist
        std::fs::create_dir_all(&startup_folder)?;

        // Look for icon.ico in the same folder as the exe, otherwise use the
        // exe itself (it has an embedded icon)
        let icon_path = exe_path
            .parent()
            .map(|parent| {
                let ico_path = parent.join("icon.ico");
                let icons_ico = parent.join("icons").join("icon.ico");
                if ico_path.exists() {
                    ico_path
                } else if icons_ico.exists() {
                    icons_ico
                } else {
                    exe_path.to_path_buf()
                }
            })
            .unwrap_or_else(|| exe_path.to_path_buf());

        // Write the .lnk directly (pure Rust) instead of spawning PowerShell +
        // WScript.Shell COM: no process launch, no AV heuristic exposure
        let mut link = mslnk::ShellLink::new(&exe_path)
            .map_err(|e| anyhow::anyhow!("Failed to build startup shortcut: {:?}", e))?;
        link.set_working_dir(
            exe_path
                .parent()
                .map(|p| p.to_string_lossy().to_string()),
        );
        link.set_icon_location(Some(icon_path.to_string_lossy().to_string()));
        link.set_name(Some(
            "Tommy Memory Cleaner - Memory Optimization Tool".to_string(),
        ));
        link.create_lnk(&shortcut_path)
            .map_err(|e| anyhow::anyhow!("Failed to create startup shortcut: {:?}", e))?;

        // Verify that the file was created
        if !shortcut_path.exists() {
            bail!("Failed to create startup shortcut - file not found");
        }
    } else {
        // Remove the shortcut if it exists
        if shortcut_path.exists() {
            std::fs::remove_file(shortcut_path)?;
        }
    }

    Ok(())
}

fn set_installed_startup(enable: bool) -> Result<()> {
    let exe = exe_path()?;
    let exe_str = exe.to_string_lossy();

    // Validate the path for safety
    if !exe.exists() {
        bail!("Executable path does not exist");
    }

    if enable {
        // Try the registry first (doesn't require admin)
        if let Ok(()) = set_registry_startup(&exe_str, true) {
            return Ok(());
        }

        // Fallback to Task Scheduler
        set_task_scheduler_startup(&exe_str, true)
    } else {
        let reg_result = set_registry_startup(&exe_str, false);
        let task_result = set_task_scheduler_startup(&exe_str, false);
        let elevated_result = crate::system::elevated_task::delete_elevated_task();
        
        // If registry removal succeeded but task scheduler failed, log warning
        if task_result.is_err() || elevated_result.is_err() {
            tracing::warn!(
                "Some startup mechanisms could not be disabled (may require admin): task={:?}, elevated={:?}",
                task_result.err(), elevated_result.err()
            );
        }
        reg_result?;
        Ok(())
    }
}

/// HKCU Run key used for the installed-mode startup entry.
#[cfg(windows)]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

#[cfg(windows)]
fn to_wide(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

/// Set or remove the HKCU Run value directly via the registry API.
/// This used to spawn PowerShell (New-ItemProperty/Remove-ItemProperty), which
/// is a common AV heuristic trigger when launched from an elevated exe.
#[cfg(windows)]
fn set_registry_startup(exe_path: &str, enable: bool) -> Result<()> {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW,
        HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ,
    };

    let key_wide = to_wide(RUN_KEY);
    let name_wide = to_wide(app_name());

    if enable {
        // FIX: Use an absolute path and verify it exists
        let exe_path_abs = if std::path::Path::new(exe_path).is_absolute() {
            exe_path.to_string()
        } else {
            std::env::current_exe()?.to_string_lossy().to_string()
        };

        // Verify that the exe exists
        if !std::path::Path::new(&exe_path_abs).exists() {
            bail!("Executable path does not exist: {}", exe_path_abs);
        }

        // Quote the path, same format the PowerShell version wrote
        let value_wide = to_wide(&format!("\"{}\"", exe_path_abs));

        unsafe {
            let mut hkey: windows_sys::Win32::System::Registry::HKEY = std::ptr::null_mut();
            let open = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                key_wide.as_ptr(),
                0,
                std::ptr::null(),
                0,
                KEY_SET_VALUE,
                std::ptr::null(),
                &mut hkey,
                std::ptr::null_mut(),
            );
            if open != 0 {
                bail!("Failed to open HKCU Run key: error {}", open);
            }
            let set = RegSetValueExW(
                hkey,
                name_wide.as_ptr(),
                0,
                REG_SZ,
                value_wide.as_ptr() as *const u8,
                (value_wide.len() * 2) as u32,
            );
            RegCloseKey(hkey);
            if set != 0 {
                bail!("Failed to set registry startup value: error {}", set);
            }
        }
    } else {
        unsafe {
            let mut hkey: windows_sys::Win32::System::Registry::HKEY = std::ptr::null_mut();
            let open = RegOpenKeyExW(HKEY_CURRENT_USER, key_wide.as_ptr(), 0, KEY_SET_VALUE, &mut hkey);
            if open == 0 {
                const ERROR_FILE_NOT_FOUND: u32 = 2; // value absent — not an error
                let del = RegDeleteValueW(hkey, name_wide.as_ptr());
                RegCloseKey(hkey);
                if del != 0 && del != ERROR_FILE_NOT_FOUND {
                    tracing::warn!(
                        "Failed to remove registry startup (non-critical): error {}",
                        del
                    );
                }
            }
        }
    }

    Ok(())
}

#[cfg(not(windows))]
fn set_registry_startup(_exe_path: &str, _enable: bool) -> Result<()> {
    Ok(())
}

fn set_task_scheduler_startup(exe_path: &str, enable: bool) -> Result<()> {
    if enable {
        // FIX: Use XML for a more robust Task Scheduler configuration
        // This avoids issues with delay and privileges
        let exe_dir = std::path::Path::new(exe_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| std::env::temp_dir().to_string_lossy().to_string());

        let xml_content = format!(
            r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Date>2025-01-01T00:00:00</Date>
    <Author>Tommy Memory Cleaner</Author>
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
      <WorkingDirectory>{}</WorkingDirectory>
    </Exec>
  </Actions>
</Task>"#,
            escape_xml(&exe_path.replace('\\', "\\\\")),
            escape_xml(&exe_dir)
        );

        // Save temporary XML
        let temp_xml = std::env::temp_dir().join("tmc_startup_task.xml");
        std::fs::write(&temp_xml, xml_content)?;

        // FIX #19: Use a timeout for the schtasks command
        #[cfg(windows)]
        let mut cmd = std::process::Command::new("schtasks");
        #[cfg(windows)]
        cmd.args([
            "/Create",
            "/F", // Force overwrite
            "/TN",
            task_name(),
            "/XML",
            &temp_xml.to_string_lossy(),
        ])
        .creation_flags(0x08000000);

        #[cfg(not(windows))]
        let mut cmd = std::process::Command::new("schtasks");
        #[cfg(not(windows))]
        cmd.args([
            "/Create",
            "/F",
            "/TN",
            task_name(),
            "/XML",
            &temp_xml.to_string_lossy(),
        ]);

        let result = run_command_with_timeout(cmd)?;

        // Remove the temporary file
        let _ = std::fs::remove_file(&temp_xml);

        if !result.status.success() {
            let error = String::from_utf8_lossy(&result.stderr);
            // Fallback to the simple method if XML fails
            tracing::warn!("XML method failed, trying simple method: {}", error);

            // FIX #19: Use a timeout for the schtasks command (fallback)
            #[cfg(windows)]
            let mut cmd = std::process::Command::new("schtasks");
            #[cfg(windows)]
            cmd.args([
                "/Create",
                "/F",
                "/SC",
                "ONLOGON",
                "/TN",
                task_name(),
                "/TR",
                &format!("\"{}\"", exe_path),
                "/RL",
                "HIGHEST",
                "/DELAY",
                "0000:30",
            ])
            .creation_flags(0x08000000);

            #[cfg(not(windows))]
            let mut cmd = std::process::Command::new("schtasks");
            #[cfg(not(windows))]
            cmd.args([
                "/Create",
                "/F",
                "/SC",
                "ONLOGON",
                "/TN",
                task_name(),
                "/TR",
                &format!("\"{}\"", exe_path),
                "/RL",
                "HIGHEST",
                "/DELAY",
                "0000:30",
            ]);

            let result = run_command_with_timeout(cmd)?;

            if !result.status.success() {
                let error = String::from_utf8_lossy(&result.stderr);
                bail!("Failed to create scheduled task: {}", error);
            }
        }
    } else {
        // FIX #13: Wrap with a timeout to prevent a zombie process on schtasks /Delete
        #[cfg(windows)]
        {
            let mut cmd = std::process::Command::new("schtasks");
            cmd.args(["/Delete", "/F", "/TN", task_name()])
                .creation_flags(0x08000000);
            let _ = run_command_with_timeout(cmd);
        }

        #[cfg(not(windows))]
        {
            let mut cmd = std::process::Command::new("schtasks");
            cmd.args(["/Delete", "/F", "/TN", task_name()]);
            let _ = run_command_with_timeout(cmd);
        }
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
            use windows_sys::Win32::System::Registry::{
                RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, KEY_QUERY_VALUE,
            };

            // Direct registry query (previously spawned PowerShell Get-ItemProperty)
            let key_wide = to_wide(RUN_KEY);
            let name_wide = to_wide(app_name());
            unsafe {
                let mut hkey: windows_sys::Win32::System::Registry::HKEY = std::ptr::null_mut();
                if RegOpenKeyExW(HKEY_CURRENT_USER, key_wide.as_ptr(), 0, KEY_QUERY_VALUE, &mut hkey)
                    == 0
                {
                    // Null data pointers: we only care whether the value exists
                    let query = RegQueryValueExW(
                        hkey,
                        name_wide.as_ptr(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    );
                    RegCloseKey(hkey);
                    if query == 0 {
                        return true;
                    }
                }
            }

            // Check Task Scheduler
            // FIX #19: Use a timeout for the schtasks command
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
