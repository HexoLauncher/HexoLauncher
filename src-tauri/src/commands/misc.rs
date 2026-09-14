use std::path::PathBuf;

use crate::error::{CmdError, CmdResult};
use crate::store::launcher::{prepare, LauncherConfig};
use crate::store::store_dir;

#[tauri::command]
pub fn open_external(url: String) -> CmdResult<()> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(CmdError::launcher(format!("不支援的網址: {}", url)));
    }
    open::that(&url).map_err(|e| CmdError::launcher(format!("無法開啟瀏覽器: {}", e)))
}

#[tauri::command]
pub async fn open_launcher_folder(target: String) -> CmdResult<()> {
    let config = LauncherConfig::load().await?;
    let base = config.base_dir()?;

    let dir: PathBuf = match target.as_str() {
        "base" => base,
        "instances" => base.join("instance"),
        "libraries" => base.join("libraries"),
        "assets" => base.join("assets"),
        "java" => base.join("java"),
        "store" => store_dir()?,
        other => return Err(CmdError::launcher(format!("未知的資料夾: {}", other))),
    };

    tokio::fs::create_dir_all(&dir).await?;
    open::that(&dir).map_err(|e| CmdError::launcher(format!("無法開啟資料夾: {}", e)))
}

#[tauri::command]
pub async fn prepare_base_dir(base_dir: String) -> CmdResult<()> {
    prepare(&PathBuf::from(base_dir)).await
}
