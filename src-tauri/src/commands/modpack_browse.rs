use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use hexomc_lib::{
    download_file, fetch_atlauncher_pack_config, fetch_ftb_version_manifest, get_ftb_pack,
    install_atlauncher_pack_files, install_ftb_pack_files, CurseForgeClient, DownloadTask,
    Platform, ProjectDetail, ProjectKind, ProjectVersion, SearchPage, SearchQuery, SortBy,
    VersionFilter,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::modpack::{
    browser, ensure_not_linked, install_local_pack, install_pack, update_from_path,
    ModpackInstallDto, ModpackPreview, ModpackUpdateDto, PackFormat, PackTarget,
};
use super::{instance_dir, minecraft_dir};
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
    };
    match platform {
        Platform::Modrinth | Platform::CurseForge => {
            let path = download_pack(&target.base_dir, platform, &pack_id, &version_id).await?;
            let result = install_local_pack(app, target, path.clone(), Some(source)).await;
            let _ = tokio::fs::remove_file(&path).await;
            result
        }
        Platform::AtLauncher => {
            atl_install(
                app,
                target,
                source,
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
                source,
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

async fn download_pack(
    base_dir: &str,
    platform: Platform,
    pack_id: &str,
    version_id: &str,
) -> CmdResult<PathBuf> {
    let file = browser()
        .await?
        .file(platform, ProjectKind::Modpack, pack_id, version_id)
        .await?;
    let extension = if platform == Platform::Modrinth {
        "mrpack"
    } else {
        "zip"
    };

    let dir = PathBuf::from(base_dir).join("temp").join("modpacks");
    tokio::fs::create_dir_all(&dir).await?;
    let path = dir.join(format!("{}.{}", uuid::Uuid::new_v4(), extension));

    let mut task = DownloadTask::new(&file.url, &path);
    if let Some(sha1) = file.sha1.as_deref() {
        task = task.with_sha1(sha1);
    }
    if let Err(e) = download_file(&task).await {
        let _ = tokio::fs::remove_file(&path).await;
        return Err(e.into());
    }
    Ok(path)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackUpdateCheck {
    pub current: ModpackSource,
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
        Some(platform @ (Platform::Modrinth | Platform::CurseForge))
            if !source.project_id.is_empty() =>
        {
            Ok((source, platform))
        }
        _ => Err(CmdError::new(
            "modpackSourceUnknown",
            "這個實例不是從 Modrinth 或 CurseForge 安裝的模組包，無法線上檢查更新；請改用本機檔案更新",
        )),
    }
}

#[tauri::command]
pub async fn check_modpack_update(
    base_dir: String,
    instance_name: String,
) -> CmdResult<ModpackUpdateCheck> {
    let (current, platform) = remote_source(&base_dir, &instance_name).await?;
    let versions = browser()
        .await?
        .versions(
            platform,
            ProjectKind::Modpack,
            &current.project_id,
            &VersionFilter::default(),
        )
        .await?;
    let versions = newer_versions(versions, &current.version_id);
    Ok(ModpackUpdateCheck { current, versions })
}

const MODPACK_STATE_FILE: &str = "modpack_state.json";

#[derive(Debug, Default, Deserialize)]
struct PackState {
    #[serde(default)]
    managed_files: Vec<String>,
}

#[tauri::command]
pub async fn modpack_missing_files(
    base_dir: String,
    instance_name: String,
) -> CmdResult<Vec<String>> {
    let base = Path::new(&base_dir);
    let state_path = instance_dir(base, &instance_name)?.join(MODPACK_STATE_FILE);
    let Ok(bytes) = tokio::fs::read(&state_path).await else {
        return Ok(Vec::new());
    };
    let state: PackState = serde_json::from_slice(&bytes).unwrap_or_default();
    let mc_dir = minecraft_dir(base, &instance_name)?;
    Ok(missing_files(&mc_dir, state.managed_files))
}

fn missing_files(mc_dir: &Path, managed: Vec<String>) -> Vec<String> {
    managed
        .into_iter()
        .filter(|relative| {
            let path = mc_dir.join(relative);
            let disabled = mc_dir.join(format!("{}.disabled", relative));
            !path.exists() && !disabled.exists()
        })
        .collect()
}

fn newer_versions(versions: Vec<ProjectVersion>, current_id: &str) -> Vec<ProjectVersion> {
    versions
        .into_iter()
        .take_while(|v| v.id != current_id)
        .filter(|v| v.downloadable)
        .collect()
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
    let (platform, pack_id) = match (platform, pack_id) {
        (Some(platform @ (Platform::Modrinth | Platform::CurseForge)), Some(id))
            if !id.trim().is_empty() =>
        {
            (platform, id.trim().to_string())
        }
        (Some(_), _) => {
            return Err(CmdError::launcher(
                "只能從 Modrinth 或 CurseForge 線上更新模組包",
            ))
        }
        _ => {
            let (current, platform) = remote_source(&base_dir, &instance_name).await?;
            (platform, current.project_id)
        }
    };
    ensure_not_linked(Path::new(&base_dir), &instance_name).await?;
    let path = download_pack(&base_dir, platform, &pack_id, &version_id).await?;
    let source = ModpackSource {
        platform: Some(platform),
        project_id: pack_id,
        version_id,
        version_name: version_name.unwrap_or_default(),
    };
    let result = update_from_path(app, &base_dir, &instance_name, path.clone(), Some(source)).await;
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
    source: ModpackSource,
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
    install_pack(
        app,
        target,
        preview,
        source,
        move |id, base, progress| async move {
            install_atlauncher_pack_files(
                &safe_name,
                &version,
                &id,
                &base,
                include_optional,
                progress,
            )
            .await
        },
    )
    .await
}

async fn ftb_install(
    app: tauri::AppHandle,
    target: PackTarget,
    source: ModpackSource,
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

    let curseforge = LauncherConfig::load()
        .await?
        .api
        .curseforge_key()
        .map(CurseForgeClient::new);

    install_pack(
        app,
        target,
        preview,
        source,
        move |id, base, progress| async move {
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
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::newer_versions;
    use hexomc_lib::{ProjectVersion, ReleaseType};

    fn version(id: &str, downloadable: bool) -> ProjectVersion {
        ProjectVersion {
            id: id.into(),
            name: id.into(),
            mc_version: String::new(),
            game_versions: Vec::new(),
            loaders: Vec::new(),
            release_type: ReleaseType::Release,
            published: 0,
            file_name: String::new(),
            downloadable,
        }
    }

    fn ids(list: Vec<ProjectVersion>) -> Vec<String> {
        list.into_iter().map(|v| v.id).collect()
    }

    #[test]
    fn missing_files_skips_present_and_disabled() {
        let temp = std::env::temp_dir().join(format!("nl-missing-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(temp.join("mods")).unwrap();
        std::fs::write(temp.join("mods/a.jar"), b"").unwrap();
        std::fs::write(temp.join("mods/b.jar.disabled"), b"").unwrap();
        let managed = vec![
            "mods/a.jar".to_string(),
            "mods/b.jar".to_string(),
            "mods/c.jar".to_string(),
        ];
        assert_eq!(super::missing_files(&temp, managed), ["mods/c.jar"]);
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn keeps_only_versions_newer_than_current() {
        let list = vec![version("c", true), version("b", true), version("a", true)];
        assert_eq!(ids(newer_versions(list, "b")), ["c"]);
    }

    #[test]
    fn current_latest_means_no_update() {
        let list = vec![version("c", true), version("b", true)];
        assert!(newer_versions(list, "c").is_empty());
    }

    #[test]
    fn unknown_current_offers_everything_downloadable() {
        let list = vec![version("c", false), version("b", true)];
        assert_eq!(ids(newer_versions(list, "")), ["b"]);
    }
}
