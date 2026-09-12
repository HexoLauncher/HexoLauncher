use std::collections::HashMap;
use std::process::Child;
use std::sync::Mutex;

use hexomc_lib::auth::microsoft::MsDeviceCodeResponse;
use hexomc_lib::ModUpdate;

use crate::error::CmdError;

/// 啟動器的執行期狀態。
///
/// 這裡只放「無法序列化給前端、但需要跨 command 存活」的東西：
/// 已啟動的遊戲行程、Microsoft 裝置碼登入 session、以及檢查到的模組更新
/// （`ModUpdate` 內含 Modrinth / CurseForge 的原始資料，前端只拿到 id）。
///
/// 資料夾（base_dir）刻意不存在這裡 —— 由前端每次呼叫時傳入。
#[derive(Default)]
pub struct AppState {
    running: Mutex<HashMap<String, Child>>,
    login_sessions: Mutex<HashMap<String, MsDeviceCodeResponse>>,
    mod_updates: Mutex<HashMap<String, ModUpdate>>,
}

fn poisoned(what: &str) -> CmdError {
    CmdError::launcher(format!("內部狀態鎖毀損: {}", what))
}

impl AppState {
    // ---- 遊戲行程 ----

    pub fn add_running(&self, instance_name: &str, child: Child) -> Result<(), CmdError> {
        let mut map = self.running.lock().map_err(|_| poisoned("running"))?;
        map.insert(instance_name.to_string(), child);
        Ok(())
    }

    pub fn is_running(&self, instance_name: &str) -> Result<bool, CmdError> {
        let map = self.running.lock().map_err(|_| poisoned("running"))?;
        Ok(map.contains_key(instance_name))
    }

    /// 回傳目前仍在執行的 instance 名稱，順便清掉已結束的項目。
    pub fn running_names(&self) -> Result<Vec<String>, CmdError> {
        let mut map = self.running.lock().map_err(|_| poisoned("running"))?;
        map.retain(|_, child| !matches!(child.try_wait(), Ok(Some(_))));
        Ok(map.keys().cloned().collect())
    }

    /// 檢查行程是否已結束；結束的話移出 map 並回傳退出碼。
    pub fn reap(&self, instance_name: &str) -> Result<Option<Option<i32>>, CmdError> {
        let mut map = self.running.lock().map_err(|_| poisoned("running"))?;
        let Some(child) = map.get_mut(instance_name) else {
            // 已經被別人收走（例如 stop_instance）。
            return Ok(Some(None));
        };
        match child.try_wait() {
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

    pub fn kill(&self, instance_name: &str) -> Result<bool, CmdError> {
        let mut map = self.running.lock().map_err(|_| poisoned("running"))?;
        match map.remove(instance_name) {
            Some(mut child) => {
                let _ = child.kill();
                let _ = child.wait();
                Ok(true)
            }
            None => Ok(false),
        }
    }

    // ---- Microsoft 登入 session ----

    pub fn put_login_session(&self, id: &str, resp: MsDeviceCodeResponse) -> Result<(), CmdError> {
        let mut map = self
            .login_sessions
            .lock()
            .map_err(|_| poisoned("loginSessions"))?;
        map.insert(id.to_string(), resp);
        Ok(())
    }

    /// 取出（並移除）登入 session。取出所有權是刻意的：輪詢會 await 數分鐘，
    /// 不能把 std Mutex 的 guard 帶過 await。
    pub fn take_login_session(&self, id: &str) -> Result<Option<MsDeviceCodeResponse>, CmdError> {
        let mut map = self
            .login_sessions
            .lock()
            .map_err(|_| poisoned("loginSessions"))?;
        Ok(map.remove(id))
    }

    // ---- 模組更新 ----

    /// 覆寫某個 instance 的更新清單，回傳 (id, ModUpdate 參照) 供組 DTO。
    pub fn set_mod_updates(
        &self,
        instance_name: &str,
        updates: Vec<ModUpdate>,
    ) -> Result<Vec<(String, ModUpdate)>, CmdError> {
        let mut map = self.mod_updates.lock().map_err(|_| poisoned("modUpdates"))?;
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
        let map = self.mod_updates.lock().map_err(|_| poisoned("modUpdates"))?;
        Ok(map.get(id).cloned())
    }

    pub fn remove_mod_update(&self, id: &str) -> Result<(), CmdError> {
        let mut map = self.mod_updates.lock().map_err(|_| poisoned("modUpdates"))?;
        map.remove(id);
        Ok(())
    }
}
