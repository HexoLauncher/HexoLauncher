use std::path::Path;

use base64::Engine;
use hexomc_lib::{detect_maps, detect_resource_packs, detect_shader_packs};
use serde::Serialize;

use super::{minecraft_dir, path_string};
use crate::error::CmdResult;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackDto {
    pub name: String,
    pub path: String,
    /// true = 資料夾形式，false = .zip
    pub is_folder: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldDto {
    pub name: String,
    pub path: String,
    /// 存檔縮圖（icon.png）轉成 data URI，可直接餵給 <img src>
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
