use std::path::Path;

use base64::Engine;
use hexomc_lib::{detect_maps, detect_resource_packs, detect_shader_packs};
use serde::Serialize;

use super::{minecraft_dir, path_string};
use crate::error::CmdResult;
use hexomc_lib::assets::nbt::{self, Nbt};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackDto {
    pub name: String,
    pub path: String,
    /// true = folder，false = .zip
    pub is_folder: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldDto {
    pub name: String,
    pub path: String,
    pub icon: Option<String>,
}

#[tauri::command]
pub async fn list_resource_packs(
    base_dir: String,
    instance_name: String,
) -> CmdResult<Vec<PackDto>> {
    let dir = minecraft_dir(Path::new(&base_dir), &instance_name)?;
    Ok(detect_resource_packs(&dir)
        .into_iter()
        .map(|p| PackDto {
            name: p.name,
            path: path_string(&p.path),
            is_folder: p.is_folder,
        })
        .collect())
}

#[tauri::command]
pub async fn list_shader_packs(base_dir: String, instance_name: String) -> CmdResult<Vec<PackDto>> {
    let dir = minecraft_dir(Path::new(&base_dir), &instance_name)?;
    Ok(detect_shader_packs(&dir)
        .into_iter()
        .map(|p| PackDto {
            name: p.name,
            path: path_string(&p.path),
            is_folder: p.is_folder,
        })
        .collect())
}

#[tauri::command]
pub async fn list_worlds(base_dir: String, instance_name: String) -> CmdResult<Vec<WorldDto>> {
    let dir = minecraft_dir(Path::new(&base_dir), &instance_name)?;
    Ok(detect_maps(&dir)
        .into_iter()
        .map(|w| WorldDto {
            name: w.name,
            path: path_string(&w.path),
            icon: w.icon.map(|bytes| {
                format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(bytes)
                )
            }),
        })
        .collect())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    /// 修改時間（unix 秒）
    pub modified: u64,
}

fn list_files(dir: &Path, accept: impl Fn(&str) -> bool) -> Vec<FileEntry> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut files: Vec<FileEntry> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if !accept(&name.to_ascii_lowercase()) {
                return None;
            }
            let meta = entry.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            Some(FileEntry {
                name,
                path: path_string(&entry.path()),
                size: meta.len(),
                modified: meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or_default(),
            })
        })
        .collect();

    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    files
}

#[tauri::command]
pub async fn list_screenshots(
    base_dir: String,
    instance_name: String,
) -> CmdResult<Vec<FileEntry>> {
    let dir = minecraft_dir(Path::new(&base_dir), &instance_name)?.join("screenshots");
    Ok(list_files(&dir, |name| {
        name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg")
    }))
}

#[tauri::command]
pub async fn list_instance_logs(
    base_dir: String,
    instance_name: String,
) -> CmdResult<Vec<FileEntry>> {
    let dir = minecraft_dir(Path::new(&base_dir), &instance_name)?.join("logs");
    Ok(list_files(&dir, |name| {
        name.ends_with(".log") || name.ends_with(".txt")
    }))
}

const LOG_TAIL_BYTES: u64 = 512 * 1024;

#[tauri::command]
pub async fn read_instance_log(
    base_dir: String,
    instance_name: String,
    file_name: String,
) -> CmdResult<String> {
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err(crate::error::CmdError::launcher(format!(
            "檔名不合法: {}",
            file_name
        )));
    }

    let path = minecraft_dir(Path::new(&base_dir), &instance_name)?
        .join("logs")
        .join(&file_name);

    let bytes = tokio::fs::read(&path).await?;
    let start = bytes.len().saturating_sub(LOG_TAIL_BYTES as usize);
    // 遊戲的日誌不見得是合法 UTF-8（舊版、或 mod 亂輸出），壞掉的位元組換成替代字元。
    Ok(String::from_utf8_lossy(&bytes[start..]).to_string())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerDto {
    pub name: String,
    pub address: String,

    pub icon: Option<String>,
    pub accept_textures: bool,
}

#[tauri::command]
pub async fn list_servers(base_dir: String, instance_name: String) -> CmdResult<Vec<ServerDto>> {
    let path = minecraft_dir(Path::new(&base_dir), &instance_name)?.join("servers.dat");

    let Ok(bytes) = tokio::fs::read(&path).await else {
        return Ok(Vec::new());
    };
    let Some(root) = nbt::parse(&bytes) else {
        return Ok(Vec::new());
    };
    let Some(servers) = root.get("servers").and_then(Nbt::as_list) else {
        return Ok(Vec::new());
    };

    Ok(servers
        .iter()
        .map(|entry| ServerDto {
            name: entry
                .get("name")
                .and_then(Nbt::as_str)
                .unwrap_or_default()
                .to_string(),
            address: entry
                .get("ip")
                .and_then(Nbt::as_str)
                .unwrap_or_default()
                .to_string(),
            icon: entry
                .get("icon")
                .and_then(Nbt::as_str)
                .filter(|s| !s.is_empty())
                .map(|s| format!("data:image/png;base64,{}", s)),
            accept_textures: matches!(entry.get("acceptTextures"), Some(Nbt::Byte(1))),
        })
        .collect())
}
