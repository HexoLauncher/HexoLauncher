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
    /// "release" | "snapshot" | "old_beta" | "old_alpha"
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

/// Mojang 版本清單。`includeSnapshots` 為 false 時只留正式版。
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

/// 某個 MC 版本可用的 loader 版本清單（vanilla 回空陣列）。
#[tauri::command]
pub async fn list_loader_versions(loader: LoaderKind, mc_version: String) -> CmdResult<Vec<String>> {
    // Forge / NeoForge 的 installer 建構需要 java 路徑，但查版本清單用不到，
    // 抓不到 java 也不該讓查詢失敗，所以這裡允許沒有。
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
