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

    pub loader: String,
    pub description: Option<String>,

    pub enabled: bool,
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
            enabled: true,
        }
    }
}

fn disabled_mod(path: &std::path::Path) -> Option<ModDto> {
    let file_name = path.file_name()?.to_string_lossy().to_string();
    let stem = file_name.trim_end_matches(DISABLED_SUFFIX).to_string();

    Some(ModDto {
        file_path: path_string(path),
        mod_id: stem.clone(),
        name: stem.trim_end_matches(".jar").to_string(),
        version: String::new(),
        loader: "unknown".to_string(),
        description: None,
        enabled: false,
        file_name,
    })
}

const DISABLED_SUFFIX: &str = ".disabled";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModUpdateDto {
    pub id: String,
    pub mod_id: String,
    pub name: String,
    pub current_version: String,
    pub new_version: String,
    pub new_file_name: Option<String>,

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

#[tauri::command]
pub async fn list_mods(base_dir: String, instance_name: String) -> CmdResult<Vec<ModDto>> {
    let mods_dir = minecraft_dir(Path::new(&base_dir), &instance_name)?.join("mods");

    let mut mods: Vec<ModDto> = detect_mods(&mods_dir)
        .into_iter()
        .map(ModDto::from)
        .collect();

    if let Ok(entries) = std::fs::read_dir(&mods_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.to_string_lossy().ends_with(DISABLED_SUFFIX) {
                mods.extend(disabled_mod(&path));
            }
        }
    }

    mods.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(mods)
}

#[tauri::command]
pub async fn set_mod_enabled(
    base_dir: String,
    instance_name: String,
    file_name: String,
    enabled: bool,
) -> CmdResult<()> {
    let mods_dir = mods_dir(&base_dir, &instance_name, &file_name)?;
    let from = mods_dir.join(&file_name);
    if !from.exists() {
        return Err(CmdError::launcher(format!("找不到模組: {}", file_name)));
    }

    let target = if enabled {
        file_name.trim_end_matches(DISABLED_SUFFIX).to_string()
    } else if file_name.ends_with(DISABLED_SUFFIX) {
        return Ok(());
    } else {
        format!("{}{}", file_name, DISABLED_SUFFIX)
    };

    if target == file_name {
        return Ok(());
    }
    tokio::fs::rename(&from, mods_dir.join(target)).await?;
    Ok(())
}

fn mods_dir(base_dir: &str, instance_name: &str, file_name: &str) -> CmdResult<std::path::PathBuf> {
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err(CmdError::launcher(format!("檔名不合法: {}", file_name)));
    }
    Ok(minecraft_dir(Path::new(base_dir), instance_name)?.join("mods"))
}

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
    let api_key = match curseforge_api_key.filter(|k| !k.is_empty()) {
        Some(key) => Some(key),
        None => Some(
            crate::store::launcher::LauncherConfig::load()
                .await?
                .api
                .curseforge_api_key,
        )
        .filter(|k| !k.is_empty()),
    };
    let curseforge = api_key.map(CurseForgeClient::new);
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

#[tauri::command]
pub async fn apply_mod_update(
    state: tauri::State<'_, AppState>,
    base_dir: String,
    instance_name: String,
    update_id: String,
) -> CmdResult<()> {
    let update = state
        .get_mod_update(&update_id)?
        .ok_or_else(|| CmdError::launcher("更新項目不存在，請重新執行 check_mod_updates"))?;

    let mods_dir = minecraft_dir(Path::new(&base_dir), &instance_name)?.join("mods");
    update_mod(&update, &mods_dir).await?;
    state.remove_mod_update(&update_id)?;
    Ok(())
}

#[tauri::command]
pub async fn delete_mod(
    base_dir: String,
    instance_name: String,
    file_name: String,
) -> CmdResult<()> {
    let path = mods_dir(&base_dir, &instance_name, &file_name)?.join(&file_name);
    if !path.exists() {
        return Err(CmdError::launcher(format!("找不到模組: {}", file_name)));
    }
    tokio::fs::remove_file(&path).await?;
    Ok(())
}
