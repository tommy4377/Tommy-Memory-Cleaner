fn main() {
    // Embed the custom Windows application manifest (asInvoker + DPI awareness
    // + Common Controls). Without this, tauri_build::build() embeds only the
    // default manifest and windows/app.manifest is silently ignored.
    let windows_attributes =
        tauri_build::WindowsAttributes::new().app_manifest(include_str!("windows/app.manifest"));

    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows_attributes))
        .expect("failed to run tauri-build");

    // Embed version info from centralized config
    println!("cargo:rustc-env=CARGO_PKG_VERSION=2.7.0");
    println!("cargo:rustc-env=CARGO_PKG_NAME=TommyMemoryCleaner");
}
