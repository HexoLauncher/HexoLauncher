use std::path::{Path, PathBuf};
use std::sync::Arc;

use hexomc_lib::{
    install_with_loader, FabricInstaller, ForgeInstaller, LoaderInstaller, NeoForgeInstaller,
    ProgressFn, VanillaInstaller,
};
use serde::Serialize;
use tauri::Manager;

use super::instance::{build_view, InstanceView, EVENT_INSTANCES_CHANGED};
use super::java::{java_not_found, pick_java_for_tools, resolve_java};
use super::{instance_dir, validate_instance_name, LoaderKind, EVENT_INSTALL_PROGRESS};
use crate::error::{CmdError, CmdResult};
use crate::store::instance_meta::{InstanceMeta, LocalStore};
use crate::store::now_secs;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub instance_name: String,
    pub done: usize,
    pub total: usize,
    pub message: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallRequest {
    pub base_dir: String,

    pub instance_name: String,

    pub name: Option<String>,
    pub icon: Option<String>,
    pub group: Option<String>,
    pub mc_version: String,
    pub loader: LoaderKind,
    pub loader_version: Option<String>,

    pub java_path: Option<String>,
}

#[tauri::command]
pub async fn install_instance(
    app: tauri::AppHandle,
    request: InstallRequest,
) -> CmdResult<InstanceView> {
    let InstallRequest {
        base_dir,
        instance_name,
        name,
        icon,
        group,
        mc_version,
        loader,
        loader_version,
        java_path,
    } = request;

    validate_instance_name(&instance_name)?;
    let base = Path::new(&base_dir);
    let dir = instance_dir(base, &instance_name)?;
    tokio::fs::create_dir_all(&dir).await?;

    let tool_java = match loader {
        LoaderKind::Forge | LoaderKind::NeoForge => Some(tool_java(java_path.as_deref()).await?),
        _ => None,
    };
    let loader_version = resolve_loader_version(loader, &mc_version, loader_version, &tool_java)
        .await?
        .unwrap_or_default();

    let installer = build_installer(loader, &loader_version, tool_java);

    let progress = progress_emitter(&app, &instance_name);

    let mut meta = InstanceMeta::load(&dir, &instance_name).await?;
    if let Some(name) = name {
        meta.name = name.trim().to_string();
    }
    if let Some(icon) = icon {
        meta.icon = icon;
    }
    if let Some(group) = group {
        meta.group = group.trim().to_string();
    }
    if meta.name.is_empty() {
        meta.name = instance_name.clone();
    }
    if meta.created_at == 0 {
        meta.created_at = now_secs();
    }
    meta.mc_version = mc_version.clone();
    meta.loader = loader;
    meta.loader_version = loader_version;
    meta.save(&dir).await?;

    install_with_loader(
        &mc_version,
        &instance_name,
        base,
        installer.as_ref(),
        progress,
    )
    .await?;

    let _ = app.emit_all(EVENT_INSTANCES_CHANGED, ());
    build_view(base, &instance_name, &LocalStore::load().await?).await
}

#[tauri::command]
pub async fn repair_instance(
    app: tauri::AppHandle,
    base_dir: String,
    instance_name: String,
) -> CmdResult<InstanceView> {
    let base = Path::new(&base_dir);
    let dir = instance_dir(base, &instance_name)?;
    if !dir.exists() {
        return Err(CmdError::new(
            "instanceNotFound",
            format!("找不到 instance: {}", instance_name),
        ));
    }

    let meta = InstanceMeta::load(&dir, &instance_name).await?;
    if meta.mc_version.is_empty() {
        return Err(CmdError::launcher(
            "這個實例沒有記錄 Minecraft 版本，請用「變更版本」重新指定",
        ));
    }

    install_instance(
        app,
        InstallRequest {
            base_dir,
            instance_name,
            name: None,
            icon: None,
            group: None,
            mc_version: meta.mc_version,
            loader: meta.loader,
            loader_version: Some(meta.loader_version).filter(|v| !v.is_empty()),
            java_path: None,
        },
    )
    .await
}

fn build_installer(
    loader: LoaderKind,
    loader_version: &str,
    tool_java: Option<PathBuf>,
) -> Box<dyn LoaderInstaller> {
    let version = Some(loader_version).filter(|v| !v.is_empty());

    match loader {
        LoaderKind::Vanilla => Box::new(VanillaInstaller),
        LoaderKind::Fabric => match version {
            Some(v) => Box::new(FabricInstaller::new(v)),
            None => Box::new(FabricInstaller::latest()),
        },
        LoaderKind::Forge => {
            let java = tool_java.unwrap_or_default();
            match version {
                Some(v) => Box::new(ForgeInstaller::with_version(java, v)),
                None => Box::new(ForgeInstaller::new(java)),
            }
        }
        LoaderKind::NeoForge => {
            let java = tool_java.unwrap_or_default();
            match version {
                Some(v) => Box::new(NeoForgeInstaller::with_version(java, v)),
                None => Box::new(NeoForgeInstaller::new(java)),
            }
        }
    }
}

async fn resolve_loader_version(
    loader: LoaderKind,
    mc_version: &str,
    requested: Option<String>,
    tool_java: &Option<PathBuf>,
) -> CmdResult<Option<String>> {
    if loader == LoaderKind::Vanilla {
        return Ok(None);
    }
    if let Some(version) = requested.filter(|v| !v.is_empty()) {
        return Ok(Some(version));
    }

    let installer = build_installer(loader, "", tool_java.clone());
    let versions = installer.available_versions(mc_version).await?;
    versions.into_iter().next().map(Some).ok_or_else(|| {
        CmdError::new(
            "unsupportedLoader",
            format!("{:?} 沒有支援 Minecraft {}", loader, mc_version),
        )
    })
}

pub(super) async fn tool_java(java_path: Option<&str>) -> CmdResult<PathBuf> {
    if let Some(path) = java_path.filter(|p| !p.is_empty()) {
        if let Some(info) = resolve_java(17, Some(path)).await? {
            return Ok(info.path);
        }
    }
    pick_java_for_tools()
        .await
        .map(|info| info.path)
        .ok_or_else(|| java_not_found(21))
}

pub(super) fn progress_emitter(app: &tauri::AppHandle, instance_name: &str) -> ProgressFn {
    let app = app.clone();
    let name = instance_name.to_string();
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
}
