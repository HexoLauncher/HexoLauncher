use std::collections::HashMap;
use std::path::Path;

use hexomc_lib::{
    check_updates, detect_mods, update_mod, CurseForgeClient, ModInfo, ModLoader, ModUpdate,
    ModrinthClient, UpdateSource,
};
use serde::Serialize;

use super::instance::load_config;
use super::{minecraft_dir, path_string, LoaderKind};
use crate::error::{CmdError, CmdResult};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModDto {
    pub file_name: String,
    pub file_path: String,
    pub mod_id: String,
    pub name: String,
    pub version: String,
    /// "fabric" | "forge" | "neoforge" | "quilt" | "unknown"
    pub loader: String,
    pub description: Option<String>,
}

impl From<ModInfo> for ModDto {
    fn from(info: ModInfo) -> Self {
        Self {
            file_name: info.file_name,
            file_path: path_string(&info.file_path),
            mod_id: info.mod_id,
            name: info.name,
            version: info.version,
            loader: loader_label(&info.loader).to_string(),
            description: info.description,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModUpdateDto {
    /// 套用更新時要帶回來的 id（`ModUpdate` 本身無法序列化給前端，故留在後端）
    pub id: String,
    pub mod_id: String,
    pub name: String,
    pub current_version: String,
    pub new_version: String,
    pub new_file_name: Option<String>,
    /// "modrinth" | "curseforge"
    pub source: String,
}

fn loader_label(loader: &ModLoader) -> &'static str {
    match loader {
        ModLoader::Fabric => "fabric",
        ModLoader::Forge => "forge",
        ModLoader::NeoForge => "neoforge",
        ModLoader::Quilt => "quilt",
        ModLoader::Unknown => "unknown",
    }
}

fn source_label(source: &UpdateSource) -> &'static str {
    match source {
        UpdateSource::Modrinth(_) => "modrinth",
        UpdateSource::CurseForge { .. } => "curseforge",
    }
}

fn to_update_dto(id: String, update: &ModUpdate) -> ModUpdateDto {
    ModUpdateDto {
        id,
        mod_id: update.current.mod_id.clone(),
        name: update.current.name.clone(),
        current_version: update.current.version.clone(),
        new_version: update.version_name().to_string(),
        new_file_name: update.file_name().map(|s| s.to_string()),
        source: source_label(&update.source).to_string(),
    }
}

/// 掃描 instance 的 `.minecraft/mods/`。
#[tauri::command]
pub async fn list_mods(base_dir: String, instance_name: String) -> CmdResult<Vec<ModDto>> {
    let mods_dir = minecraft_dir(Path::new(&base_dir), &instance_name)?.join("mods");
    Ok(detect_mods(&mods_dir).into_iter().map(ModDto::from).collect())
}

/// 檢查模組更新。先查 Modrinth（以檔案雜湊比對，不需要對照表）；
/// 有給 `curseforgeApiKey` 與 `modIdMap`（mod_id -> CurseForge project id）時，
/// Modrinth 查不到的模組會再問 CurseForge。
#[tauri::command]
pub async fn check_mod_updates(
    state: tauri::State<'_, AppState>,
    base_dir: String,
    instance_name: String,
    mc_version: Option<String>,
    curseforge_api_key: Option<String>,
    mod_id_map: Option<HashMap<String, u64>>,
) -> CmdResult<Vec<ModUpdateDto>> {
    let base = Path::new(&base_dir);
    let config = load_config(base, &instance_name).await?;

    let loader = LoaderKind::from(&config.loader_type).mod_loader();
    if loader == ModLoader::Unknown {
        return Err(CmdError::new(
            "unsupportedLoader",
            "原版 instance 沒有模組可以更新",
        ));
    }

    let mods_dir = minecraft_dir(base, &instance_name)?.join("mods");
    let mc_version = mc_version.unwrap_or(config.version_id);

    let modrinth = ModrinthClient::new();
    let curseforge = curseforge_api_key
        .filter(|k| !k.is_empty())
        .map(CurseForgeClient::new);
    let id_map = mod_id_map.unwrap_or_default();

    let updates = check_updates(
        &mods_dir,
        &mc_version,
        &loader,
        &modrinth,
        curseforge.as_ref(),
        &id_map,
    )
    .await?;

    Ok(state
        .set_mod_updates(&instance_name, updates)?
        .iter()
        .map(|(id, update)| to_update_dto(id.clone(), update))
        .collect())
}

/// 套用一筆 `check_mod_updates` 找到的更新：下載新檔並刪掉舊檔。
#[tauri::command]
pub async fn apply_mod_update(
    state: tauri::State<'_, AppState>,
    base_dir: String,
    instance_name: String,
    update_id: String,
) -> CmdResult<()> {
    let update = state.get_mod_update(&update_id)?.ok_or_else(|| {
        CmdError::launcher("更新項目不存在，請重新執行 check_mod_updates")
    })?;

    let mods_dir = minecraft_dir(Path::new(&base_dir), &instance_name)?.join("mods");
    update_mod(&update, &mods_dir).await?;
    state.remove_mod_update(&update_id)?;
    Ok(())
}

/// 刪除 mods 資料夾內的某個檔案。
#[tauri::command]
pub async fn delete_mod(
    base_dir: String,
    instance_name: String,
    file_name: String,
) -> CmdResult<()> {
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err(CmdError::launcher(format!("檔名不合法: {}", file_name)));
    }

    let path = minecraft_dir(Path::new(&base_dir), &instance_name)?
        .join("mods")
        .join(&file_name);
    if !path.exists() {
        return Err(CmdError::launcher(format!("找不到模組: {}", file_name)));
    }
    tokio::fs::remove_file(&path).await?;
    Ok(())
}
