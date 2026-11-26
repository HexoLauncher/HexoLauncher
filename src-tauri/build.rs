fn main() {
    println!("cargo:rerun-if-env-changed=NOLAUNCHER_MS_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=NOLAUNCHER_BLUEPRINT_SERVER");
    println!("cargo:rerun-if-env-changed=NOLAUNCHER_NEWS_FEED");
    tauri_build::build()
}
