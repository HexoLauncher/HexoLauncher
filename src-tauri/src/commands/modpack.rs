use std::collections::HashMap;
use std::future::Future;
use std::path::{Component, Path, PathBuf, MAIN_SEPARATOR};
use std::sync::{Arc, Mutex, Weak};

use hexomc_lib::{
    apply_online_modpack, identify_modpack, install_modpack_files, modpack_file_sha1,
    modpack_game_matches, resolve_forge_version, update_modpack, update_modpack_files, Browser,
    CurseForgeClient, HexoError, LoaderType, ManualDownload, ModpackFormat, ModpackInfo,
    ModpackInstallResult, ModpackUpdateResult, OnlineModpack, PackGameInstall, ProgressFn,
};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

use crate::state::AppState;

use super::install::{progress_emitter, tool_java};
use super::instance::{build_view, unique_instance_id, InstanceView, EVENT_INSTANCES_CHANGED};
use super::{instance_dir, minecraft_dir, LoaderKind};
use crate::error::{CmdError, CmdResult};
use crate::store::instance_meta::{InstanceMeta, LocalStore, ModpackSource, DEFAULT_ICON};
use crate::store::launcher::LauncherConfig;
use crate::store::now_secs;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PackFormat {
    Modrinth,
    CurseForge,
    Hmcl,
    Mcbbs,
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
    pub icon_path: Option<String>,
    pub icon_url: Option<String>,
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

    pub sha1: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackInstallDto {
    pub instance: InstanceView,

    pub manual_downloads: Vec<ManualDownloadDto>,

    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackUpdateDto {
    pub instance: InstanceView,

    pub manual_downloads: Vec<ManualDownloadDto>,

    pub skipped: Vec<String>,

    pub removed_files: Vec<String>,
    pub preserved_files: Vec<String>,
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
    install_local_pack(app, target, PathBuf::from(pack_path), None).await
}

pub(super) async fn install_local_pack(
    app: tauri::AppHandle,
    target: PackTarget,
    path: PathBuf,
    source: Option<ModpackSource>,
) -> CmdResult<ModpackInstallDto> {
    let preview = read_pack_file(&path).await?;

    let curseforge = match preview.format {
        PackFormat::CurseForge => Some(CurseForgeClient::new(curseforge_key().await?)),
        _ => None,
    };

    let source = match source {
        Some(source) => source,
        None => identify_local_pack(&path, &preview, curseforge.as_ref()).await,
    };

    install_pack(
        app,
        target,
        preview,
        source,
        move |id, base, progress| async move {
            install_modpack_files(&path, &id, &base, curseforge.as_ref(), progress).await
        },
    )
    .await
}

async fn identify_local_pack(
    path: &Path,
    preview: &ModpackPreview,
    curseforge: Option<&CurseForgeClient>,
) -> ModpackSource {
    let format = match preview.format {
        PackFormat::Modrinth => ModpackFormat::Modrinth,
        PackFormat::CurseForge => ModpackFormat::CurseForge,
        PackFormat::Hmcl => ModpackFormat::Hmcl,
        PackFormat::Mcbbs => ModpackFormat::Mcbbs,
        _ => unreachable!("a local archive has a local format"),
    };
    identify_modpack(path, format, curseforge)
        .await
        .ok()
        .flatten()
        .map_or_else(
            || ModpackSource {
                version_name: preview.version.clone().unwrap_or_default(),
                ..ModpackSource::default()
            },
            |found| ModpackSource {
                platform: Some(found.platform),
                project_id: found.project_id,
                version_id: found.version_id,
                version_name: found.version_name,
                ..ModpackSource::default()
            },
        )
}

pub(super) async fn file_sha1(path: &Path) -> CmdResult<String> {
    Ok(modpack_file_sha1(path).await?)
}

pub(super) async fn browser() -> CmdResult<Browser> {
    let key = LauncherConfig::load().await?.api.curseforge_key();
    Ok(Browser::new().with_curseforge_key(key.unwrap_or_default()))
}

pub(super) async fn curseforge_key() -> CmdResult<String> {
    LauncherConfig::load()
        .await?
        .api
        .curseforge_key()
        .ok_or_else(|| CmdError::coded("curseforgeKeyMissing", "curseforge_key_missing", &[]))
}

async fn read_pack_file(path: &Path) -> CmdResult<ModpackPreview> {
    let preview = hexomc_lib::inspect_modpack(path)
        .await
        .map_err(|error| match error {
            HexoError::Other(ref message)
                if message.starts_with("unrecognized modpack format:") =>
            {
                CmdError::coded("notModpack", "not_modpack", &[])
            }
            other => other.into(),
        })?;
    let format = match preview.format {
        ModpackFormat::Modrinth => PackFormat::Modrinth,
        ModpackFormat::CurseForge => PackFormat::CurseForge,
        ModpackFormat::Hmcl => PackFormat::Hmcl,
        ModpackFormat::Mcbbs => PackFormat::Mcbbs,
    };
    Ok(ModpackPreview::new(
        format,
        preview.info,
        preview.file_count,
    ))
}

#[tauri::command]
pub async fn update_modpack_file(
    app: tauri::AppHandle,
    base_dir: String,
    instance_name: String,
    pack_path: String,
) -> CmdResult<ModpackUpdateDto> {
    update_from_path(
        app,
        &base_dir,
        &instance_name,
        PathBuf::from(pack_path),
        None,
    )
    .await
}

pub(super) async fn ensure_not_linked(base: &Path, instance_name: &str) -> CmdResult<()> {
    let mc_dir = minecraft_dir(base, instance_name)?;
    let linked = tokio::fs::symlink_metadata(&mc_dir)
        .await
        .is_ok_and(|meta| super::foreign::is_link(&meta));
    if !linked {
        return Ok(());
    }
    let target = tokio::fs::read_link(&mc_dir)
        .await
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| mc_dir.display().to_string());
    Err(CmdError::coded(
        "instanceLinked",
        "instance_linked_update",
        &[("path", &target)],
    ))
}

fn same_game_version(meta: &InstanceMeta, preview: &ModpackPreview) -> bool {
    modpack_game_matches(
        &meta.mc_version,
        &meta.loader.into(),
        &meta.loader_version,
        &preview.mc_version,
        &preview.loader.into(),
        preview.loader_version.as_deref(),
    )
}

#[derive(Clone, Copy)]
pub(super) enum PackUpdate<'a> {
    Archive(&'a Path),
    Online(&'a OnlineModpack, PackFormat),
}

type InstanceLock = tokio::sync::Mutex<()>;
static INSTANCE_LOCKS: Mutex<Option<HashMap<String, Weak<InstanceLock>>>> = Mutex::new(None);

pub(super) async fn lock_pack_instance(instance_name: &str) -> tokio::sync::OwnedMutexGuard<()> {
    let lock = {
        let mut locks = INSTANCE_LOCKS
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let locks = locks.get_or_insert_with(HashMap::new);
        locks.retain(|_, lock| lock.strong_count() > 0);
        if let Some(lock) = locks.get(instance_name).and_then(Weak::upgrade) {
            lock
        } else {
            let lock = Arc::new(InstanceLock::new(()));
            locks.insert(instance_name.to_owned(), Arc::downgrade(&lock));
            lock
        }
    };
    lock.lock_owned().await
}

pub(super) async fn update_from_path(
    app: tauri::AppHandle,
    base_dir: &str,
    instance_name: &str,
    path: PathBuf,
    source: Option<ModpackSource>,
) -> CmdResult<ModpackUpdateDto> {
    update_pack(
        app,
        base_dir,
        instance_name,
        PackUpdate::Archive(&path),
        source,
    )
    .await
}

pub(super) async fn update_pack(
    app: tauri::AppHandle,
    base_dir: &str,
    instance_name: &str,
    pack: PackUpdate<'_>,
    source: Option<ModpackSource>,
) -> CmdResult<ModpackUpdateDto> {
    let _lock = lock_pack_instance(instance_name).await;
    let base = PathBuf::from(base_dir);
    let dir = instance_dir(&base, instance_name)?;
    if !dir.exists() {
        return Err(CmdError::instance_not_found(&instance_name));
    }
    ensure_not_linked(&base, instance_name).await?;

    if app.state::<AppState>().is_running(instance_name)? {
        return Err(CmdError::launcher_code("game_running_close_first", &[]));
    }
    super::manual_download::stop_manual_downloads(instance_name.to_owned());
    let include_optional = source
        .as_ref()
        .is_some_and(|source| source.include_optional);
    let preview = match &pack {
        PackUpdate::Archive(path) => read_pack_file(path).await?,
        PackUpdate::Online(pack, format) => {
            ModpackPreview::new(*format, pack.info()?, pack.file_count(include_optional))
        }
    };
    let curseforge = match preview.format {
        PackFormat::CurseForge => Some(CurseForgeClient::new(curseforge_key().await?)),
        PackFormat::Ftb => LauncherConfig::load()
            .await?
            .api
            .curseforge_key()
            .map(CurseForgeClient::new),
        _ => None,
    };

    let progress = progress_emitter(&app, instance_name);
    super::backup::create_with_progress(
        &base,
        instance_name,
        "modpack-update",
        Some(progress.clone()),
    )
    .await?;
    progress(0, 1, "progress.update_modpack");

    let mut meta = InstanceMeta::load(&dir, instance_name).await?;
    let installed = super::instance::load_config(&base, instance_name)
        .await
        .is_ok_and(|config| super::instance::is_healthy(&dir, &config));

    let needs_game = !installed || !same_game_version(&meta, &preview);
    let java = if needs_game && matches!(preview.loader, LoaderKind::Forge | LoaderKind::NeoForge) {
        Some(tool_java(None).await?)
    } else {
        None
    };
    let game = if needs_game {
        PackGameInstall::WithGame {
            java_path: java.as_deref(),
        }
    } else {
        PackGameInstall::FilesOnly
    };
    let ModpackUpdateResult {
        install,
        removed_files,
        preserved_files,
    } = match pack {
        PackUpdate::Archive(path) => match game {
            PackGameInstall::WithGame { java_path } => {
                update_modpack(
                    path,
                    instance_name,
                    &base,
                    java_path,
                    curseforge.as_ref(),
                    progress,
                )
                .await?
            }
            PackGameInstall::FilesOnly => {
                update_modpack_files(path, instance_name, &base, curseforge.as_ref(), progress)
                    .await?
            }
        },
        PackUpdate::Online(pack, _) => {
            apply_online_modpack(
                pack,
                instance_name,
                &base,
                game,
                curseforge.as_ref(),
                include_optional,
                progress,
            )
            .await?
        }
    };

    let ModpackInstallResult {
        info,
        manual_downloads,
        skipped,
    } = install;

    let source = match (source, pack) {
        (Some(source), _) => source,
        (None, PackUpdate::Archive(path)) => {
            identify_local_pack(path, &preview, curseforge.as_ref()).await
        }
        (None, PackUpdate::Online(..)) => unreachable!("online updates supply a modpack source"),
    };

    meta.loader = LoaderKind::from(&info.loader);
    meta.loader_version = pack_loader_version(&info).await;
    meta.mc_version = info.mc_version.clone();
    meta.modpack = Some(source);
    meta.save(&dir).await?;

    let mc_dir = minecraft_dir(&base, instance_name)?;
    let _ = app.emit(EVENT_INSTANCES_CHANGED, ());

    Ok(ModpackUpdateDto {
        instance: build_view(&base, instance_name, &LocalStore::load().await?).await?,
        manual_downloads: manual_download_dtos(manual_downloads, &mc_dir),
        skipped,
        preserved_files: preserved_files
            .iter()
            .map(|p| relative_path(p, &mc_dir))
            .collect(),
        removed_files: removed_files
            .iter()
            .map(|p| relative_path(p, &mc_dir))
            .collect(),
    })
}

pub(super) async fn install_pack<F, Fut>(
    app: tauri::AppHandle,
    target: PackTarget,
    preview: ModpackPreview,
    source: ModpackSource,
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

    let loader_version = pack_loader_version(&info).await;
    let mut meta = InstanceMeta {
        name,
        icon: target
            .icon
            .filter(|i| !i.is_empty())
            .unwrap_or_else(|| DEFAULT_ICON.to_string()),
        group: target.group.unwrap_or_default().trim().to_string(),
        loader: LoaderKind::from(&info.loader),
        loader_version,
        mc_version: info.mc_version,
        created_at: now_secs(),
        modpack: Some(source),
        ..InstanceMeta::default()
    };
    super::icon::apply_source(
        &dir,
        &mut meta,
        target.icon_path.as_deref(),
        target.icon_url.as_deref(),
    )
    .await;
    meta.save(&dir).await?;

    let mc_dir = minecraft_dir(&base, &id)?;
    let _ = app.emit(EVENT_INSTANCES_CHANGED, ());
    Ok(ModpackInstallDto {
        instance: build_view(&base, &id, &LocalStore::load().await?).await?,
        manual_downloads: manual_download_dtos(manual_downloads, &mc_dir),
        skipped,
    })
}

async fn pack_loader_version(info: &ModpackInfo) -> String {
    match (&info.loader, &info.loader_version) {
        (LoaderType::Forge, Some(version)) => resolve_forge_version(&info.mc_version, version)
            .await
            .unwrap_or_else(|_| format!("{}-{}", info.mc_version, version)),
        (_, version) => version.clone().unwrap_or_default(),
    }
}

fn relative_path(path: &Path, mc_dir: &Path) -> String {
    path.strip_prefix(mc_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace(MAIN_SEPARATOR, "/")
}

fn manual_download_dtos(downloads: Vec<ManualDownload>, mc_dir: &Path) -> Vec<ManualDownloadDto> {
    downloads
        .into_iter()
        .map(|d| ManualDownloadDto {
            folder: folder_of(d.dest.strip_prefix(mc_dir).unwrap_or(&d.dest)).to_string(),
            dest: relative_path(&d.dest, mc_dir),
            name: d.name,
            file_name: d.file_name,
            website: d.website,
            sha1: d.sha1,
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::{same_game_version, ModpackPreview, PackFormat};
    use crate::commands::LoaderKind;
    use crate::store::instance_meta::InstanceMeta;

    fn meta(mc: &str, loader: LoaderKind, version: &str) -> InstanceMeta {
        InstanceMeta {
            mc_version: mc.into(),
            loader,
            loader_version: version.into(),
            ..InstanceMeta::default()
        }
    }

    fn preview(mc: &str, loader: LoaderKind, version: Option<&str>) -> ModpackPreview {
        ModpackPreview {
            format: PackFormat::CurseForge,
            name: String::new(),
            version: None,
            mc_version: mc.into(),
            loader,
            loader_version: version.map(str::to_string),
            file_count: 0,
        }
    }

    #[test]
    fn same_versions_skip_game_install() {
        let fabric = meta("1.21.1", LoaderKind::Fabric, "0.16.5");
        assert!(same_game_version(
            &fabric,
            &preview("1.21.1", LoaderKind::Fabric, Some("0.16.5"))
        ));

        let forge = meta("1.20.1", LoaderKind::Forge, "1.20.1-47.2.0");
        assert!(same_game_version(
            &forge,
            &preview("1.20.1", LoaderKind::Forge, Some("47.2.0"))
        ));

        let vanilla = meta("1.21.1", LoaderKind::Vanilla, "");
        assert!(same_game_version(
            &vanilla,
            &preview("1.21.1", LoaderKind::Vanilla, None)
        ));
    }

    #[test]
    fn changed_versions_need_game_install() {
        let fabric = meta("1.21.1", LoaderKind::Fabric, "0.16.5");
        assert!(!same_game_version(
            &fabric,
            &preview("1.21.4", LoaderKind::Fabric, Some("0.16.5"))
        ));
        assert!(!same_game_version(
            &fabric,
            &preview("1.21.1", LoaderKind::Fabric, Some("0.16.9"))
        ));
        assert!(!same_game_version(
            &fabric,
            &preview("1.21.1", LoaderKind::NeoForge, Some("0.16.5"))
        ));
        assert!(!same_game_version(
            &fabric,
            &preview("1.21.1", LoaderKind::Fabric, None)
        ));

        let forge = meta("1.20.1", LoaderKind::Forge, "1.20.1-47.2.0");
        assert!(!same_game_version(
            &forge,
            &preview("1.20.1", LoaderKind::Forge, Some("47.2"))
        ));
    }
}
