use std::path::Path;
use std::time::Duration;

use hexomc_lib::{launch, LaunchOptions};
use serde::{Deserialize, Serialize};
use tauri::Manager;

use super::instance::load_config;
use super::java::resolve_java;
use super::{path_string, EVENT_GAME_EXIT};
use crate::error::{CmdError, CmdResult};
use crate::state::AppState;

/// 要用哪個帳號進遊戲。`accessToken` 為 null 就是離線模式，
/// UUID 由 hexomc-lib 依玩家名稱自動生成。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInput {
    pub player_name: String,
    pub access_token: Option<String>,
    pub uuid: Option<String>,
    pub xuid: Option<String>,
}

/// 啟動一次遊戲需要的全部資訊。除了 `baseDir` / `instanceName` / `account`
/// 之外都可省略：Java 自動偵測、記憶體用預設值。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequest {
    /// 啟動器根目錄
    pub base_dir: String,
    pub instance_name: String,
    pub account: AccountInput,
    /// 指定 java 執行檔；不給就自動偵測（必要時下載）
    pub java_path: Option<String>,
    /// 額外的 JVM 參數
    pub jvm_args: Option<Vec<String>>,
    pub max_memory_mb: Option<u32>,
    pub min_memory_mb: Option<u32>,
    /// 找不到合適的 Java 時是否自動下載（預設 true）
    pub auto_download_java: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchInfo {
    pub instance_name: String,
    pub pid: u32,
    pub java_path: String,
    pub java_version: u32,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameExit {
    pub instance_name: String,
    pub exit_code: Option<i32>,
}

/// 啟動遊戲。回傳時遊戲已經是獨立行程，結束時會發出 `hexo://game-exit` 事件。
#[tauri::command]
pub async fn launch_instance(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    options: LaunchRequest,
) -> CmdResult<LaunchInfo> {
    let LaunchRequest {
        base_dir,
        instance_name,
        account,
        java_path,
        jvm_args,
        max_memory_mb,
        min_memory_mb,
        auto_download_java,
    } = options;

    if account.player_name.trim().is_empty() {
        return Err(CmdError::launcher("玩家名稱不可為空"));
    }
    if state.is_running(&instance_name)? {
        return Err(CmdError::new(
            "alreadyRunning",
            format!("instance {} 已經在執行中", instance_name),
        ));
    }

    let base = Path::new(&base_dir);
    let config = load_config(base, &instance_name).await?;

    let java = resolve_java(
        base,
        config.java_version,
        java_path.as_deref(),
        auto_download_java.unwrap_or(true),
    )
    .await?;

    let mut jvm_extra_args = vec![
        format!("-Xmx{}M", max_memory_mb.unwrap_or(4096)),
        format!("-Xms{}M", min_memory_mb.unwrap_or(512)),
        // 中文玩家名 / 路徑在非 UTF-8 預設編碼下會變亂碼。
        "-Dfile.encoding=UTF-8".to_string(),
        "-Dstdout.encoding=UTF-8".to_string(),
        "-Dstderr.encoding=UTF-8".to_string(),
    ];
    jvm_extra_args.extend(jvm_args.unwrap_or_default());

    let online = account.access_token.is_some();
    let options = LaunchOptions {
        instance_id: instance_name.clone(),
        java_path: java.path.clone(),
        player_name: account.player_name,
        auth_token: account.access_token,
        auth_uuid: account.uuid,
        xuid: account.xuid,
        jvm_extra_args,
    };

    let child = launch(&options, base).await?;
    let pid = child.id();
    state.add_running(&instance_name, child)?;
    watch_process(app, instance_name.clone());

    Ok(LaunchInfo {
        instance_name,
        pid,
        java_path: path_string(&java.path),
        java_version: java.version,
        online,
    })
}

/// 強制結束遊戲。回傳 false 表示本來就沒在跑。
#[tauri::command]
pub fn stop_instance(state: tauri::State<'_, AppState>, instance_name: String) -> CmdResult<bool> {
    state.kill(&instance_name)
}

/// 目前正在執行的 instance 名稱。
#[tauri::command]
pub fn running_instances(state: tauri::State<'_, AppState>) -> CmdResult<Vec<String>> {
    state.running_names()
}

/// 背景輪詢遊戲行程；結束後發事件通知前端。
///
/// 用獨立 thread 而不是 `child.wait()`：wait 需要 `&mut Child`，而 Child 存在
/// state 裡才能被 `stop_instance` 中止，所以改成定期 `try_wait`。
fn watch_process(app: tauri::AppHandle, instance_name: String) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(800));

        let state = app.state::<AppState>();
        match state.reap(&instance_name) {
            Ok(Some(exit_code)) => {
                let _ = app.emit_all(
                    EVENT_GAME_EXIT,
                    GameExit {
                        instance_name,
                        exit_code,
                    },
                );
                break;
            }
            Ok(None) => continue,
            Err(_) => break,
        }
    });
}
