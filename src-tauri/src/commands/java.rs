use std::path::{Path, PathBuf};

use hexomc_lib::java::detector::probe_java;
use hexomc_lib::{download_java, find_java, JavaInfo};
use serde::Serialize;

use super::path_string;
use crate::error::{CmdError, CmdResult};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaDto {
    pub path: String,
    pub version: u32,
}

impl From<JavaInfo> for JavaDto {
    fn from(info: JavaInfo) -> Self {
        Self {
            path: path_string(&info.path),
            version: info.version,
        }
    }
}

#[tauri::command]
pub fn detect_java(required_version: Option<u32>) -> Option<JavaDto> {
    find_java(required_version.unwrap_or(21)).map(JavaDto::from)
}

#[tauri::command]
pub async fn install_java(base_dir: String, version: u32) -> CmdResult<JavaDto> {
    let path = download_java(version, Path::new(&base_dir)).await?;
    Ok(JavaDto {
        path: path_string(&path),
        version,
    })
}

#[tauri::command]
pub async fn ensure_java(
    base_dir: String,
    required_version: u32,
    allow_download: Option<bool>,
) -> CmdResult<JavaDto> {
    let info = resolve_java(
        Path::new(&base_dir),
        required_version,
        None,
        allow_download.unwrap_or(true),
    )
    .await?;
    Ok(info.into())
}

pub async fn resolve_java(
    base_dir: &Path,
    required_version: u32,
    explicit_path: Option<&str>,
    allow_download: bool,
) -> CmdResult<JavaInfo> {
    if let Some(raw) = explicit_path.filter(|p| !p.is_empty()) {
        let path = PathBuf::from(raw);
        if !path.exists() {
            return Err(CmdError::launcher(format!("指定的 Java 不存在: {}", raw)));
        }

        return Ok(probe_java(&path).unwrap_or(JavaInfo {
            path,
            version: required_version,
        }));
    }

    if let Some(info) = find_java(required_version) {
        return Ok(info);
    }

    if !allow_download {
        return Err(CmdError::new(
            "javaNotFound",
            format!("找不到 Java {}（未允許自動下載）", required_version),
        ));
    }

    let path = download_java(required_version, base_dir).await?;
    Ok(probe_java(&path).unwrap_or(JavaInfo {
        path,
        version: required_version,
    }))
}

/// 給 Forge / NeoForge installer 用的 java：它們的 processor 要跑 Java，
/// 版本要求比遊戲本身寬鬆，所以由新往舊找。
pub fn pick_java_for_tools() -> Option<JavaInfo> {
    find_java(21)
        .or_else(|| find_java(17))
        .or_else(|| find_java(8))
}
