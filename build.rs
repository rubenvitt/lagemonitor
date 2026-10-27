// Übernommen aus Suchlage d621418 (27.10.2026): build.rs
fn main() {
    // Ohne diese Zeile bettet sqlx::migrate! neue Migrationen erst nach der nächsten Rust-Änderung ein.
    println!("cargo:rerun-if-changed=migrations");
    // rust-embed braucht den Ordner, damit das Crate auch ohne gebautes Frontend kompiliert.
    let keep = std::path::Path::new("frontend/dist/.gitkeep");
    if !keep.exists() {
        std::fs::create_dir_all("frontend/dist").expect("frontend/dist anlegen");
        std::fs::write(keep, "").expect(".gitkeep anlegen");
    }
}
