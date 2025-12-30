fn main() {
    // NOTA: Tauri gestisce automaticamente le risorse Windows tramite i metadati in Cargo.toml
    // Il CompanyName viene letto da [package.metadata.winres] e dovrebbe essere usato da Tauri
    // Chiamare winres.compile() esplicitamente causa conflitti (risorsa VERSION duplicata)
    tauri_build::build();
}