use std::path::Path;

use hexomc_lib::{
    download_file, DownloadTask, Platform, ProjectDetail, ProjectKind, ProjectVersion,
    SearchPage, SearchQuery, SortBy, VersionFilter,
};
use serde::{Deserialize, Serialize};

use super::modpack::browser;
use super::{minecraft_dir, path_string, LoaderKind};
use crate::error::{CmdError, CmdResult};

pub mod install;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchRequest {
    pub platform: Platform,
    pub kind: ProjectKind,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub sort: SortBy,
    #[serde(default)]
    pub mc_version: Option<String>,
    #[serde(default)]
    pub loader: Option<LoaderKind>,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledContent {
    pub file_name: String,
    pub path: String,
}

#[tauri::command]
pub async fn search_content(request: ContentSearchRequest) -> CmdResult<SearchPage> {
    let query = SearchQuery {
        platform: request.platform,
        kind: request.kind,
        query: request.query,
        sort: request.sort,
        mc_version: request.mc_version,
        loader: request.loader.map(LoaderKind::mod_loader),
        offset: request.offset,
    };
    Ok(browser().await?.search(&query).await?)
}

#[tauri::command]
pub async fn list_content_versions(
    platform: Platform,
    kind: ProjectKind,
    project_id: String,
    mc_version: Option<String>,
    loader: Option<LoaderKind>,
) -> CmdResult<Vec<ProjectVersion>> {
    let filter = VersionFilter {
        mc_version,
        loader: loader.map(LoaderKind::mod_loader),
    };
    Ok(browser()
        .await?
        .versions(platform, kind, &project_id, &filter)
        .await?)
}

#[tauri::command]
pub async fn install_content(
    base_dir: String,
    instance_name: String,
    platform: Platform,
    kind: ProjectKind,
    project_id: String,
    version_id: String,
) -> CmdResult<InstalledContent> {
    let folder = match kind {
        ProjectKind::Mod => {
            return Err(CmdError::new(
                "installPlanRequired",
                "Prepare a mod installation plan first.",
            ))
        }
        _ => kind
            .folder()
            .ok_or_else(|| CmdError::launcher("模組包請從「新增實例」安裝"))?,
    };
    let file = browser()
        .await?
        .file(platform, kind, &project_id, &version_id)
        .await?;

    let dir = minecraft_dir(Path::new(&base_dir), &instance_name)?.join(folder);
    tokio::fs::create_dir_all(&dir).await?;
    let path = dir.join(&file.file_name);

    let mut task = DownloadTask::new(&file.url, &path);
    if let Some(sha1) = file.sha1.as_deref() {
        task = task.with_sha1(sha1);
    }
    download_file(&task).await?;

    Ok(InstalledContent {
        path: path_string(&path),
        file_name: file.file_name,
    })
}

#[tauri::command]
pub async fn get_content_detail(
    platform: Platform,
    kind: ProjectKind,
    project_id: String,
) -> CmdResult<ProjectDetail> {
    Ok(browser()
        .await?
        .detail(platform, kind, &project_id)
        .await?)
}
