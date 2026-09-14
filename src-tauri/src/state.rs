use std::collections::HashMap;
use std::sync::Mutex;

use hexomc_lib::auth::microsoft::MsDeviceCodeResponse;
use hexomc_lib::{GameProcess, ModUpdate};

use crate::error::CmdError;

#[derive(Default)]
pub struct AppState {
    running: Mutex<HashMap<String, GameProcess>>,
    login_sessions: Mutex<HashMap<String, MsDeviceCodeResponse>>,
    mod_updates: Mutex<HashMap<String, ModUpdate>>,
}

fn poisoned(what: &str) -> CmdError {
    CmdError::launcher(format!("內部狀態鎖毀損: {}", what))
}

impl AppState {
    pub fn add_running(&self, instance_name: &str, process: GameProcess) -> Result<(), CmdError> {
        let mut map = self.running.lock().map_err(|_| poisoned("running"))?;
        map.insert(instance_name.to_string(), process);
        Ok(())
    }

    pub fn is_running(&self, instance_name: &str) -> Result<bool, CmdError> {
        let map = self.running.lock().map_err(|_| poisoned("running"))?;
        Ok(map.contains_key(instance_name))
    }

    pub fn running_names(&self) -> Result<Vec<String>, CmdError> {
        let mut map = self.running.lock().map_err(|_| poisoned("running"))?;
        map.retain(|_, process| !matches!(process.try_wait(), Ok(Some(_))));
        Ok(map.keys().cloned().collect())
    }

    pub fn reap(&self, instance_name: &str) -> Result<Option<Option<i32>>, CmdError> {
        let mut map = self.running.lock().map_err(|_| poisoned("running"))?;
        let Some(process) = map.get_mut(instance_name) else {
            // 已經被別人收走。
            return Ok(Some(None));
        };
        match process.try_wait() {
            Ok(Some(status)) => {
                map.remove(instance_name);
                Ok(Some(status.code()))
            }
            Ok(None) => Ok(None),
            Err(_) => {
                map.remove(instance_name);
                Ok(Some(None))
            }
        }
    }

    /// 要求結束遊戲
    pub fn kill(&self, instance_name: &str) -> Result<bool, CmdError> {
        let mut map = self.running.lock().map_err(|_| poisoned("running"))?;
        match map.get_mut(instance_name) {
            Some(process) => {
                let _ = process.child_mut().start_kill();
                Ok(true)
            }
            None => Ok(false),
        }
    }

    pub fn put_login_session(&self, id: &str, resp: MsDeviceCodeResponse) -> Result<(), CmdError> {
        let mut map = self
            .login_sessions
            .lock()
            .map_err(|_| poisoned("loginSessions"))?;
        map.insert(id.to_string(), resp);
        Ok(())
    }

    /// 取出並移除登入 session
    pub fn take_login_session(&self, id: &str) -> Result<Option<MsDeviceCodeResponse>, CmdError> {
        let mut map = self
            .login_sessions
            .lock()
            .map_err(|_| poisoned("loginSessions"))?;
        Ok(map.remove(id))
    }

    /// 覆寫某個 instance 的更新清單，回傳 (id, ModUpdate 參照) 供組 DTO。
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

    pub fn remove_mod_update(&self, id: &str) -> Result<(), CmdError> {
        let mut map = self
            .mod_updates
            .lock()
            .map_err(|_| poisoned("modUpdates"))?;
        map.remove(id);
        Ok(())
    }
}
