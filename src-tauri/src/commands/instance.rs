use std::path::Path;

use hexomc_lib::InstanceConfig;
use serde::Serialize;

use super::{instance_dir, minecraft_dir, path_string, LoaderKind};
use crate::error::{CmdError, CmdResult};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSummary {
    /// instance 資料夾名稱（前端自己取的，例如 "1.21.1-fabric"）
    pub name: String,
    /// Minecraft 版本，例如 "1.21.1"
    pub version_id: String,
    pub loader: LoaderKind,
    pub main_class: String,
    /// 這個 instance 需要的 Java 主版本
    pub java_version: u32,
    pub assets_id: String,
    pub dir: String,
    /// 遊戲資料目錄（mods / saves / resourcepacks 都在這下面）
    pub minecraft_dir: String,
}

impl InstanceSummary {
    pub fn build(base_dir: &Path, name: &str, config: &InstanceConfig) -> CmdResult<Self> {
        Ok(Self {
            name: name.to_string(),
            version_id: config.version_id.clone(),
            loader: LoaderKind::from(&config.loader_type),
            main_class: config.main_class.clone(),
            java_version: config.java_version,
            assets_id: config.assets_id.clone(),
            dir: path_string(&instance_dir(base_dir, name)?),
            minecraft_dir: path_string(&minecraft_dir(base_dir, name)?),
        })
    }
}

pub async fn load_config(base_dir: &Path, name: &str) -> CmdResult<InstanceConfig> {
    let dir = instance_dir(base_dir, name)?;
    if !InstanceConfig::save_path(&dir).exists() {
        return Err(CmdError::new(
            "instanceNotFound",
            format!("找不到 instance: {}", name),
        ));
    }
    Ok(InstanceConfig::load(&dir).await?)
}

#[tauri::command]
pub async fn list_instances(base_dir: String) -> CmdResult<Vec<InstanceSummary>> {
    let base = Path::new(&base_dir);
    let root = base.join("instance");

    let Ok(mut entries) = tokio::fs::read_dir(&root).await else {
        return Ok(Vec::new());
    };

    let mut result = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        if !entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        // 讀不到 config 的資料夾（安裝到一半、手動放的）直接略過。
        let Ok(config) = InstanceConfig::load(&entry.path()).await else {
            continue;
        };
        result.push(InstanceSummary::build(base, &name, &config)?);
    }

    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

#[tauri::command]
pub async fn get_instance(base_dir: String, instance_name: String) -> CmdResult<InstanceSummary> {
    let base = Path::new(&base_dir);
    let config = load_config(base, &instance_name).await?;
    InstanceSummary::build(base, &instance_name, &config)
}

#[tauri::command]
pub async fn delete_instance(base_dir: String, instance_name: String) -> CmdResult<()> {
    let dir = instance_dir(Path::new(&base_dir), &instance_name)?;
    if !dir.exists() {
        return Err(CmdError::new(
            "instanceNotFound",
            format!("找不到 instance: {}", instance_name),
        ));
    }
    tokio::fs::remove_dir_all(&dir).await?;
    Ok(())
}

#[tauri::command]
pub fn open_instance_folder(base_dir: String, instance_name: String) -> CmdResult<()> {
    let dir = minecraft_dir(Path::new(&base_dir), &instance_name)?;
    if !dir.exists() {
        return Err(CmdError::new(
            "instanceNotFound",
            format!("找不到資料夾: {}", dir.display()),
        ));
    }
    open::that(&dir).map_err(|e| CmdError::launcher(format!("無法開啟資料夾: {}", e)))
}
