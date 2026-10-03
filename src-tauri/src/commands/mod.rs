pub mod assets;
pub mod auth;
pub mod blueprint;
pub mod content;
pub mod export;
pub mod foreign;
pub mod icon;
pub mod install;
pub mod instance;
pub mod java;
pub mod launch;
pub mod misc;
pub mod modpack;
pub mod modpack_browse;
pub mod mods;
pub mod schematic;
pub mod skin;
pub mod theme;
pub mod version;

use std::path::{Path, PathBuf};

use hexomc_lib::{resolve_forge_version, LoaderType, ModLoader};
use serde::{Deserialize, Serialize};

use crate::error::{CmdError, CmdResult};

/// 安裝進度事件：`{ instanceName, done, total, message }`
pub const EVENT_INSTALL_PROGRESS: &str = "hexo://install-progress";
/// 遊戲開始事件：內容是 RunningInfo。
pub const EVENT_GAME_START: &str = "hexo://game-start";
/// 遊戲結束事件：`{ instanceName, exitCode }`
pub const EVENT_GAME_EXIT: &str = "hexo://game-exit";
/// 遊戲輸出事件（每行一次）：`{ instanceName, kind: "stdout" | "stderr", line }`
pub const EVENT_GAME_OUTPUT: &str = "hexo://game-output";
/// Java 下載進度事件：`{ version, done, total, message }`；下載階段 done / total 是位元組。
pub const EVENT_JAVA_PROGRESS: &str = "hexo://java-progress";

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoaderKind {
    #[default]
    Vanilla,
    Fabric,
    Forge,
    NeoForge,
}

impl From<LoaderKind> for LoaderType {
    fn from(kind: LoaderKind) -> Self {
        match kind {
            LoaderKind::Vanilla => LoaderType::Vanilla,
            LoaderKind::Fabric => LoaderType::Fabric,
            LoaderKind::Forge => LoaderType::Forge,
            LoaderKind::NeoForge => LoaderType::NeoForge,
        }
    }
}

impl From<&LoaderType> for LoaderKind {
    fn from(ty: &LoaderType) -> Self {
        match ty {
            LoaderType::Vanilla => LoaderKind::Vanilla,
            LoaderType::Fabric => LoaderKind::Fabric,
            LoaderType::Forge => LoaderKind::Forge,
            LoaderType::NeoForge => LoaderKind::NeoForge,
        }
    }
}

impl LoaderKind {
    pub fn mod_loader(self) -> ModLoader {
        match self {
            LoaderKind::Vanilla => ModLoader::Unknown,
            LoaderKind::Fabric => ModLoader::Fabric,
            LoaderKind::Forge => ModLoader::Forge,
            LoaderKind::NeoForge => ModLoader::NeoForge,
        }
    }
}

pub async fn full_loader_version(
    loader: LoaderKind,
    mc_version: &str,
    loader_version: &str,
) -> String {
    if loader != LoaderKind::Forge || loader_version.is_empty() {
        return loader_version.to_string();
    }
    resolve_forge_version(mc_version, loader_version)
        .await
        .unwrap_or_else(|_| format!("{}-{}", mc_version, loader_version))
}

pub fn validate_instance_name(name: &str) -> CmdResult<()> {
    let bad = name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains(':')
        || Path::new(name).is_absolute();

    if bad {
        return Err(CmdError::launcher(format!(
            "instance 名稱不合法: {:?}（不可為空、不可含路徑分隔符）",
            name
        )));
    }
    Ok(())
}

pub fn instance_dir(base_dir: &Path, name: &str) -> CmdResult<PathBuf> {
    validate_instance_name(name)?;
    Ok(base_dir.join("instance").join(name))
}

pub fn minecraft_dir(base_dir: &Path, name: &str) -> CmdResult<PathBuf> {
    Ok(instance_dir(base_dir, name)?.join(".minecraft"))
}

pub fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
