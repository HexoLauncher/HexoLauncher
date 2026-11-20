use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use hexomc_lib::{
    apply_online_modpack, download_modpack_archive, missing_modpack_files,
    modpack_update_candidates, CurseForgeClient, OnlineModpack, PackGameInstall, Platform,
    ProjectDetail, ProjectKind, ProjectVersion, SearchPage, SearchQuery, SortBy, VersionFilter,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::instance_dir;
use super::modpack::{
    browser, ensure_not_linked, install_local_pack, install_pack, update_pack, ModpackInstallDto,
    ModpackPreview, ModpackUpdateDto, PackFormat, PackTarget, PackUpdate,
};
use crate::error::{CmdError, CmdResult};
use crate::store::instance_meta::{InstanceMeta, ModpackSource};
use crate::store::launcher::LauncherConfig;

const USER_AGENT: &str = concat!("Nolauncher/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub platform: Platform,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub sort: SortBy,
    #[serde(default)]
    pub mc_version: Option<String>,
    #[serde(default)]
    pub offset: usize,
}

#[tauri::command]
pub async fn search_modpacks(request: SearchRequest) -> CmdResult<SearchPage> {
    let query = SearchQuery {
        platform: request.platform,
        kind: ProjectKind::Modpack,
        query: request.query,
        sort: request.sort,
        mc_version: request.mc_version,
        loader: None,
        offset: request.offset,
    };
    Ok(browser().await?.search(&query).await?)
}

#[tauri::command]
pub async fn list_modpack_versions(
    platform: Platform,
    pack_id: String,
) -> CmdResult<Vec<ProjectVersion>> {
    Ok(browser()
        .await?
        .versions(
            platform,
            ProjectKind::Modpack,
            &pack_id,
            &VersionFilter::default(),
        )
        .await?)
}

#[tauri::command]
pub async fn get_modpack_detail(platform: Platform, pack_id: String) -> CmdResult<ProjectDetail> {
    Ok(browser()
        .await?
        .detail(platform, ProjectKind::Modpack, &pack_id)
        .await?)
}

#[tauri::command]
pub async fn install_remote_modpack(
    app: tauri::AppHandle,
    target: PackTarget,
    platform: Platform,
    pack_id: String,
    version_id: String,
    version_name: Option<String>,
    include_optional: bool,
) -> CmdResult<ModpackInstallDto> {
    let source = ModpackSource {
        platform: Some(platform),
        project_id: pack_id.clone(),
        version_id: version_id.clone(),
        version_name: version_name.unwrap_or_default(),
        include_optional,
    };
    match platform {
        Platform::Modrinth | Platform::CurseForge => {
            let browser = browser().await?;
            let archive = download_modpack_archive(
                &browser,
                platform,
                &pack_id,
                &version_id,
                &temp_root(&target.base_dir),
            )
            .await?;
            install_local_pack(app, target, archive.path().to_owned(), Some(source)).await
        }
        Platform::AtLauncher | Platform::Ftb => {
            let online = OnlineModpack::fetch(platform, &pack_id, &version_id).await?;
            let format = online_format(platform);
            let preview =
                ModpackPreview::new(format, online.info()?, online.file_count(include_optional));
            let curseforge = if platform == Platform::Ftb {
                LauncherConfig::load()
                    .await?
                    .api
                    .curseforge_key()
                    .map(CurseForgeClient::new)
            } else {
                None
            };
            install_pack(
                app,
                target,
                preview,
                source,
                move |id, base, progress| async move {
                    Ok(apply_online_modpack(
                        &online,
                        &id,
                        &base,
                        PackGameInstall::FilesOnly,
                        curseforge.as_ref(),
                        include_optional,
                        progress,
                    )
                    .await?
                    .install)
                },
            )
            .await
        }
    }
}

fn online_format(platform: Platform) -> PackFormat {
    match platform {
        Platform::AtLauncher => PackFormat::AtLauncher,
        Platform::Ftb => PackFormat::Ftb,
        _ => unreachable!("archive formats have a separate install path"),
    }
}

fn temp_root(base_dir: &str) -> PathBuf {
    Path::new(base_dir).join("temp").join("modpacks")
}

pub(super) fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .unwrap_or_default()
    })
}

pub(super) async fn get_json<T: DeserializeOwned>(
    request: reqwest::RequestBuilder,
) -> CmdResult<T> {
    Ok(request.send().await?.error_for_status()?.json().await?)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackUpdateCheck {
    pub current: ModpackSource,
    pub current_version_found: bool,
    pub versions: Vec<ProjectVersion>,
}

async fn remote_source(
    base_dir: &str,
    instance_name: &str,
) -> CmdResult<(ModpackSource, Platform)> {
    let dir = instance_dir(Path::new(base_dir), instance_name)?;
    let meta = InstanceMeta::load(&dir, instance_name).await?;
    let source = meta.modpack.unwrap_or_default();
    match source.platform {
        Some(platform) if !source.project_id.is_empty() => Ok((source, platform)),
        _ => Err(CmdError::coded(
            "modpackSourceUnknown",
            "modpack_source_unknown",
            &[],
        )),
    }
}

#[tauri::command]
pub async fn check_modpack_update(
    base_dir: String,
    instance_name: String,
) -> CmdResult<ModpackUpdateCheck> {
    let (current, platform) = remote_source(&base_dir, &instance_name).await?;
    let candidates = modpack_update_candidates(
        &browser().await?,
        platform,
        &current.project_id,
        &current.version_id,
    )
    .await?;
    Ok(ModpackUpdateCheck {
        current,
        versions: candidates.versions,
        current_version_found: candidates.current_version_found,
    })
}

#[tauri::command]
pub async fn modpack_missing_files(
    base_dir: String,
    instance_name: String,
) -> CmdResult<Vec<String>> {
    Ok(missing_modpack_files(Path::new(&base_dir), &instance_name).await?)
}

#[tauri::command]
pub async fn update_remote_modpack(
    app: tauri::AppHandle,
    base_dir: String,
    instance_name: String,
    version_id: String,
    version_name: Option<String>,
    platform: Option<Platform>,
    pack_id: Option<String>,
) -> CmdResult<ModpackUpdateDto> {
    let current = remote_source(&base_dir, &instance_name).await;
    let (platform, pack_id, include_optional) = match (platform, pack_id) {
        (Some(platform), Some(id)) if !id.trim().is_empty() => {
            let id = id.trim().to_string();
            let optional = current.ok().is_some_and(|(source, from)| {
                from == platform && source.project_id == id && source.include_optional
            });
            (platform, id, optional)
        }
        (None, None) => {
            let (source, platform) = current?;
            (platform, source.project_id, source.include_optional)
        }
        _ => return Err(CmdError::launcher_code("modpack_online_only", &[])),
    };
    ensure_not_linked(Path::new(&base_dir), &instance_name).await?;
    super::install::progress_emitter(&app, &instance_name)(0, 1, "progress.download_modpack");
    let source = ModpackSource {
        platform: Some(platform),
        project_id: pack_id,
        version_id,
        version_name: version_name.unwrap_or_default(),
        include_optional,
    };
    match platform {
        Platform::Modrinth | Platform::CurseForge => {
            let archive = download_modpack_archive(
                &browser().await?,
                platform,
                &source.project_id,
                &source.version_id,
                &temp_root(&base_dir),
            )
            .await?;
            update_pack(
                app,
                &base_dir,
                &instance_name,
                PackUpdate::Archive(archive.path()),
                Some(source),
            )
            .await
        }
        Platform::AtLauncher | Platform::Ftb => {
            let online =
                OnlineModpack::fetch(platform, &source.project_id, &source.version_id).await?;
            let pack = PackUpdate::Online(&online, online_format(platform));
            update_pack(app, &base_dir, &instance_name, pack, Some(source)).await
        }
    }
}
