use std::path::PathBuf;
use std::sync::OnceLock;

use hexomc_lib::{
    download_file, fetch_atlauncher_pack_config, fetch_ftb_version_manifest, get_ftb_pack,
    install_atlauncher_pack_files, install_ftb_pack_files, CurseForgeClient, DownloadTask,
    Platform, ProjectDetail, ProjectKind, ProjectVersion, SearchPage, SearchQuery, SortBy,
    VersionFilter,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;

use super::modpack::{
    browser, install_local_pack, install_pack, ModpackInstallDto, ModpackPreview, PackFormat,
    PackTarget,
};
use crate::error::{CmdError, CmdResult};
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
    include_optional: bool,
) -> CmdResult<ModpackInstallDto> {
    match platform {
        Platform::Modrinth | Platform::CurseForge => {
            let file = browser()
                .await?
                .file(platform, ProjectKind::Modpack, &pack_id, &version_id)
                .await?;
            let extension = if platform == Platform::Modrinth {
                "mrpack"
            } else {
                "zip"
            };
            install_downloaded(app, target, &file.url, file.sha1.as_deref(), extension).await
        }
        Platform::AtLauncher => {
            atl_install(
                app,
                target,
                safe_name(&pack_id)?,
                segment(&version_id)?,
                include_optional,
            )
            .await
        }
        Platform::Ftb => {
            ftb_install(
                app,
                target,
                numeric_id(&pack_id)?,
                numeric_id(&version_id)?,
                include_optional,
            )
            .await
        }
    }
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

async fn install_downloaded(
    app: tauri::AppHandle,
    target: PackTarget,
    url: &str,
    sha1: Option<&str>,
    extension: &str,
) -> CmdResult<ModpackInstallDto> {
    let dir = PathBuf::from(&target.base_dir)
        .join("temp")
        .join("modpacks");
    tokio::fs::create_dir_all(&dir).await?;
    let path = dir.join(format!("{}.{}", uuid::Uuid::new_v4(), extension));

    let mut task = DownloadTask::new(url, &path);
    if let Some(sha1) = sha1 {
        task = task.with_sha1(sha1);
    }

    let result = match download_file(&task).await {
        Ok(()) => install_local_pack(app, target, path.clone()).await,
        Err(e) => Err(e.into()),
    };
    let _ = tokio::fs::remove_file(&path).await;
    result
}

fn segment(value: &str) -> CmdResult<&str> {
    let value = value.trim();
    let ok = !value.is_empty()
        && !value.contains("..")
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ' '));
    if ok {
        Ok(value)
    } else {
        Err(CmdError::launcher(format!("ID 不合法: {:?}", value)))
    }
}

fn numeric_id(value: &str) -> CmdResult<u64> {
    value
        .trim()
        .parse()
        .map_err(|_| CmdError::launcher(format!("ID 不合法: {:?}", value)))
}

fn safe_name(value: &str) -> CmdResult<&str> {
    let value = value.trim();
    if value.is_empty() || !value.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(CmdError::launcher(format!(
            "ATLauncher 模組包名稱不合法: {:?}",
            value
        )));
    }
    Ok(value)
}

async fn atl_install(
    app: tauri::AppHandle,
    target: PackTarget,
    safe_name: &str,
    version: &str,
    include_optional: bool,
) -> CmdResult<ModpackInstallDto> {
    let config = fetch_atlauncher_pack_config(safe_name, version).await?;
    let count = config
        .mods
        .iter()
        .filter(|m| m.client && (!m.optional || include_optional))
        .count();
    let preview = ModpackPreview::new(PackFormat::AtLauncher, config.info(safe_name)?, count);

    let safe_name = safe_name.to_string();
    let version = version.to_string();
    install_pack(app, target, preview, move |id, base, progress| async move {
        install_atlauncher_pack_files(&safe_name, &version, &id, &base, include_optional, progress)
            .await
    })
    .await
}

async fn ftb_install(
    app: tauri::AppHandle,
    target: PackTarget,
    pack_id: u64,
    version_id: u64,
    include_optional: bool,
) -> CmdResult<ModpackInstallDto> {
    let pack = get_ftb_pack(pack_id).await?;

    let manifest = fetch_ftb_version_manifest(pack_id, version_id).await?;
    let count = manifest
        .files
        .iter()
        .filter(|f| !f.serveronly && (!f.optional || include_optional))
        .count();
    let preview = ModpackPreview::new(PackFormat::Ftb, manifest.info(&pack.name)?, count);

    let key = LauncherConfig::load().await?.api.curseforge_api_key;
    let curseforge = Some(key.trim())
        .filter(|k| !k.is_empty())
        .map(CurseForgeClient::new);

    install_pack(app, target, preview, move |id, base, progress| async move {
        install_ftb_pack_files(
            pack_id,
            version_id,
            &id,
            &base,
            curseforge.as_ref(),
            include_optional,
            progress,
        )
        .await
    })
    .await
}
