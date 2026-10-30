fn main() {
  println!("cargo:rerun-if-env-changed=NOLAUNCHER_MS_CLIENT_ID");
  println!("cargo:rerun-if-env-changed=NOLAUNCHER_BLUEPRINT_SERVER");
  tauri_build::build()
}
