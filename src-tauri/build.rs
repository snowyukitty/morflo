fn main() {
    println!("cargo:rerun-if-env-changed=MORFLO_REVIEWED_ENGINE_MANIFEST_SHA256");
    tauri_build::build();
}
