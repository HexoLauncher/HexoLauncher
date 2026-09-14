use std::path::PathBuf;

use serde::Serialize;

use super::path_string;
use crate::error::{CmdError, CmdResult};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseDirInfo {
    pub path: String,
    pub exists: bool,
}

/// Windows: `%APPDATA%\HexoLauncher`、macOS: `~/Library/Application Support/HexoLauncher`、
/// Linux: `~/.local/share/HexoLauncher`
#[tauri::command]
pub fn default_base_dir() -> CmdResult<BaseDirInfo> {
    let dir = dirs::data_dir()
        .map(|d| d.join("HexoLauncher"))
        .or_else(|| std::env::current_dir().ok().map(|d| d.join("mc_data")))
        .ok_or_else(|| CmdError::launcher("無法取得系統資料目錄"))?;

    Ok(BaseDirInfo {
        exists: dir.exists(),
        path: path_string(&dir),
    })
}

#[tauri::command]
pub async fn prepare_base_dir(base_dir: String) -> CmdResult<BaseDirInfo> {
    let base = PathBuf::from(&base_dir);
    for sub in ["assets", "libraries", "instance", "java"] {
        tokio::fs::create_dir_all(base.join(sub)).await?;
    }
    Ok(BaseDirInfo {
        exists: true,
        path: path_string(&base),
    })
}

/// 用系統瀏覽器開啟網址
#[tauri::command]
pub fn open_external(url: String) -> CmdResult<()> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(CmdError::launcher(format!("不支援的網址: {}", url)));
    }
    open::that(&url).map_err(|e| CmdError::launcher(format!("無法開啟瀏覽器: {}", e)))
}
