use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{now_secs, read_json, store_file, write_json};
use crate::commands::LoaderKind;
use crate::error::CmdResult;

pub const META_FILE: &str = "hexo.instance.json";

const LOCAL_FILE: &str = "instances.local.json";

const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InstanceMeta {
    pub format_version: u32,

    pub name: String,
    pub icon: String,

    pub group: String,
    pub notes: String,
    pub mc_version: String,
    pub loader: LoaderKind,
    pub loader_version: String,

    pub created_at: u64,
    pub last_played: u64,
    pub play_time_seconds: u64,
    pub launch_count: u32,
    pub settings: InstanceSettings,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InstanceSettings {
    pub override_memory: bool,
    pub min_memory_mb: u32,
    pub max_memory_mb: u32,
    pub override_jvm_args: bool,
    pub jvm_args: String,
    pub override_commands: bool,
    pub pre_launch: String,
    pub post_exit: String,
}

impl Default for InstanceMeta {
    fn default() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            name: String::new(),
            icon: "grass".to_string(),
            group: String::new(),
            notes: String::new(),
            mc_version: String::new(),
            loader: LoaderKind::Vanilla,
            loader_version: String::new(),
            created_at: 0,
            last_played: 0,
            play_time_seconds: 0,
            launch_count: 0,
            settings: InstanceSettings::default(),
        }
    }
}

impl InstanceMeta {
    pub fn path(instance_dir: &Path) -> PathBuf {
        instance_dir.join(META_FILE)
    }

    pub async fn load(instance_dir: &Path, fallback_name: &str) -> CmdResult<Self> {
        let mut meta: Self = read_json(&Self::path(instance_dir))
            .await?
            .unwrap_or_default();

        if meta.name.trim().is_empty() {
            meta.name = fallback_name.to_string();
        }
        if meta.icon.trim().is_empty() {
            meta.icon = "grass".to_string();
        }
        Ok(meta)
    }

    pub async fn save(&self, instance_dir: &Path) -> CmdResult<()> {
        let mut meta = self.clone();
        meta.format_version = FORMAT_VERSION;
        if meta.created_at == 0 {
            meta.created_at = now_secs();
        }
        write_json(&Self::path(instance_dir), &meta).await
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InstanceLocal {
    pub java_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LocalStore(pub HashMap<String, InstanceLocal>);

impl LocalStore {
    pub async fn load() -> CmdResult<Self> {
        Ok(read_json(&store_file(LOCAL_FILE)?)
            .await?
            .unwrap_or_default())
    }

    pub async fn save(&self) -> CmdResult<()> {
        write_json(&store_file(LOCAL_FILE)?, self).await
    }

    pub fn key(base_dir: &Path, instance_name: &str) -> String {
        format!("{}|{}", base_dir.display(), instance_name)
    }

    pub fn get(&self, base_dir: &Path, instance_name: &str) -> InstanceLocal {
        self.0
            .get(&Self::key(base_dir, instance_name))
            .cloned()
            .unwrap_or_default()
    }

    pub fn set(&mut self, base_dir: &Path, instance_name: &str, value: InstanceLocal) {
        self.0.insert(Self::key(base_dir, instance_name), value);
    }
}
