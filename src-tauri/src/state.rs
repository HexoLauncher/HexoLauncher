use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Instant;

use hexomc_lib::auth::microsoft::MsDeviceCodeResponse;
use hexomc_lib::{GameProcess, ModUpdate};
use serde::Serialize;

use crate::commands::crash::CrashAnalysis;
use crate::error::CmdError;

const MAX_LOG_LINES: usize = 10_000;

struct RunningGame {
    process: GameProcess,
    started: Instant,
    info: RunningInfo,
    killed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningInfo {
    pub instance_name: String,
    pub pid: u32,
    pub started_at: u64,
    pub player_name: String,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameLogLine {
    pub seq: u64,
    pub kind: &'static str,
    pub line: String,
}

#[derive(Default)]
struct LogBuffer {
    lines: VecDeque<GameLogLine>,
    next_seq: u64,
}

#[derive(Default)]
pub struct AppState {
    running: Mutex<HashMap<String, RunningGame>>,
    logs: Mutex<HashMap<String, LogBuffer>>,
    login_sessions: Mutex<HashMap<String, MsDeviceCodeResponse>>,
    mod_updates: Mutex<HashMap<String, ModUpdate>>,
    pub schematic_sync: tokio::sync::Mutex<()>,
    crashes: Mutex<HashMap<String, CrashAnalysis>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GameEnded {
    pub exit_code: Option<i32>,
    pub seconds: u64,
    pub killed: bool,
}

fn poisoned(what: &str) -> CmdError {
    CmdError::launcher(format!("內部狀態鎖毀損: {}", what))
}

impl AppState {
    pub fn add_running(&self, process: GameProcess, info: RunningInfo) -> Result<(), CmdError> {
        let mut map = self.running.lock().map_err(|_| poisoned("running"))?;
        map.insert(
            info.instance_name.clone(),
            RunningGame {
                process,
                started: Instant::now(),
                info,
                killed: false,
            },
        );
        Ok(())
    }

    pub fn is_running(&self, instance_name: &str) -> Result<bool, CmdError> {
        let map = self.running.lock().map_err(|_| poisoned("running"))?;
        Ok(map.contains_key(instance_name))
    }

    pub fn running_list(&self) -> Result<Vec<RunningInfo>, CmdError> {
        let mut map = self.running.lock().map_err(|_| poisoned("running"))?;
        map.retain(|_, game| !matches!(game.process.try_wait(), Ok(Some(_))));
        Ok(map.values().map(|game| game.info.clone()).collect())
    }

    pub fn reap(&self, instance_name: &str) -> Result<Option<GameEnded>, CmdError> {
        let mut map = self.running.lock().map_err(|_| poisoned("running"))?;
        let Some(game) = map.get_mut(instance_name) else {
            return Ok(Some(GameEnded::default()));
        };

        let seconds = game.started.elapsed().as_secs();
        let killed = game.killed;
        match game.process.try_wait() {
            Ok(Some(status)) => {
                map.remove(instance_name);
                Ok(Some(GameEnded {
                    exit_code: status.code(),
                    seconds,
                    killed,
                }))
            }
            Ok(None) => Ok(None),
            Err(_) => {
                map.remove(instance_name);
                Ok(Some(GameEnded {
                    exit_code: None,
                    seconds,
                    killed,
                }))
            }
        }
    }

    pub fn kill(&self, instance_name: &str) -> Result<bool, CmdError> {
        let mut map = self.running.lock().map_err(|_| poisoned("running"))?;
        match map.get_mut(instance_name) {
            Some(game) => {
                game.killed = true;
                let _ = game.process.child_mut().start_kill();
                Ok(true)
            }
            None => Ok(false),
        }
    }

    pub fn push_log(
        &self,
        instance_name: &str,
        kind: &'static str,
        line: String,
    ) -> Result<GameLogLine, CmdError> {
        let mut map = self.logs.lock().map_err(|_| poisoned("logs"))?;
        let buffer = map.entry(instance_name.to_string()).or_default();

        buffer.next_seq += 1;
        let entry = GameLogLine {
            seq: buffer.next_seq,
            kind,
            line,
        };

        buffer.lines.push_back(entry.clone());
        while buffer.lines.len() > MAX_LOG_LINES {
            buffer.lines.pop_front();
        }
        Ok(entry)
    }

    pub fn game_log(&self, instance_name: &str) -> Result<Vec<GameLogLine>, CmdError> {
        let map = self.logs.lock().map_err(|_| poisoned("logs"))?;
        Ok(map
            .get(instance_name)
            .map(|buffer| buffer.lines.iter().cloned().collect())
            .unwrap_or_default())
    }

    pub fn clear_log(&self, instance_name: &str) -> Result<(), CmdError> {
        let mut map = self.logs.lock().map_err(|_| poisoned("logs"))?;
        map.remove(instance_name);
        Ok(())
    }

    pub fn put_login_session(&self, id: &str, resp: MsDeviceCodeResponse) -> Result<(), CmdError> {
        let mut map = self
            .login_sessions
            .lock()
            .map_err(|_| poisoned("loginSessions"))?;
        map.insert(id.to_string(), resp);
        Ok(())
    }

    pub fn take_login_session(&self, id: &str) -> Result<Option<MsDeviceCodeResponse>, CmdError> {
        let mut map = self
            .login_sessions
            .lock()
            .map_err(|_| poisoned("loginSessions"))?;
        Ok(map.remove(id))
    }

    pub fn set_mod_updates(
        &self,
        instance_name: &str,
        updates: Vec<ModUpdate>,
    ) -> Result<Vec<(String, ModUpdate)>, CmdError> {
        let mut map = self
            .mod_updates
            .lock()
            .map_err(|_| poisoned("modUpdates"))?;
        let prefix = format!("{}::", instance_name);
        map.retain(|k, _| !k.starts_with(&prefix));

        let mut result = Vec::with_capacity(updates.len());
        for update in updates {
            let id = format!("{}{}", prefix, uuid::Uuid::new_v4());
            map.insert(id.clone(), update.clone());
            result.push((id, update));
        }
        Ok(result)
    }

    pub fn get_mod_update(&self, id: &str) -> Result<Option<ModUpdate>, CmdError> {
        let map = self
            .mod_updates
            .lock()
            .map_err(|_| poisoned("modUpdates"))?;
        Ok(map.get(id).cloned())
    }

    pub fn set_crash(
        &self,
        instance_name: &str,
        crash: Option<CrashAnalysis>,
    ) -> Result<(), CmdError> {
        let mut map = self.crashes.lock().map_err(|_| poisoned("crashes"))?;
        match crash {
            Some(crash) => map.insert(instance_name.to_string(), crash),
            None => map.remove(instance_name),
        };
        Ok(())
    }

    pub fn crash(&self, instance_name: &str) -> Result<Option<CrashAnalysis>, CmdError> {
        let map = self.crashes.lock().map_err(|_| poisoned("crashes"))?;
        Ok(map.get(instance_name).cloned())
    }

    pub fn remove_mod_update(&self, id: &str) -> Result<(), CmdError> {
        let mut map = self
            .mod_updates
            .lock()
            .map_err(|_| poisoned("modUpdates"))?;
        map.remove(id);
        Ok(())
    }
}
