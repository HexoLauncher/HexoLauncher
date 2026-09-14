pub mod accounts;
pub mod instance_meta;
pub mod launcher;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{CmdError, CmdResult};

pub fn store_dir() -> CmdResult<PathBuf> {
    dirs::config_dir()
        .map(|d| d.join("HexoLauncher"))
        .or_else(|| std::env::current_dir().ok().map(|d| d.join("config")))
        .ok_or_else(|| CmdError::launcher("無法取得系統設定目錄"))
}

pub fn store_file(name: &str) -> CmdResult<PathBuf> {
    Ok(store_dir()?.join(name))
}

pub async fn read_json<T: DeserializeOwned>(path: &Path) -> CmdResult<Option<T>> {
    let data = match tokio::fs::read_to_string(path).await {
        Ok(data) => data,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    Ok(serde_json::from_str(&data).ok())
}

pub async fn write_json<T: Serialize>(path: &Path, value: &T) -> CmdResult<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let json = serde_json::to_string_pretty(value)?;
    let tmp = path.with_extension("json.tmp");
    tokio::fs::write(&tmp, json).await?;
    tokio::fs::rename(&tmp, path).await?;
    Ok(())
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}
