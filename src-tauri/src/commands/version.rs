use hexomc_lib::{
    create_loader, fetch_version_manifest, ForgeInstaller, LoaderInstaller, NeoForgeInstaller,
};
use serde::Serialize;

use super::LoaderKind;
use crate::commands::java::pick_java_for_tools;
use crate::error::CmdResult;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    pub id: String,

    pub version_type: String,
    pub release_time: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionList {
    pub latest_release: String,
    pub latest_snapshot: String,
    pub versions: Vec<VersionSummary>,
}

#[tauri::command]
pub async fn list_versions(include_snapshots: Option<bool>) -> CmdResult<VersionList> {
    let manifest = fetch_version_manifest().await?;
    let include_snapshots = include_snapshots.unwrap_or(false);

    let versions = manifest
        .versions
        .into_iter()
        .filter(|v| include_snapshots || v.version_type == "release")
        .map(|v| VersionSummary {
            id: v.id,
            version_type: v.version_type,
            release_time: v.release_time,
        })
        .collect();

    Ok(VersionList {
        latest_release: manifest.latest.release,
        latest_snapshot: manifest.latest.snapshot,
        versions,
    })
}

#[tauri::command]
pub async fn list_loader_versions(
    loader: LoaderKind,
    mc_version: String,
) -> CmdResult<Vec<String>> {
    let installer: Box<dyn LoaderInstaller> = match loader {
        LoaderKind::Forge => Box::new(ForgeInstaller::new(
            pick_java_for_tools().map(|j| j.path).unwrap_or_default(),
        )),
        LoaderKind::NeoForge => Box::new(NeoForgeInstaller::new(
            pick_java_for_tools().map(|j| j.path).unwrap_or_default(),
        )),
        other => create_loader(other.into(), None),
    };

    Ok(installer.available_versions(&mc_version).await?)
}
