fn main() {
  println!("cargo:rerun-if-env-changed=NOLAUNCHER_MS_CLIENT_ID");
  tauri_build::build()
}
