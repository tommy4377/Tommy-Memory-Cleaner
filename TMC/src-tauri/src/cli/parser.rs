use crate::config::{Config, Profile};
use crate::memory::types::{Areas, Reason};
use crate::engine::Engine;
use std::sync::{Arc, Mutex};

/// Run the application in console mode (CLI)
pub fn run_console_mode(args: &[String]) {
    use std::io::{self, Write};
    
    // Parse arguments
    let mut areas = Areas::empty();
    let mut profile_mode = false;
    let mut profile_name = String::new();
    
    for arg in args {
        match arg.as_str() {
            "/?" | "/help" | "-h" | "--help" => {
                println!("Tommy Memory Cleaner - Console Mode");
                println!();
                println!("Usage: tmc.exe [OPTIONS]");
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
                println!("  tmc.exe /WorkingSet /StandbyList");
                println!("  tmc.exe /Profile:Balanced");
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
                eprintln!("Unknown argument: {}", arg);
                eprintln!("Use /? for help");
                std::process::exit(1);
            }
        }
    }
    
    // Se profile mode è specificato, usa le aree del profilo
    if profile_mode {
        let profile = match profile_name.as_str() {
            "Normal" => Profile::Normal,
            "Balanced" => Profile::Balanced,
            "Gaming" => Profile::Gaming,
            _ => {
                eprintln!("Invalid profile: {}. Use Normal, Balanced, or Gaming", profile_name);
                std::process::exit(1);
            }
        };
        areas = profile.get_memory_areas();
        println!("Using profile: {:?}", profile);
    }
    
    // Se nessuna area è specificata, usa il profilo Balanced di default
    if areas.is_empty() {
        areas = Profile::Balanced.get_memory_areas();
        println!("No areas specified, using Balanced profile");
    }
    
    println!("Optimizing memory areas: {:?}", areas.get_names());
    io::stdout().flush().unwrap();
    
    // Esegui ottimizzazione in modo sincrono (console mode)
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // Inizializza config
        let cfg = match Config::load() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to load config: {}", e);
                eprintln!("Using default configuration");
                Config::default()
            }
        };
        
        // Crea Arc<Mutex<Config>> per l'engine
        let cfg_arc = Arc::new(Mutex::new(cfg));
        let engine = Engine::new(cfg_arc.clone());
        
        // Esegui ottimizzazione
            match engine.optimize::<fn(u8, u8, String)>(Reason::Manual, areas, None) {
            Ok(result) => {
                let freed_mb = result.freed_physical_bytes.abs() as f64 / 1024.0 / 1024.0;
                println!("Optimization completed successfully");
                println!("Freed: {:.2} MB", freed_mb);
                
                // Mostra risultati per area
                for area in result.areas {
                    if let Some(error) = area.error {
                        eprintln!("  {}: FAILED - {}", area.name, error);
                    } else {
                        println!("  {}: OK", area.name);
                    }
                }
                
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Optimization failed: {}", e);
                std::process::exit(1);
            }
        }
    });
}

