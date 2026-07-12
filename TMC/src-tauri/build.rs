fn main() {
    // Embed the custom Windows application manifest (asInvoker + DPI awareness
    // + Common Controls). Without this, tauri_build::build() embeds only the
    // default manifest and windows/app.manifest is silently ignored.
    let windows_attributes =
        tauri_build::WindowsAttributes::new().app_manifest(include_str!("windows/app.manifest"));

    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows_attributes))
        .expect("failed to run tauri-build");

    // NOTE: never override CARGO_PKG_VERSION/CARGO_PKG_NAME here — cargo sets
    // them from Cargo.toml, and an override silently desyncs env!() values
    // (a stale 2.7.0 override used to shadow the real version).
}
