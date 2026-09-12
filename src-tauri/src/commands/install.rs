use std::path::Path;
use std::sync::Arc;

use hexomc_lib::{
    install_with_loader, FabricInstaller, ForgeInstaller, LoaderInstaller, NeoForgeInstaller,
    ProgressFn, VanillaInstaller,
};
use serde::Serialize;
use tauri::Manager;

use super::instance::{load_config, InstanceSummary};
use super::java::{pick_java_for_tools, resolve_java};
use super::{validate_instance_name, LoaderKind, EVENT_INSTALL_PROGRESS};
use crate::error::CmdResult;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub instance_name: String,
    pub done: usize,
    pub total: usize,
    pub message: String,
}

/// 安裝一個 instance（vanilla 或 vanilla + loader）。
///
/// `baseDir` 為啟動器根目錄，`instanceName` 由前端決定 —— 同一個 MC 版本可以有
/// 多個不同 loader 的 instance，所以名稱不從版本推導。
/// 安裝過程會持續發出 `hexo://install-progress` 事件。
#[tauri::command]
pub async fn install_instance(
    app: tauri::AppHandle,
    base_dir: String,
    mc_version: String,
    instance_name: String,
    loader: LoaderKind,
    loader_version: Option<String>,
    java_path: Option<String>,
) -> CmdResult<InstanceSummary> {
    validate_instance_name(&instance_name)?;
    let base = Path::new(&base_dir);
    tokio::fs::create_dir_all(base).await?;

    let installer: Box<dyn LoaderInstaller> = match loader {
        LoaderKind::Vanilla => Box::new(VanillaInstaller),
        LoaderKind::Fabric => match loader_version {
            Some(v) => Box::new(FabricInstaller::new(v)),
            None => Box::new(FabricInstaller::latest()),
        },
        LoaderKind::Forge | LoaderKind::NeoForge => {
            // Forge / NeoForge 的 installer 會用 Java 跑 processor，這裡要先備妥。
            let java = tool_java(base, java_path.as_deref()).await?;
            match (loader, loader_version) {
                (LoaderKind::Forge, Some(v)) => Box::new(ForgeInstaller::with_version(java, v)),
                (LoaderKind::Forge, None) => Box::new(ForgeInstaller::new(java)),
                (_, Some(v)) => Box::new(NeoForgeInstaller::with_version(java, v)),
                (_, None) => Box::new(NeoForgeInstaller::new(java)),
            }
        }
    };

    let progress: ProgressFn = {
        let app = app.clone();
        let name = instance_name.clone();
        Arc::new(move |done, total, message| {
            let _ = app.emit_all(
                EVENT_INSTALL_PROGRESS,
                InstallProgress {
                    instance_name: name.clone(),
                    done,
                    total,
                    message: message.to_string(),
                },
            );
        })
    };

    install_with_loader(
        &mc_version,
        &instance_name,
        base,
        installer.as_ref(),
        progress,
    )
    .await?;

    let config = load_config(base, &instance_name).await?;
    InstanceSummary::build(base, &instance_name, &config)
}

/// 給 Forge / NeoForge processor 用的 java 路徑：指定 > 系統偵測 > 下載 Java 17。
async fn tool_java(
    base_dir: &Path,
    java_path: Option<&str>,
) -> CmdResult<std::path::PathBuf> {
    if let Some(path) = java_path.filter(|p| !p.is_empty()) {
        return Ok(resolve_java(base_dir, 17, Some(path), false).await?.path);
    }
    if let Some(info) = pick_java_for_tools() {
        return Ok(info.path);
    }
    Ok(resolve_java(base_dir, 17, None, true).await?.path)
}
