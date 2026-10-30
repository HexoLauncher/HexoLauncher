use std::path::{Path, PathBuf};

use hexomc_lib::DownloadSource;
use serde::{Deserialize, Serialize};

use super::{read_json, store_file, write_json};
use crate::commands::path_string;
use crate::error::{CmdError, CmdResult};
use crate::state::AppState;

const FILE: &str = "launcher.json";
const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LauncherConfig {
    pub format_version: u32,
    pub base_dir: String,
    pub ui: UiConfig,
    pub theme: ThemeConfig,
    pub game: GameConfig,
    pub java: JavaConfig,
    pub commands: CommandsConfig,
    pub api: ApiConfig,
    pub schematics: SchematicsConfig,
    pub download: DownloadConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UiConfig {
    pub instance_toolbar: bool,
    pub status_bar: bool,

    pub running_badge: bool,

    pub include_snapshots: bool,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ThemeConfig {

    pub mode: String,

    pub preset: String,

    pub accent: String,

    pub tint: f32,

    pub opacity: f32,

    pub background: String,

    pub background_dim: f32,

    pub background_blur: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GameConfig {
    pub min_memory_mb: u32,
    pub max_memory_mb: u32,

    pub jvm_args: String,

    pub record_play_time: bool,

    pub show_progress_window: bool,

    pub close_progress_when_done: bool,

    pub keep_log_lines: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct JavaConfig {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CommandsConfig {
    pub pre_launch: String,
    pub post_exit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ApiConfig {
    pub curseforge_api_key: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DownloadConfig {
    pub source: DownloadKind,
    pub custom_url: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadKind {
    #[default]
    Official,
    Bmclapi,
    Custom,
}

impl DownloadConfig {
    pub fn to_source(&self) -> CmdResult<DownloadSource> {
        match self.source {
            DownloadKind::Official => Ok(DownloadSource::Official),
            DownloadKind::Bmclapi => Ok(DownloadSource::Bmclapi),
            DownloadKind::Custom if self.custom_url.trim().is_empty() => Ok(DownloadSource::Official),
            DownloadKind::Custom => DownloadSource::custom(&self.custom_url)
                .map_err(|_| CmdError::launcher(format!("鏡像網址無效: {}", self.custom_url.trim()))),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SchematicsConfig {
    pub sync: bool,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            base_dir: String::new(),
            ui: UiConfig::default(),
            theme: ThemeConfig::default(),
            game: GameConfig::default(),
            java: JavaConfig::default(),
            commands: CommandsConfig::default(),
            api: ApiConfig::default(),
            schematics: SchematicsConfig::default(),
            download: DownloadConfig::default(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            instance_toolbar: true,
            status_bar: true,
            running_badge: true,
            include_snapshots: false,
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            mode: "dark".into(),
            preset: "crimson".into(),
            accent: "#e5343f".into(),
            tint: 0.5,
            opacity: 0.82,
            background: String::new(),
            background_dim: 0.35,
            background_blur: 0,
        }
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            min_memory_mb: 512,
            max_memory_mb: 4096,
            jvm_args: String::new(),
            record_play_time: true,
            show_progress_window: true,
            close_progress_when_done: true,
            keep_log_lines: 5000,
        }
    }
}

impl Default for JavaConfig {
    fn default() -> Self {
        Self {
            path: String::new(),
        }
    }
}

impl Default for CommandsConfig {
    fn default() -> Self {
        Self {
            pre_launch: String::new(),
            post_exit: String::new(),
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            curseforge_api_key: String::new(),
        }
    }
}

impl LauncherConfig {
    pub async fn load() -> CmdResult<Self> {
        Ok(read_json(&store_file(FILE)?).await?.unwrap_or_default())
    }

    pub async fn save(&self) -> CmdResult<()> {
        write_json(&store_file(FILE)?, self).await
    }

    pub fn base_dir(&self) -> CmdResult<PathBuf> {
        if !self.base_dir.trim().is_empty() {
            return Ok(PathBuf::from(self.base_dir.trim()));
        }
        default_base_dir()
    }
}

/// Windows: `%APPDATA%\Nolauncher`、macOS: `~/Library/Application Support/Nolauncher`、
/// Linux: `~/.local/share/Nolauncher`
pub fn default_base_dir() -> CmdResult<PathBuf> {
    dirs::data_dir()
        .map(|d| d.join(super::APP_DIR))
        .or_else(|| std::env::current_dir().ok().map(|d| d.join("mc_data")))
        .ok_or_else(|| CmdError::launcher("無法取得系統資料目錄"))
}

pub async fn prepare(base: &Path) -> CmdResult<()> {
    for sub in ["assets", "libraries", "instance", "java"] {
        tokio::fs::create_dir_all(base.join(sub)).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_launcher_config() -> CmdResult<LauncherConfig> {
    LauncherConfig::load().await
}

#[tauri::command]
pub async fn save_launcher_config(
    state: tauri::State<'_, AppState>,
    config: LauncherConfig,
) -> CmdResult<LauncherConfig> {
    let mut config = config;
    config.format_version = FORMAT_VERSION;
    let source = config.download.to_source()?;
    {
        let _sync = state.schematic_sync.lock().await;
        config.schematics = LauncherConfig::load().await?.schematics;
        config.save().await?;
    }
    hexomc_lib::set_download_source(source);

    if let Ok(base) = config.base_dir() {
        let _ = prepare(&base).await;
    }
    Ok(config)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Paths {
    /// 實際使用的資料根目錄（已建立）
    pub base_dir: String,
    /// 系統預設的資料根目錄
    pub default_base_dir: String,
    /// 啟動器設定檔所在目錄
    pub store_dir: String,
}

#[tauri::command]
pub async fn resolve_paths() -> CmdResult<Paths> {
    let config = LauncherConfig::load().await?;
    let base = config.base_dir()?;
    prepare(&base).await?;

    Ok(Paths {
        base_dir: path_string(&base),
        default_base_dir: path_string(&default_base_dir()?),
        store_dir: path_string(&super::store_dir()?),
    })
}
