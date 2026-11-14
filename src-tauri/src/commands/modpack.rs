use std::future::Future;
use std::path::{Component, Path, PathBuf, MAIN_SEPARATOR};

use hexomc_lib::{
    detect_modpack_format, file_curseforge_fingerprint, install_modpack_files, read_cf_manifest,
    read_hmcl_pack, read_mcbbs_manifest, read_mrpack_index, resolve_forge_version, update_modpack,
    update_modpack_files, Browser, CurseForgeClient, HexoError, LoaderType, ManualDownload,
    ModpackFormat, ModpackInfo, ModpackInstallResult, ModpackUpdateResult, ModrinthClient,
    Platform, ProgressFn,
};
use serde::{Deserialize, Serialize};
use tauri::Emitter;

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

    /// 上一版模組包帶進來、這次不再需要而被刪掉的檔案（相對 `.minecraft`）。
    pub removed_files: Vec<String>,
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
        None => identify_local_pack(&path, &preview).await,
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

async fn identify_local_pack(path: &Path, preview: &ModpackPreview) -> ModpackSource {
    let unknown = ModpackSource {
        version_name: preview.version.clone().unwrap_or_default(),
        ..ModpackSource::default()
    };
    let found = match preview.format {
        PackFormat::Modrinth => identify_modrinth_pack(path).await,
        PackFormat::CurseForge => identify_curseforge_pack(path).await,
        _ => None,
    };
    found.unwrap_or(unknown)
}

async fn identify_modrinth_pack(path: &Path) -> Option<ModpackSource> {
    let sha1 = file_sha1(path).await.ok()?;
    let version = ModrinthClient::new()
        .get_version_by_hash(&sha1)
        .await
        .ok()??;
    Some(ModpackSource {
        platform: Some(Platform::Modrinth),
        project_id: version.project_id,
        version_id: version.id,
        version_name: version.name,
    })
}

async fn identify_curseforge_pack(path: &Path) -> Option<ModpackSource> {
    let client = CurseForgeClient::new(curseforge_key().await.ok()?);
    let fingerprint = file_curseforge_fingerprint(path).await.ok()?;
    let found = client
        .match_fingerprints(&[fingerprint])
        .await
        .ok()?
        .into_iter()
        .next()?;
    Some(ModpackSource {
        platform: Some(Platform::CurseForge),
        project_id: found.mod_id.to_string(),
        version_id: found.file.id.to_string(),
        version_name: found.file.display_name,
    })
}

pub(super) async fn file_sha1(path: &Path) -> CmdResult<String> {
    use sha1::{Digest, Sha1};
    let bytes = tokio::fs::read(path).await?;
    Ok(format!("{:x}", Sha1::digest(&bytes)))
}

pub(super) async fn browser() -> CmdResult<Browser> {
    let key = LauncherConfig::load().await?.api.curseforge_key();
    Ok(Browser::new().with_curseforge_key(key.unwrap_or_default()))
}

pub(super) async fn curseforge_key() -> CmdResult<String> {
    LauncherConfig::load().await?.api.curseforge_key().ok_or_else(|| {
        CmdError::new(
            "curseforgeKeyMissing",
            "使用 CurseForge 需要 API 金鑰，請先到「設定」填入",
        )
    })
}

async fn read_pack_file(path: &Path) -> CmdResult<ModpackPreview> {
    let format = detect_modpack_format(path).await.map_err(|e| match e {
        HexoError::Other(_) => CmdError::new(
            "notModpack",
            "這不是 Modrinth（.mrpack）、CurseForge 或 HMCL 模組包；「匯出實例」產生的壓縮檔請改用「從壓縮檔匯入」",
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
        ModpackFormat::Hmcl => {
            let info = read_hmcl_pack(path).await?.info()?;
            let count = count_zip_files(path, "minecraft/").await?.saturating_sub(1);
            ModpackPreview::new(PackFormat::Hmcl, info, count)
        }
        ModpackFormat::Mcbbs => {
            let info = read_mcbbs_manifest(path).await?.info()?;
            let count = count_zip_files(path, "overrides/").await?;
            ModpackPreview::new(PackFormat::Mcbbs, info, count)
        }
    })
}

async fn count_zip_files(path: &Path, prefix: &'static str) -> CmdResult<usize> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&path)?;
        let archive = zip::ZipArchive::new(std::io::BufReader::new(file))
            .map_err(|e| CmdError::new("zip", e.to_string()))?;
        Ok(archive
            .file_names()
            .filter(|n| n.starts_with(prefix) && !n.ends_with('/'))
            .count())
    })
    .await
    .map_err(|e| CmdError::launcher(e.to_string()))?
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
    Err(CmdError::new(
        "instanceLinked",
        format!(
            "這個實例是用「連結」方式從其他啟動器匯入的，遊戲資料夾實際上在：{}\n更新模組包會直接改到那個啟動器的檔案，而它記錄的載入器版本不會跟著更新，可能導致它無法啟動，所以已停止更新。\n請到原本的啟動器更新這個模組包，或改用「複製」方式重新匯入後再更新。",
            target
        ),
    ))
}

fn same_game_version(meta: &InstanceMeta, preview: &ModpackPreview) -> bool {
    if meta.mc_version != preview.mc_version || meta.loader != preview.loader {
        return false;
    }
    if preview.loader == LoaderKind::Vanilla {
        return true;
    }
    let Some(wanted) = preview.loader_version.as_deref() else {
        return false;
    };
    let current = meta.loader_version.as_str();
    let full = format!("{}-{}", preview.mc_version, wanted);
    current == wanted || current == full || current.starts_with(&format!("{}-", full))
}

pub(super) async fn update_from_path(
    app: tauri::AppHandle,
    base_dir: &str,
    instance_name: &str,
    path: PathBuf,
    source: Option<ModpackSource>,
) -> CmdResult<ModpackUpdateDto> {
    let base = PathBuf::from(base_dir);
    let dir = instance_dir(&base, instance_name)?;
    if !dir.exists() {
        return Err(CmdError::new(
            "instanceNotFound",
            format!("找不到 instance: {}", instance_name),
        ));
    }
    ensure_not_linked(&base, instance_name).await?;

    let preview = read_pack_file(&path).await?;

    let curseforge = match preview.format {
        PackFormat::CurseForge => Some(CurseForgeClient::new(curseforge_key().await?)),
        _ => None,
    };

    let mut meta = InstanceMeta::load(&dir, instance_name).await?;
    let installed = super::instance::load_config(&base, instance_name)
        .await
        .is_ok_and(|config| super::instance::is_healthy(&dir, &config));

    let progress = progress_emitter(&app, instance_name);
    let ModpackUpdateResult {
        install,
        removed_files,
    } = if installed && same_game_version(&meta, &preview) {
        update_modpack_files(&path, instance_name, &base, curseforge.as_ref(), progress).await?
    } else {
        let java = match preview.loader {
            LoaderKind::Forge | LoaderKind::NeoForge => Some(tool_java(None).await?),
            _ => None,
        };
        update_modpack(
            &path,
            instance_name,
            &base,
            java.as_deref(),
            curseforge.as_ref(),
            progress,
        )
        .await?
    };

    let ModpackInstallResult {
        info,
        manual_downloads,
        skipped,
    } = install;

    let source = match source {
        Some(source) => source,
        None => identify_local_pack(&path, &preview).await,
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
