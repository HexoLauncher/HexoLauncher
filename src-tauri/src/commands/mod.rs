pub mod assets;
pub mod auth;
pub mod install;
pub mod instance;
pub mod java;
pub mod launch;
pub mod misc;
pub mod mods;
pub mod version;

use std::path::{Path, PathBuf};

use hexomc_lib::{LoaderType, ModLoader};
use serde::{Deserialize, Serialize};

use crate::error::{CmdError, CmdResult};

/// 安裝進度事件：`{ instanceName, done, total, message }`
pub const EVENT_INSTALL_PROGRESS: &str = "hexo://install-progress";
/// 遊戲結束事件：`{ instanceName, exitCode }`
pub const EVENT_GAME_EXIT: &str = "hexo://game-exit";

/// 前端傳進來的 loader 種類。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoaderKind {
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
    /// 對應到模組偵測/更新用的 loader。Vanilla 沒有模組概念，回 Unknown。
    pub fn mod_loader(self) -> ModLoader {
        match self {
            LoaderKind::Vanilla => ModLoader::Unknown,
            LoaderKind::Fabric => ModLoader::Fabric,
            LoaderKind::Forge => ModLoader::Forge,
            LoaderKind::NeoForge => ModLoader::NeoForge,
        }
    }
}

/// instance 名稱由前端決定，這裡擋掉會跳出 base_dir 的名稱。
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

/// `{base_dir}/instance/{name}`
pub fn instance_dir(base_dir: &Path, name: &str) -> CmdResult<PathBuf> {
    validate_instance_name(name)?;
    Ok(base_dir.join("instance").join(name))
}

/// `{base_dir}/instance/{name}/.minecraft` —— 遊戲資料目錄（mods、saves…）
pub fn minecraft_dir(base_dir: &Path, name: &str) -> CmdResult<PathBuf> {
    Ok(instance_dir(base_dir, name)?.join(".minecraft"))
}

pub fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
