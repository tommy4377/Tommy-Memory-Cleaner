/// Critical Windows processes that should NEVER be optimized
/// These are hardcoded and hidden from the user
use once_cell::sync::Lazy;
use std::collections::HashSet;

static CRITICAL_PROCESSES: Lazy<HashSet<String>> = Lazy::new(|| {
    let mut set = HashSet::new();

    // Kernel & Core System
    set.insert("system".to_string()); // Windows Kernel
    set.insert("smss.exe".to_string()); // Session Manager Subsystem
    set.insert("csrss.exe".to_string()); // Client/Server Runtime Subsystem
    set.insert("wininit.exe".to_string()); // Windows Init Process
    set.insert("winlogon.exe".to_string()); // Windows Logon Process
    set.insert("services.exe".to_string()); // Service Control Manager
    set.insert("lsass.exe".to_string()); // Local Security Authority
    set.insert("lsm.exe".to_string()); // Local Session Manager
    set.insert("svchost.exe".to_string()); // Service Host Process
    set.insert("rpcss".to_string()); // RPC Endpoint Mapper
    set.insert("dllhost.exe".to_string()); // COM Surrogate

    // Critical Security
    set.insert("msmpeng.exe".to_string()); // Windows Defender Antimalware
    set.insert("nissrv.exe".to_string()); // Windows Defender Network Inspector
    set.insert("securityhealthservice.exe".to_string()); // Windows Security Health
    set.insert("sgrmbroker.exe".to_string()); // System Guard Runtime Broker
    set.insert("vmcompute.exe".to_string()); // Hyper-V Host Compute Service
    set.insert("vmms.exe".to_string()); // Hyper-V Virtual Machine Management

    // Desktop & UI Critical
    set.insert("dwm.exe".to_string()); // Desktop Window Manager
    set.insert("explorer.exe".to_string()); // Windows Explorer
    set.insert("sihost.exe".to_string()); // Shell Infrastructure Host
    set.insert("fontdrvhost.exe".to_string()); // Font Driver Host
    set.insert("winlogon.exe".to_string()); // Windows Logon
    set.insert("logonui.exe".to_string()); // Logon User Interface
    set.insert("userinit.exe".to_string()); // User Init

    // Memory & Storage Management
    set.insert("memory compression".to_string()); // Memory Compression (Windows 10+)
    set.insert("registry".to_string()); // Registry Process
    set.insert("vmmem".to_string()); // Virtual Machine Memory (WSL2)
    set.insert("vmwp.exe".to_string()); // Virtual Machine Worker Process
    set.insert("pagefileconfig.exe".to_string()); // Pagefile Configuration

    // Critical Drivers & Hardware
    set.insert("ntoskrnl.exe".to_string()); // NT Kernel
    set.insert("hal.dll".to_string()); // Hardware Abstraction Layer
    set.insert("win32k.sys".to_string()); // Win32 Kernel Driver
    set.insert("win32kbase.sys".to_string()); // Win32 Kernel Base Driver
    set.insert("win32kfull.sys".to_string()); // Win32 Kernel Full Driver
    set.insert("cng.sys".to_string()); // Cryptography Next Generation
    set.insert("ksecdd.sys".to_string()); // Kernel Security Device Driver
    set.insert("mountmgr.sys".to_string()); // Mount Manager
    set.insert("volmgr.sys".to_string()); // Volume Manager
    set.insert("volsnap.sys".to_string()); // Volume Shadow Copy
    set.insert("fltmgr.sys".to_string()); // File System Filter Manager
    set.insert("ntfs.sys".to_string()); // NTFS Driver
    set.insert("tcpip.sys".to_string()); // TCP/IP Driver
    set.insert("afd.sys".to_string()); // Ancillary Function Driver
    set.insert("http.sys".to_string()); // HTTP Protocol Stack
    set.insert("mrxsmb.sys".to_string()); // SMB Redirector
    set.insert("rdbss.sys".to_string()); // Redirected Buffering Subsystem
    set.insert("csc.sys".to_string()); // Client Side Caching Driver

    // Power Management
    set.insert("powercfg.exe".to_string()); // Power Configuration
    set.insert("poqexec.exe".to_string()); // Power Quality Executor

    // Windows Update & Maintenance
    set.insert("wuauserv".to_string()); // Windows Update Service
    set.insert("trustedinstaller.exe".to_string()); // Windows Modules Installer
    set.insert("tiworker.exe".to_string()); // Windows Update Worker
    set.insert("wuauclt.exe".to_string()); // Windows Update Client

    // Critical Network Services
    set.insert("dhcp".to_string()); // DHCP Client Service
    set.insert("dnscache".to_string()); // DNS Client Service
    set.insert("netman".to_string()); // Network Connections Manager
    set.insert("nlasvc".to_string()); // Network Location Awareness
    set.insert("nsi".to_string()); // Network Store Interface

    // Audio/Video Critical
    set.insert("audiodg.exe".to_string()); // Audio Device Graph Isolation
    set.insert("audiosrv".to_string()); // Windows Audio Service

    // Third-party Critical Antivirus
    // Common antivirus processes that should not be touched

    // Kaspersky
    set.insert("avp.exe".to_string());
    set.insert("avpui.exe".to_string());
    set.insert("klnagent.exe".to_string());

    // Bitdefender
    set.insert("vsserv.exe".to_string());
    set.insert("bdagent.exe".to_string());
    set.insert("updatesrv.exe".to_string());

    // Norton/Symantec
    set.insert("ns.exe".to_string());
    set.insert("nsbu.exe".to_string());
    set.insert("ccsvchst.exe".to_string());

    // AVG/Avast
    set.insert("avgnt.exe".to_string());
    set.insert("avguard.exe".to_string());
    set.insert("avastsvc.exe".to_string());
    set.insert("avastui.exe".to_string());

    // ESET
    set.insert("ekrn.exe".to_string());
    set.insert("egui.exe".to_string());

    // McAfee
    set.insert("mcshield.exe".to_string());
    set.insert("mfefire.exe".to_string());
    set.insert("mfemms.exe".to_string());

    // Malwarebytes
    set.insert("mbamservice.exe".to_string());
    set.insert("mbamtray.exe".to_string());

    // Windows Defender/Security
    set.insert("mpcmdrun.exe".to_string());
    set.insert("msascuil.exe".to_string());
    set.insert("msmpeng.exe".to_string());

    // Virtualization
    // Critical virtualization processes
    set.insert("vmware-vmx.exe".to_string()); // VMware
    set.insert("virtualbox.exe".to_string()); // VirtualBox
    set.insert("vboxsvc.exe".to_string()); // VirtualBox Service
    set.insert("qemu-system-x86_64.exe".to_string()); // QEMU

    // Database Services
    // Database services that should not be interrupted
    set.insert("sqlservr.exe".to_string()); // SQL Server
    set.insert("mysqld.exe".to_string()); // MySQL
    set.insert("postgres.exe".to_string()); // PostgreSQL
    set.insert("oracle.exe".to_string()); // Oracle Database
    set.insert("mongod.exe".to_string()); // MongoDB

    // Development Critical
    // Critical development processes if present
    set.insert("devenv.exe".to_string()); // Visual Studio
    set.insert("code.exe".to_string()); // VS Code (if debugging)
    set.insert("docker.exe".to_string()); // Docker
    set.insert("dockerd.exe".to_string()); // Docker Daemon
    set.insert("com.docker.service".to_string()); // Docker Service

    // Backup & Sync Critical
    set.insert("onedrive.exe".to_string()); // OneDrive (during sync)
    set.insert("googledrivesync.exe".to_string()); // Google Drive
    set.insert("dropbox.exe".to_string()); // Dropbox

    // System Monitors
    // System monitors that might crash if optimized
    set.insert("procmon.exe".to_string()); // Process Monitor
    set.insert("procexp.exe".to_string()); // Process Explorer
    set.insert("perfmon.exe".to_string()); // Performance Monitor
    set.insert("resmon.exe".to_string()); // Resource Monitor

    // Current Process
    // Our own process!
    set.insert("tommymemorycleaner.exe".to_string());
    set.insert("tmc.exe".to_string());

    set
});

/// Check if a process is critical and should not be optimized
pub fn is_critical_process(process_name: &str) -> bool {
    let name_lower = process_name.to_lowercase();

    // Remove extension if present
    let clean_name = name_lower
        .trim_end_matches(".exe")
        .trim_end_matches(".sys")
        .trim_end_matches(".dll");

    // Exact check
    if CRITICAL_PROCESSES.contains(clean_name) {
        return true;
    }

    // Pattern check (some processes have PIDs or numbers)
    // e.g., svchost.exe, RuntimeBroker.exe can have variants
    let critical_patterns = [
        "svchost",
        "runtimebroker",
        "taskhostw",
        "searchindexer",
        "searchprotocolhost",
        "conhost",
        "wmiprv",
        "spoolsv",
        "msdtc",
        "lsaiso",
        "ctfmon",
        "dashost",
        "backgroundtaskhost",
        "compattelrunner",
    ];

    for pattern in &critical_patterns {
        if clean_name.contains(pattern) {
            return true;
        }
    }

    false
}

/// Get critical processes list (for debug/logging)
/// Note: Function kept for future debug or logging use
#[allow(dead_code)]
pub fn get_critical_processes_list() -> Vec<String> {
    CRITICAL_PROCESSES.iter().cloned().collect()
}
