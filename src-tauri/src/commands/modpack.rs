use std::future::Future;
use std::path::{Component, Path, PathBuf};

use hexomc_lib::{
    detect_modpack_format, install_modpack_files, read_cf_manifest, read_mrpack_index,
    resolve_forge_version, CurseForgeClient, HexoError, LoaderType, ModpackFormat, ModpackInfo,
    ModpackInstallResult, ProgressFn,
};
use serde::{Deserialize, Serialize};
use tauri::Manager;

use super::install::progress_emitter;
use super::instance::{build_view, unique_instance_id, InstanceView, EVENT_INSTANCES_CHANGED};
use super::{instance_dir, minecraft_dir, LoaderKind};
use crate::error::{CmdError, CmdResult};
use crate::store::instance_meta::{InstanceMeta, LocalStore};
use crate::store::launcher::LauncherConfig;
use crate::store::now_secs;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PackFormat {
    Modrinth,
    CurseForge,
    AtLauncher,
    Ftb,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackPreview {
    pub format: PackFormat,
    pub name: String,
    pub version: Option<String>,
    pub mc_version: String,
    pub loader: LoaderKind,

    pub loader_version: Option<String>,

    pub file_count: usize,
}

impl ModpackPreview {
    pub(super) fn new(format: PackFormat, info: ModpackInfo, file_count: usize) -> Self {
        Self {
            format,
            loader: LoaderKind::from(&info.loader),
            name: info.name,
            version: info.version,
            mc_version: info.mc_version,
            loader_version: info.loader_version,
            file_count,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackTarget {
    pub base_dir: String,

    pub name: Option<String>,
    pub icon: Option<String>,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualDownloadDto {
    pub name: String,
    pub file_name: String,
    pub website: Option<String>,

    pub dest: String,

    pub folder: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackInstallDto {
    pub instance: InstanceView,

    pub manual_downloads: Vec<ManualDownloadDto>,

    pub skipped: Vec<String>,
}

#[tauri::command]
pub async fn inspect_modpack(pack_path: String) -> CmdResult<ModpackPreview> {
    read_pack_file(Path::new(&pack_path)).await
}

#[tauri::command]
pub async fn install_modpack_file(
    app: tauri::AppHandle,
    target: PackTarget,
    pack_path: String,
) -> CmdResult<ModpackInstallDto> {
    install_local_pack(app, target, PathBuf::from(pack_path)).await
}

pub(super) async fn install_local_pack(
    app: tauri::AppHandle,
    target: PackTarget,
    path: PathBuf,
) -> CmdResult<ModpackInstallDto> {
    let preview = read_pack_file(&path).await?;

    let curseforge = match preview.format {
        PackFormat::CurseForge => Some(CurseForgeClient::new(curseforge_key().await?)),
        _ => None,
    };

    install_pack(app, target, preview, move |id, base, progress| async move {
        install_modpack_files(&path, &id, &base, curseforge.as_ref(), progress).await
    })
    .await
}

pub(super) async fn curseforge_key() -> CmdResult<String> {
    let key = LauncherConfig::load().await?.api.curseforge_api_key;
    let key = key.trim();
    if key.is_empty() {
        return Err(CmdError::new(
            "curseforgeKeyMissing",
            "使用 CurseForge 需要 API 金鑰，請先到「設定」填入",
        ));
    }
    Ok(key.to_string())
}

async fn read_pack_file(path: &Path) -> CmdResult<ModpackPreview> {
    let format = detect_modpack_format(path).await.map_err(|e| match e {
        HexoError::Other(_) => CmdError::new(
            "notModpack",
            "這不是 Modrinth（.mrpack）或 CurseForge 模組包；「匯出實例」產生的壓縮檔請改用「從壓縮檔匯入」",
        ),
        other => other.into(),
    })?;

    Ok(match format {
        ModpackFormat::Modrinth => {
            let index = read_mrpack_index(path).await?;
            let count = index
                .files
                .iter()
                .filter(|f| f.is_client_supported())
                .count();
            ModpackPreview::new(PackFormat::Modrinth, index.info()?, count)
        }
        ModpackFormat::CurseForge => {
            let manifest = read_cf_manifest(path).await?;
            let count = manifest.files.iter().filter(|f| f.required).count();
            ModpackPreview::new(PackFormat::CurseForge, manifest.info()?, count)
        }
    })
}

pub(super) async fn install_pack<F, Fut>(
    app: tauri::AppHandle,
    target: PackTarget,
    preview: ModpackPreview,
    run: F,
) -> CmdResult<ModpackInstallDto>
where
    F: FnOnce(String, PathBuf, ProgressFn) -> Fut,
    Fut: Future<Output = hexomc_lib::Result<ModpackInstallResult>>,
{
    let base = PathBuf::from(&target.base_dir);
    let name = target
        .name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| preview.name.trim().to_string());
    let name = if name.is_empty() {
        "模組包".to_string()
    } else {
        name
    };

    let id = unique_instance_id(&base, &name).await?;
    let dir = instance_dir(&base, &id)?;

    tokio::fs::create_dir_all(&dir).await?;
    let progress = progress_emitter(&app, &id);

    let ModpackInstallResult {
        info,
        manual_downloads,
        skipped,
    } = match run(id.clone(), base.clone(), progress).await {
        Ok(result) => result,
        Err(e) => {
            let _ = tokio::fs::remove_dir_all(&dir).await;
            return Err(e.into());
        }
    };

    let loader_version = match (&info.loader, &info.loader_version) {
        (LoaderType::Forge, Some(version)) => resolve_forge_version(&info.mc_version, version)
            .await
            .unwrap_or_else(|_| format!("{}-{}", info.mc_version, version)),
        (_, version) => version.clone().unwrap_or_default(),
    };
    let meta = InstanceMeta {
        name,
        icon: target
            .icon
            .filter(|i| !i.is_empty())
            .unwrap_or_else(|| "grass".to_string()),
        group: target.group.unwrap_or_default().trim().to_string(),
        loader: LoaderKind::from(&info.loader),
        loader_version,
        mc_version: info.mc_version,
        created_at: now_secs(),
        ..InstanceMeta::default()
    };
    meta.save(&dir).await?;

    let mc_dir = minecraft_dir(&base, &id)?;
    let manual_downloads = manual_downloads
        .into_iter()
        .map(|d| {
            let relative = d.dest.strip_prefix(&mc_dir).unwrap_or(&d.dest);
            ManualDownloadDto {
                folder: folder_of(relative).to_string(),
                dest: relative.to_string_lossy().replace('\\', "/"),
                name: d.name,
                file_name: d.file_name,
                website: d.website,
            }
        })
        .collect();

    let _ = app.emit_all(EVENT_INSTANCES_CHANGED, ());
    Ok(ModpackInstallDto {
        instance: build_view(&base, &id, &LocalStore::load().await?).await?,
        manual_downloads,
        skipped,
    })
}

fn folder_of(relative: &Path) -> &'static str {
    let Some(Component::Normal(first)) = relative.components().next() else {
        return "minecraft";
    };
    match first.to_str() {
        Some("mods") => "mods",
        Some("resourcepacks") => "resourcepacks",
        Some("shaderpacks") => "shaderpacks",
        Some("config") => "config",
        Some("saves") => "saves",
        _ => "minecraft",
    }
}
