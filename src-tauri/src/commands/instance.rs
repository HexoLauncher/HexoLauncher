use std::path::{Path, PathBuf};

use hexomc_lib::InstanceConfig;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

use super::{instance_dir, minecraft_dir, path_string, validate_instance_name, LoaderKind};
use crate::error::{CmdError, CmdResult};
use crate::store::instance_meta::{
    InstanceLocal, InstanceMeta, InstanceSettings, LocalStore, DEFAULT_ICON,
};
use crate::store::now_secs;

pub const EVENT_INSTANCES_CHANGED: &str = "nolauncher://instances-changed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceView {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub icon_image: Option<String>,
    pub group: String,
    pub notes: String,
    pub version_id: String,
    pub loader: LoaderKind,
    pub loader_version: String,
    pub java_version: u32,
    pub assets_id: String,
    pub main_class: String,
    pub dir: String,
    pub minecraft_dir: String,
    pub mod_count: usize,

    pub last_played: u64,
    pub play_time_seconds: u64,
    pub launch_count: u32,

    pub installed: bool,

    pub healthy: bool,

    pub java_path: String,
    pub settings: InstanceSettings,
}

pub async fn load_config(base_dir: &Path, name: &str) -> CmdResult<InstanceConfig> {
    let dir = instance_dir(base_dir, name)?;
    if !InstanceConfig::save_path(&dir).exists() {
        return Err(CmdError::new(
            "instanceNotFound",
            format!("找不到 instance: {}", name),
        ));
    }
    Ok(InstanceConfig::load(&dir).await?)
}

fn is_healthy(instance_dir: &Path, config: &InstanceConfig) -> bool {
    if !instance_dir
        .join(format!("{}.jar", config.version_id))
        .exists()
    {
        return false;
    }
    config.lib_list.iter().take(8).all(|lib| lib.path.exists())
        && config.natives.iter().take(4).all(|n| n.path.exists())
}

fn count_mods(minecraft_dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(minecraft_dir.join("mods")) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .ends_with(".jar")
        })
        .count()
}

pub async fn ensure_meta(
    dir: &Path,
    name: &str,
    config: Option<&InstanceConfig>,
) -> CmdResult<InstanceMeta> {
    let existed = InstanceMeta::path(dir).exists();
    let mut meta = InstanceMeta::load(dir, name).await?;

    let mut changed = !existed;
    if let Some(config) = config {
        if meta.mc_version.is_empty() {
            meta.mc_version = config.version_id.clone();
            changed = true;
        }
        let loader = LoaderKind::from(&config.loader_type);
        if meta.loader != loader {
            meta.loader = loader;
            changed = true;
        }
    }

    if changed && dir.exists() {
        meta.save(dir).await?;
    }
    Ok(meta)
}

pub async fn build_view(
    base_dir: &Path,
    name: &str,
    local: &LocalStore,
) -> CmdResult<InstanceView> {
    let dir = instance_dir(base_dir, name)?;
    let mc_dir = minecraft_dir(base_dir, name)?;

    let config = InstanceConfig::load(&dir).await.ok();
    let meta = ensure_meta(&dir, name, config.as_ref()).await?;

    Ok(InstanceView {
        id: name.to_string(),
        name: meta.name,
        icon: meta.icon,
        icon_image: super::icon::read_icon(&dir),
        group: meta.group,
        notes: meta.notes,
        version_id: config
            .as_ref()
            .map(|c| c.version_id.clone())
            .unwrap_or(meta.mc_version),
        loader: meta.loader,
        loader_version: meta.loader_version,
        java_version: config.as_ref().map(|c| c.java_version).unwrap_or(21),
        assets_id: config
            .as_ref()
            .map(|c| c.assets_id.clone())
            .unwrap_or_default(),
        main_class: config
            .as_ref()
            .map(|c| c.main_class.clone())
            .unwrap_or_default(),
        healthy: config
            .as_ref()
            .map(|c| is_healthy(&dir, c))
            .unwrap_or(false),
        installed: config.is_some(),
        dir: path_string(&dir),
        minecraft_dir: path_string(&mc_dir),
        mod_count: count_mods(&mc_dir),
        last_played: meta.last_played,
        play_time_seconds: meta.play_time_seconds,
        launch_count: meta.launch_count,
        java_path: local.get(base_dir, name).java_path,
        settings: meta.settings,
    })
}

#[tauri::command]
pub async fn list_instances(base_dir: String) -> CmdResult<Vec<InstanceView>> {
    let base = Path::new(&base_dir);
    let local = LocalStore::load().await?;

    let Ok(mut entries) = tokio::fs::read_dir(base.join("instance")).await else {
        return Ok(Vec::new());
    };

    let mut result = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        if !entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if validate_instance_name(&name).is_err() {
            continue;
        }

        let dir = entry.path();
        if !InstanceMeta::path(&dir).exists() && !InstanceConfig::save_path(&dir).exists() {
            continue;
        }
        result.push(build_view(base, &name, &local).await?);
    }

    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(result)
}

#[tauri::command]
pub async fn get_instance(base_dir: String, instance_name: String) -> CmdResult<InstanceView> {
    let base = Path::new(&base_dir);
    let dir = instance_dir(base, &instance_name)?;
    if !dir.exists() {
        return Err(CmdError::new(
            "instanceNotFound",
            format!("找不到 instance: {}", instance_name),
        ));
    }
    build_view(base, &instance_name, &LocalStore::load().await?).await
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    pub base_dir: String,

    pub name: String,
    pub icon: Option<String>,
    pub icon_path: Option<String>,
    pub group: Option<String>,
    pub mc_version: String,
    pub loader: LoaderKind,
    pub loader_version: Option<String>,
}

#[tauri::command]
pub async fn create_instance(
    app: tauri::AppHandle,
    request: CreateRequest,
) -> CmdResult<InstanceView> {
    let CreateRequest {
        base_dir,
        name,
        icon,
        icon_path,
        group,
        mc_version,
        loader,
        loader_version,
    } = request;

    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(CmdError::launcher("實例名稱不可為空"));
    }
    if mc_version.trim().is_empty() {
        return Err(CmdError::launcher("請先選一個 Minecraft 版本"));
    }

    let base = Path::new(&base_dir);
    let id = unique_instance_id(base, &name).await?;
    let dir = instance_dir(base, &id)?;

    tokio::fs::create_dir_all(minecraft_dir(base, &id)?).await?;

    let mut meta = InstanceMeta {
        name,
        icon: icon
            .filter(|i| !i.is_empty())
            .unwrap_or_else(|| DEFAULT_ICON.to_string()),
        group: group.unwrap_or_default().trim().to_string(),
        mc_version,
        loader,
        loader_version: loader_version.unwrap_or_default(),
        created_at: now_secs(),
        ..InstanceMeta::default()
    };
    super::icon::apply_source(&dir, &mut meta, icon_path.as_deref(), None).await;
    meta.save(&dir).await?;

    let _ = app.emit(EVENT_INSTANCES_CHANGED, ());
    build_view(base, &id, &LocalStore::load().await?).await
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaPatch {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub group: Option<String>,
    pub notes: Option<String>,
    pub settings: Option<InstanceSettings>,

    pub java_path: Option<String>,
}

#[tauri::command]
pub async fn save_instance_meta(
    app: tauri::AppHandle,
    base_dir: String,
    instance_name: String,
    patch: MetaPatch,
) -> CmdResult<InstanceView> {
    let base = Path::new(&base_dir);
    let dir = instance_dir(base, &instance_name)?;
    if !dir.exists() {
        return Err(CmdError::new(
            "instanceNotFound",
            format!("找不到 instance: {}", instance_name),
        ));
    }

    let mut meta = InstanceMeta::load(&dir, &instance_name).await?;
    if let Some(name) = patch.name {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(CmdError::launcher("實例名稱不可為空"));
        }
        meta.name = name;
    }
    if let Some(icon) = patch.icon {
        meta.icon = icon;
    }
    if let Some(group) = patch.group {
        meta.group = group.trim().to_string();
    }
    if let Some(notes) = patch.notes {
        meta.notes = notes;
    }
    if let Some(settings) = patch.settings {
        meta.settings = settings;
    }
    meta.save(&dir).await?;

    if let Some(java_path) = patch.java_path {
        let mut local = LocalStore::load().await?;
        local.set(base, &instance_name, InstanceLocal { java_path });
        local.save().await?;
    }

    let _ = app.emit(EVENT_INSTANCES_CHANGED, ());
    build_view(base, &instance_name, &LocalStore::load().await?).await
}

pub async fn record_play(base_dir: &Path, name: &str, seconds: u64, launched: bool) {
    let Ok(dir) = instance_dir(base_dir, name) else {
        return;
    };
    let Ok(mut meta) = InstanceMeta::load(&dir, name).await else {
        return;
    };
    if launched {
        meta.launch_count += 1;
        meta.last_played = now_secs();
    }
    meta.play_time_seconds += seconds;
    let _ = meta.save(&dir).await;
}

#[tauri::command]
pub async fn delete_instance(
    app: tauri::AppHandle,
    base_dir: String,
    instance_name: String,
) -> CmdResult<()> {
    let base = Path::new(&base_dir);
    let dir = instance_dir(base, &instance_name)?;
    if !dir.exists() {
        return Err(CmdError::new(
            "instanceNotFound",
            format!("找不到 instance: {}", instance_name),
        ));
    }
    // 從別的啟動器連結進來的遊戲目錄要先拆掉連結本身，
    // 不然 remove_dir_all 有機會順著連結把人家的存檔一起刪掉。
    let mc = minecraft_dir(base, &instance_name)?;
    if let Ok(meta) = tokio::fs::symlink_metadata(&mc).await {
        if super::foreign::is_link(&meta) {
            super::foreign::unlink(&mc).await?;
        }
    }
    let schematics = mc.join("schematics");
    if let Ok(meta) = tokio::fs::symlink_metadata(&schematics).await {
        if super::foreign::is_link(&meta) {
            super::foreign::unlink(&schematics).await?;
        }
    }
    tokio::fs::remove_dir_all(&dir).await?;

    let mut local = LocalStore::load().await?;
    local.0.remove(&LocalStore::key(base, &instance_name));
    local.save().await?;

    let _ = app.emit(EVENT_INSTANCES_CHANGED, ());
    Ok(())
}

#[tauri::command]
pub async fn duplicate_instance(
    app: tauri::AppHandle,
    base_dir: String,
    instance_name: String,
    new_name: String,
) -> CmdResult<InstanceView> {
    let base = Path::new(&base_dir);
    let source = instance_dir(base, &instance_name)?;
    if !source.exists() {
        return Err(CmdError::new(
            "instanceNotFound",
            format!("找不到 instance: {}", instance_name),
        ));
    }

    let id = unique_instance_id(base, &new_name).await?;
    let target = instance_dir(base, &id)?;
    copy_dir(&source, &target).await?;

    let mut meta = InstanceMeta::load(&target, &id).await?;
    meta.name = new_name.trim().to_string();
    meta.created_at = now_secs();
    meta.last_played = 0;
    meta.play_time_seconds = 0;
    meta.launch_count = 0;
    meta.save(&target).await?;

    let _ = app.emit(EVENT_INSTANCES_CHANGED, ());
    build_view(base, &id, &LocalStore::load().await?).await
}

async fn copy_dir(source: &Path, target: &Path) -> CmdResult<()> {
    let mut pending = vec![(source.to_path_buf(), target.to_path_buf())];

    while let Some((from, to)) = pending.pop() {
        tokio::fs::create_dir_all(&to).await?;
        let mut entries = tokio::fs::read_dir(&from).await?;
        while let Some(entry) = entries.next_entry().await? {
            let kind = entry.file_type().await?;
            let child_from = entry.path();
            let child_to = to.join(entry.file_name());
            if kind.is_dir() {
                pending.push((child_from, child_to));
            } else if kind.is_file() {
                tokio::fs::copy(&child_from, &child_to).await?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn suggest_instance_id(base_dir: String, name: String) -> CmdResult<String> {
    unique_instance_id(Path::new(&base_dir), &name).await
}

pub async fn unique_instance_id(base_dir: &Path, name: &str) -> CmdResult<String> {
    let root = base_dir.join("instance");
    let slug = slugify(name);

    for n in 0..1000 {
        let candidate = if n == 0 {
            slug.clone()
        } else {
            format!("{}-{}", slug, n + 1)
        };
        if !root.join(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err(CmdError::launcher("找不到可用的實例資料夾名稱"))
}

fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;

    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' {
            out.push(ch);
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }

    let out = out.trim_matches(|c| c == '-' || c == '.').to_string();
    if out.is_empty() {
        "instance".to_string()
    } else {
        out
    }
}

fn folder_target(base_dir: &Path, name: &str, target: &str) -> CmdResult<PathBuf> {
    let dir = instance_dir(base_dir, name)?;
    let mc = minecraft_dir(base_dir, name)?;
    Ok(match target {
        "instance" => dir,
        "minecraft" => mc,
        "mods" => mc.join("mods"),
        "resourcepacks" => mc.join("resourcepacks"),
        "shaderpacks" => mc.join("shaderpacks"),
        "saves" => mc.join("saves"),
        "screenshots" => mc.join("screenshots"),
        "logs" => mc.join("logs"),
        "config" => mc.join("config"),
        "schematics" => mc.join("schematics"),
        other => return Err(CmdError::launcher(format!("未知的資料夾: {}", other))),
    })
}

#[tauri::command]
pub async fn open_instance_folder(
    base_dir: String,
    instance_name: String,
    target: Option<String>,
) -> CmdResult<()> {
    let dir = folder_target(
        Path::new(&base_dir),
        &instance_name,
        target.as_deref().unwrap_or("minecraft"),
    )?;

    tokio::fs::create_dir_all(&dir).await?;
    open::that(&dir).map_err(|e| CmdError::launcher(format!("無法開啟資料夾: {}", e)))
}
