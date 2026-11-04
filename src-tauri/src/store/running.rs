use serde::{Deserialize, Serialize};

use super::{read_json, store_file, write_json};
use crate::commands::process;
use crate::error::CmdResult;
use crate::state::{AppState, RunningInfo};

const FILE: &str = "running.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookVars {
    pub instance_name: String,
    pub display_name: String,
    pub instance_dir: String,
    pub minecraft_dir: String,
    pub java_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningRecord {
    pub instance_name: String,
    pub base_dir: String,
    pub pid: u32,
    pub started_at: u64,

    pub proc_start: u64,
    pub player_name: String,
    pub online: bool,
    pub record_play_time: bool,
    #[serde(default)]
    pub post_exit: String,
    #[serde(default)]
    pub hook_vars: HookVars,
}

impl RunningRecord {
    pub fn info(&self) -> RunningInfo {
        RunningInfo {
            instance_name: self.instance_name.clone(),
            pid: self.pid,
            started_at: self.started_at,
            player_name: self.player_name.clone(),
            online: self.online,
        }
    }
}

pub async fn load_alive() -> Vec<RunningRecord> {
    let Ok(path) = store_file(FILE) else {
        return Vec::new();
    };
    let records: Vec<RunningRecord> = read_json(&path).await.ok().flatten().unwrap_or_default();
    records
        .into_iter()
        .filter(|r| process::is_alive(r.pid, r.proc_start))
        .collect()
}

pub async fn persist(state: &AppState) -> CmdResult<()> {
    let _guard = state.running_file.lock().await;
    let records = state.running_records()?;
    write_json(&store_file(FILE)?, &records).await
}
