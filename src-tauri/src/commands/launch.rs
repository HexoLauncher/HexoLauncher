use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use hexomc_lib::{launch_with_output, LaunchOptions, OutputFn, OutputKind, OutputLine};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

use super::instance::{load_config, record_play};
use super::java::{java_not_found, resolve_instance_java, JavaDto};
use super::{
    instance_dir, minecraft_dir, path_string, EVENT_GAME_EXIT, EVENT_GAME_OUTPUT, EVENT_GAME_START,
};
use crate::error::{CmdError, CmdResult};
use crate::state::{AppState, GameLogLine, RunningInfo};
use crate::store::instance_meta::{InstanceMeta, InstanceSettings, LocalStore};
use crate::store::launcher::LauncherConfig;
use crate::store::{accounts, now_secs};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequest {
    pub base_dir: String,
    pub instance_name: String,

    pub account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaCheck {
    pub required: u32,
    pub java: Option<JavaDto>,
}

#[tauri::command]
pub async fn check_instance_java(base_dir: String, instance_name: String) -> CmdResult<JavaCheck> {
    let base = Path::new(&base_dir);
    let config = load_config(base, &instance_name).await?;
    let launcher = LauncherConfig::load().await?;
    let local = LocalStore::load().await?.get(base, &instance_name);

    Ok(JavaCheck {
        required: config.java_version,
        java: resolve_instance_java(config.java_version, &local.java_path, &launcher.java.path)
            .await?
            .map(JavaDto::from),
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchInfo {
    pub instance_name: String,
    pub pid: u32,
    pub java_path: String,
    pub java_version: u32,
    pub player_name: String,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameOutput {
    pub instance_name: String,
    #[serde(flatten)]
    pub line: GameLogLine,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameExit {
    pub instance_name: String,
    pub exit_code: Option<i32>,
    pub seconds: u64,
    pub crashed: bool,
}

#[tauri::command]
pub async fn launch_instance(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    options: LaunchRequest,
) -> CmdResult<LaunchInfo> {
    let LaunchRequest {
        base_dir,
        instance_name,
        account_id,
    } = options;

    if state.is_running(&instance_name)? {
        return Err(CmdError::new(
            "alreadyRunning",
            format!("instance {} 已經在執行中", instance_name),
        ));
    }

    let base = Path::new(&base_dir);
    let dir = instance_dir(base, &instance_name)?;
    let config = load_config(base, &instance_name).await?;

    let sync_guard = state.schematic_sync.lock().await;
    let launcher = LauncherConfig::load().await?;
    let _ = super::schematic_sync::apply(base, &instance_name, launcher.schematics.sync).await;
    let meta = InstanceMeta::load(&dir, &instance_name).await?;
    let local = LocalStore::load().await?.get(base, &instance_name);
    let account = accounts::resolve(account_id.as_deref()).await?;

    let java = resolve_instance_java(config.java_version, &local.java_path, &launcher.java.path)
        .await?
        .ok_or_else(|| java_not_found(config.java_version))?;

    let hook_vars = HookVars {
        instance_name: instance_name.clone(),
        display_name: meta.name.clone(),
        instance_dir: path_string(&dir),
        minecraft_dir: path_string(&minecraft_dir(base, &instance_name)?),
        java_path: path_string(&java.path),
    };

    let commands = effective_commands(&launcher, &meta.settings);
    if !commands.0.is_empty() {
        run_hook(&commands.0, &hook_vars, "啟動前指令").await?;
    }

    let online = account.access_token.is_some();
    let player_name = account.player_name.clone();
    let options = LaunchOptions {
        instance_id: instance_name.clone(),
        java_path: windowless_java(&java.path),
        player_name: account.player_name,
        auth_token: account.access_token,
        auth_uuid: account.uuid,
        xuid: account.xuid,
        jvm_extra_args: jvm_args(&launcher, &meta.settings),
    };

    state.clear_log(&instance_name)?;

    let process = launch_with_output(&options, base, output_sink(&app, &instance_name)).await?;
    let pid = process.id().unwrap_or_default();

    let info = RunningInfo {
        instance_name: instance_name.clone(),
        pid,
        started_at: now_secs(),
        player_name: player_name.clone(),
        online,
    };
    let info_started_at = info.started_at;
    state.add_running(process, info.clone())?;
    drop(sync_guard);

    let _ = app.emit(EVENT_GAME_START, info);

    record_play(base, &instance_name, 0, true).await;
    let _ = app.emit(super::instance::EVENT_INSTANCES_CHANGED, ());

    watch_process(
        app,
        Watch {
            instance_name: instance_name.clone(),
            started_at: info_started_at,
            base_dir: base.to_path_buf(),
            record_play_time: launcher.game.record_play_time,
            post_exit: commands.1,
            hook_vars,
        },
    );

    Ok(LaunchInfo {
        instance_name,
        pid,
        java_path: path_string(&java.path),
        java_version: java.version,
        player_name,
        online,
    })
}

/// GUI 程式沒有主控台可以繼承，Windows 會幫主控台程式另外開一個視窗。
/// `javaw.exe` 是同一個 JVM 的無視窗版本，輸出導向管線一樣收得到，
/// 所以啟動遊戲時改用它，才不會多跳一個 cmd 視窗出來。
#[cfg(windows)]
fn windowless_java(path: &Path) -> PathBuf {
    let is_java = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("java.exe"));

    if !is_java {
        return path.to_path_buf();
    }
    let javaw = path.with_file_name("javaw.exe");
    if javaw.is_file() {
        javaw
    } else {
        path.to_path_buf()
    }
}

#[cfg(not(windows))]
fn windowless_java(path: &Path) -> PathBuf {
    path.to_path_buf()
}

fn jvm_args(launcher: &LauncherConfig, settings: &InstanceSettings) -> Vec<String> {
    let (min, max) = if settings.override_memory {
        (settings.min_memory_mb, settings.max_memory_mb)
    } else {
        (launcher.game.min_memory_mb, launcher.game.max_memory_mb)
    };

    let extra = if settings.override_jvm_args {
        settings.jvm_args.as_str()
    } else {
        launcher.game.jvm_args.as_str()
    };

    let mut args = vec![
        format!("-Xmx{}M", max.max(512)),
        format!("-Xms{}M", min.max(256)),
        "-Dfile.encoding=UTF-8".to_string(),
        "-Dstdout.encoding=UTF-8".to_string(),
        "-Dstderr.encoding=UTF-8".to_string(),
    ];
    args.extend(extra.split_whitespace().map(str::to_string));
    args
}

fn effective_commands(launcher: &LauncherConfig, settings: &InstanceSettings) -> (String, String) {
    if settings.override_commands {
        (settings.pre_launch.clone(), settings.post_exit.clone())
    } else {
        (
            launcher.commands.pre_launch.clone(),
            launcher.commands.post_exit.clone(),
        )
    }
}

#[derive(Debug, Clone)]
struct HookVars {
    instance_name: String,
    display_name: String,
    instance_dir: String,
    minecraft_dir: String,
    java_path: String,
}

fn expand(command: &str, vars: &HookVars) -> String {
    command
        .replace("$INST_ID", &vars.instance_name)
        .replace("$INST_NAME", &vars.display_name)
        .replace("$INST_DIR", &vars.instance_dir)
        .replace("$INST_MC_DIR", &vars.minecraft_dir)
        .replace("$INST_JAVA", &vars.java_path)
}

async fn run_hook(command: &str, vars: &HookVars, label: &str) -> CmdResult<()> {
    let line = expand(command, vars);
    let mut child = if cfg!(windows) {
        let mut cmd = tokio::process::Command::new("cmd");
        cmd.args(["/C", &line]);
        cmd
    } else {
        let mut cmd = tokio::process::Command::new("sh");
        cmd.args(["-c", &line]);
        cmd
    };
    child.current_dir(&vars.minecraft_dir);
    #[cfg(windows)]
    child.creation_flags(CREATE_NO_WINDOW);

    let status = child
        .status()
        .await
        .map_err(|e| CmdError::launcher(format!("{}執行失敗: {}", label, e)))?;

    if !status.success() {
        return Err(CmdError::launcher(format!(
            "{}以錯誤結束（{}）: {}",
            label,
            status.code().unwrap_or(-1),
            line
        )));
    }
    Ok(())
}

#[tauri::command]
pub fn stop_instance(state: tauri::State<'_, AppState>, instance_name: String) -> CmdResult<bool> {
    state.kill(&instance_name)
}

#[tauri::command]
pub fn running_instances(state: tauri::State<'_, AppState>) -> CmdResult<Vec<RunningInfo>> {
    state.running_list()
}

#[tauri::command]
pub fn game_log(
    state: tauri::State<'_, AppState>,
    instance_name: String,
) -> CmdResult<Vec<GameLogLine>> {
    state.game_log(&instance_name)
}

#[tauri::command]
pub fn clear_game_log(state: tauri::State<'_, AppState>, instance_name: String) -> CmdResult<()> {
    state.clear_log(&instance_name)
}

fn output_sink(app: &tauri::AppHandle, instance_name: &str) -> OutputFn {
    let app = app.clone();
    let instance_name = instance_name.to_string();

    Arc::new(move |OutputLine { kind, line }| {
        let state = app.state::<AppState>();
        let kind = match kind {
            OutputKind::Stdout => "stdout",
            OutputKind::Stderr => "stderr",
        };

        if let Ok(entry) = state.push_log(&instance_name, kind, line) {
            let _ = app.emit(
                EVENT_GAME_OUTPUT,
                GameOutput {
                    instance_name: instance_name.clone(),
                    line: entry,
                },
            );
        }
    })
}

struct Watch {
    instance_name: String,
    started_at: u64,
    base_dir: PathBuf,
    record_play_time: bool,
    post_exit: String,
    hook_vars: HookVars,
}

fn watch_process(app: tauri::AppHandle, watch: Watch) {
    tauri::async_runtime::spawn(async move {
        let ended = loop {
            tokio::time::sleep(Duration::from_millis(800)).await;

            let state = app.state::<AppState>();
            match state.reap(&watch.instance_name) {
                Ok(Some(ended)) => break ended,
                Ok(None) => continue,
                Err(_) => return,
            }
        };

        if watch.record_play_time {
            record_play(&watch.base_dir, &watch.instance_name, ended.seconds, false).await;
        }

        let crash = if ended.killed {
            None
        } else {
            let state = app.state::<AppState>();
            super::crash::analyze(
                &state,
                &watch.base_dir,
                &watch.instance_name,
                watch.started_at,
                ended.exit_code,
            )
            .await
        };
        let crashed = crash.is_some();
        let _ = app.state::<AppState>().set_crash(&watch.instance_name, crash);
        super::schematic_sync::sync_after_exit(&app.state::<AppState>(), &watch.base_dir, &watch.instance_name).await;

        let _ = app.emit(
            EVENT_GAME_EXIT,
            GameExit {
                instance_name: watch.instance_name.clone(),
                exit_code: ended.exit_code,
                seconds: ended.seconds,
                crashed,
            },
        );
        let _ = app.emit(super::instance::EVENT_INSTANCES_CHANGED, ());

        if !watch.post_exit.is_empty() {
            let _ = run_hook(&watch.post_exit, &watch.hook_vars, "結束後指令").await;
        }
    });
}
