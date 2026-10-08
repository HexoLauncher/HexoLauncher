use std::collections::HashMap;
use std::path::Path;

use base64::Engine;
use hexomc_lib::{
    check_updates, read_mod_icon, read_mod_info, update_mod, CurseForgeClient, DependencyKind,
    ModInfo, ModLoader, ModUpdate, ModrinthClient, UpdateSource,
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

impl ModDto {
    fn from_info(info: ModInfo) -> Self {
        let enabled = !info.file_name.ends_with(DISABLED_SUFFIX);
        let (name, version) = if info.loader == ModLoader::Unknown {
            let jar_name = info.file_name.trim_end_matches(DISABLED_SUFFIX);
            (jar_name.trim_end_matches(".jar").to_string(), String::new())
        } else {
            (info.name, info.version)
        };

        Self {
            file_path: path_string(&info.file_path),
            mod_id: info.mod_id,
            loader: loader_label(&info.loader).to_string(),
            description: info.description,
            file_name: info.file_name,
            name,
            version,
            enabled,
        }
    }
}

fn is_mod_file(path: &Path) -> bool {
    let name = path.to_string_lossy().to_ascii_lowercase();
    name.ends_with(".jar") || name.ends_with(".jar.disabled")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModDependencyDto {
    pub mod_id: String,
    pub version: Option<String>,
    /// "required" | "optional" | "incompatible"
    pub kind: DependencyKind,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModDetailsDto {
    #[serde(flatten)]
    pub summary: ModDto,
    pub size: u64,
    pub modified: u64,
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub sources: Option<String>,
    pub issues: Option<String>,
    /// data URL
    pub icon: Option<String>,
    pub dependencies: Vec<ModDependencyDto>,
}


const MAX_ICON_BYTES: u64 = 4 * 1024 * 1024;

fn icon_data_url(jar: &Path, icon: &str) -> Option<String> {
    let mime = match icon.rsplit('.').next()?.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => return None,
    };
    let bytes = read_mod_icon(jar, icon, MAX_ICON_BYTES)?;
    Some(format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
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

    let mut mods = tokio::task::spawn_blocking(move || {
        let Ok(entries) = std::fs::read_dir(&mods_dir) else {
            return Vec::new();
        };
        entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| is_mod_file(path))
            .filter_map(|path| read_mod_info(&path))
            .map(ModDto::from_info)
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|e| CmdError::launcher(e.to_string()))?;

    mods.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(mods)
}

#[tauri::command]
pub async fn mod_details(
    base_dir: String,
    instance_name: String,
    file_name: String,
) -> CmdResult<ModDetailsDto> {
    let path = mods_dir(&base_dir, &instance_name, &file_name)?.join(&file_name);

    tokio::task::spawn_blocking(move || {
        let not_found = || CmdError::launcher(format!("找不到模組: {}", file_name));
        let file_meta = std::fs::metadata(&path).map_err(|_| not_found())?;
        let info = read_mod_info(&path).ok_or_else(not_found)?;

        Ok(ModDetailsDto {
            size: file_meta.len(),
            modified: file_meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
            authors: info.authors.clone(),
            license: info.license.clone(),
            homepage: info.homepage.clone(),
            sources: info.sources.clone(),
            issues: info.issues.clone(),
            icon: info
                .icon
                .as_deref()
                .and_then(|icon| icon_data_url(&path, icon))
                .or_else(|| icon_data_url(&path, "pack.png")),
            dependencies: info
                .dependencies
                .iter()
                .map(|d| ModDependencyDto {
                    mod_id: d.mod_id.clone(),
                    version: d.version.clone(),
                    kind: d.kind,
                })
                .collect(),
            summary: ModDto::from_info(info),
        })
    })
    .await
    .map_err(|e| CmdError::launcher(e.to_string()))?
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
