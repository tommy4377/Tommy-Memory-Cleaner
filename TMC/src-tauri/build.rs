fn main() {
    tauri_build::build();
    
    // Embed version info from centralized config
    println!("cargo:rustc-env=CARGO_PKG_VERSION=2.7.0");
    println!("cargo:rustc-env=CARGO_PKG_NAME=TommyMemoryCleaner");
}
