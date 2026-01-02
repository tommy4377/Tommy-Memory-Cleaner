/// Command-line argument parser and console mode executor.
///
/// This module handles parsing of command-line arguments for memory optimization
/// and executes the optimization in console mode without GUI. It supports both
/// individual memory area selection and predefined profiles.
use crate::config::{Config, Profile};
use crate::engine::Engine;
use crate::memory::types::{Areas, Reason};
use std::sync::{Arc, Mutex};

#[cfg(not(windows))]
use std::io;

/// Runs the application in console mode with command-line arguments.
///
/// Parses the provided arguments to determine which memory areas to optimize
/// or which profile to use, then executes the optimization and reports results.
///
/// # Arguments
///
/// * `args` - Slice of command-line arguments
pub fn run_console_mode(args: &[String]) {
    // Global function to write to console on Windows
    #[cfg(windows)]
    fn console_print(text: &str) {
        unsafe {
            use std::ptr;
            use std::sync::atomic::{AtomicPtr, Ordering};
            use windows_sys::Win32::System::Console::STD_OUTPUT_HANDLE;
            use windows_sys::Win32::System::Console::{GetStdHandle, WriteConsoleW};

            static CONSOLE_HANDLE: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(ptr::null_mut());

            // Initialize console handle if not done yet
            let handle = CONSOLE_HANDLE.load(Ordering::Relaxed);
            if handle.is_null() {
                use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
                AttachConsole(ATTACH_PARENT_PROCESS);
                let new_handle = GetStdHandle(STD_OUTPUT_HANDLE);
                if new_handle as isize != 0 && new_handle as isize != !0 {
                    CONSOLE_HANDLE.store(new_handle as *mut std::ffi::c_void, Ordering::Relaxed);
                }
            }

            // Write to console if handle is available
            let handle = CONSOLE_HANDLE.load(Ordering::Relaxed);
            if !handle.is_null() {
                let wide_text: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
                let mut written = 0u32;
                WriteConsoleW(
                    handle as windows_sys::Win32::Foundation::HANDLE,
                    wide_text.as_ptr() as *const _,
                    wide_text.len() as u32 - 1,
                    &mut written,
                    ptr::null_mut(),
                );
            }
        }
    }

    // Parse command-line arguments
    let mut areas = Areas::empty();
    let mut profile_mode = false;
    let mut profile_name = String::new();

    for arg in args {
        match arg.as_str() {
            "/?" | "/help" | "-h" | "--help" => {
                #[cfg(windows)]
                {
                    console_print("Tommy Memory Cleaner - Console Mode\n\n");
                    console_print("Usage: TommyMemoryCleaner.exe [OPTIONS]\n\n");
                    console_print("Options:\n");
                    console_print("  /WorkingSet              Optimize Working Set\n");
                    console_print("  /ModifiedPageList        Optimize Modified Page List\n");
                    console_print("  /StandbyList             Optimize Standby List\n");
                    console_print(
                        "  /StandbyListLow          Optimize Low Priority Standby List\n",
                    );
                    console_print("  /SystemFileCache         Optimize System File Cache\n");
                    console_print("  /CombinedPageList        Optimize Combined Page List\n");
                    console_print("  /ModifiedFileCache       Optimize Modified File Cache\n");
                    console_print("  /RegistryCache           Optimize Registry Cache\n");
                    console_print("  /Profile:Normal          Use Normal profile\n");
                    console_print("  /Profile:Balanced        Use Balanced profile\n");
                    console_print("  /Profile:Gaming          Use Gaming profile\n");
                    console_print("  /?                       Show this help\n\n");
                    console_print("Examples:\n");
                    console_print("  TommyMemoryCleaner.exe /WorkingSet /StandbyList\n");
                    console_print("  TommyMemoryCleaner.exe /Profile:Balanced\n");
                }
                #[cfg(not(windows))]
                {
                    println!("Tommy Memory Cleaner - Console Mode");
                    println!();
                    println!("Usage: TommyMemoryCleaner.exe [OPTIONS]");
                    println!();
                    println!("Options:");
                    println!("  /WorkingSet              Optimize Working Set");
                    println!("  /ModifiedPageList        Optimize Modified Page List");
                    println!("  /StandbyList             Optimize Standby List");
                    println!("  /StandbyListLow          Optimize Low Priority Standby List");
                    println!("  /SystemFileCache         Optimize System File Cache");
                    println!("  /CombinedPageList        Optimize Combined Page List");
                    println!("  /ModifiedFileCache       Optimize Modified File Cache");
                    println!("  /RegistryCache           Optimize Registry Cache");
                    println!("  /Profile:Normal          Use Normal profile");
                    println!("  /Profile:Balanced        Use Balanced profile");
                    println!("  /Profile:Gaming          Use Gaming profile");
                    println!("  /?                       Show this help");
                    println!();
                    println!("Examples:");
                    println!("  TommyMemoryCleaner.exe /WorkingSet /StandbyList");
                    println!("  TommyMemoryCleaner.exe /Profile:Balanced");
                }
                return;
            }
            arg if arg.starts_with("/Profile:") => {
                profile_mode = true;
                profile_name = arg.strip_prefix("/Profile:").unwrap_or("").to_string();
            }
            "/WorkingSet" => areas |= Areas::WORKING_SET,
            "/ModifiedPageList" => areas |= Areas::MODIFIED_PAGE_LIST,
            "/StandbyList" => areas |= Areas::STANDBY_LIST,
            "/StandbyListLow" => areas |= Areas::STANDBY_LIST_LOW,
            "/SystemFileCache" => areas |= Areas::SYSTEM_FILE_CACHE,
            "/CombinedPageList" => areas |= Areas::COMBINED_PAGE_LIST,
            "/ModifiedFileCache" => areas |= Areas::MODIFIED_FILE_CACHE,
            "/RegistryCache" => areas |= Areas::REGISTRY_CACHE,
            _ => {
                #[cfg(windows)]
                {
                    console_print(&format!("Unknown argument: {}\n", arg));
                    console_print("Use /? for help\n");
                }
                #[cfg(not(windows))]
                {
                    eprintln!("Unknown argument: {}", arg);
                    eprintln!("Use /? for help");
                }
                std::process::exit(1);
            }
        }
    }

    // If profile mode is specified, use the profile's areas
    if profile_mode {
        let profile = match profile_name.as_str() {
            "Normal" => Profile::Normal,
            "Balanced" => Profile::Balanced,
            "Gaming" => Profile::Gaming,
            _ => {
                #[cfg(windows)]
                {
                    console_print(&format!(
                        "Invalid profile: {}. Use Normal, Balanced, or Gaming\n",
                        profile_name
                    ));
                }
                #[cfg(not(windows))]
                {
                    eprintln!(
                        "Invalid profile: {}. Use Normal, Balanced, or Gaming",
                        profile_name
                    );
                }
                std::process::exit(1);
            }
        };
        areas = profile.get_memory_areas();
        #[cfg(windows)]
        {
            console_print(&format!("Using profile: {:?}\n", profile));
        }
        #[cfg(not(windows))]
        {
            println!("Using profile: {:?}", profile);
        }
    }

    // If no areas are specified, use Balanced profile by default
    if areas.is_empty() {
        areas = Profile::Balanced.get_memory_areas();
        #[cfg(windows)]
        {
            console_print("No areas specified, using Balanced profile\n");
        }
        #[cfg(not(windows))]
        {
            println!("No areas specified, using Balanced profile");
        }
    }

    #[cfg(windows)]
    {
        console_print(&format!(
            "Optimizing memory areas: {:?}\n",
            areas.get_names()
        ));
    }
    #[cfg(not(windows))]
    {
        println!("Optimizing memory areas: {:?}", areas.get_names());
        io::stdout().flush().unwrap();
    }

    // Execute optimization synchronously in console mode
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // Initialize privileges before optimization
        if let Err(e) = crate::ensure_privileges_initialized() {
            #[cfg(windows)]
            {
                console_print(&format!("Warning: Failed to initialize privileges: {}\n", e));
            }
            #[cfg(not(windows))]
            {
                eprintln!("Warning: Failed to initialize privileges: {}", e);
            }
        }
        
        // Initialize configuration
        let cfg = match Config::load() {
            Ok(c) => c,
            Err(e) => {
                #[cfg(windows)]
                {
                    console_print(&format!("Failed to load config: {}\n", e));
                    console_print("Using default configuration\n");
                }
                #[cfg(not(windows))]
                {
                    eprintln!("Failed to load config: {}", e);
                    eprintln!("Using default configuration");
                }
                Config::default()
            }
        };

        // Create Arc<Mutex<Config>> for the engine
        let cfg_arc = Arc::new(Mutex::new(cfg));
        let engine = Engine::new(cfg_arc.clone());

        // Execute memory optimization with progress callback
        let progress_callback = |current: u8, total: u8, area: String| {
            #[cfg(windows)]
            {
                console_print(&format!("[{}/{}] Optimizing: {}\n", current + 1, total, area));
            }
            #[cfg(not(windows))]
            {
                println!("[{}/{}] Optimizing: {}", current + 1, total, area);
                io::stdout().flush().unwrap();
            }
        };
        
        match engine.optimize(Reason::Manual, areas, Some(progress_callback)) {
            Ok(result) => {
                let freed_mb = result.freed_physical_bytes.abs() as f64 / 1024.0 / 1024.0;
                #[cfg(windows)]
                {
                    console_print("Optimization completed successfully\n");
                    console_print(&format!("Freed: {:.2} MB\n", freed_mb));
                }
                #[cfg(not(windows))]
                {
                    println!("Optimization completed successfully");
                    println!("Freed: {:.2} MB", freed_mb);
                }

                // Display results for each optimized area
                for area in result.areas {
                    if let Some(error) = area.error {
                        #[cfg(windows)]
                        {
                            console_print(&format!("  {}: FAILED - {}\n", area.name, error));
                        }
                        #[cfg(not(windows))]
                        {
                            eprintln!("  {}: FAILED - {}", area.name, error);
                        }
                    } else {
                        #[cfg(windows)]
                        {
                            console_print(&format!("  {}: OK\n", area.name));
                        }
                        #[cfg(not(windows))]
                        {
                            println!("  {}: OK", area.name);
                        }
                    }
                }

                std::process::exit(0);
            }
            Err(e) => {
                #[cfg(windows)]
                {
                    console_print(&format!("Optimization failed: {}\n", e));
                }
                #[cfg(not(windows))]
                {
                    eprintln!("Optimization failed: {}", e);
                }
                std::process::exit(1);
            }
        }
    });
}
