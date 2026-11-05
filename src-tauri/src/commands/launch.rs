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
use crate::store::running::{self, HookVars, RunningRecord};
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
    pub lines: Vec<GameLogLine>,
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
        jvm_extra_args: jvm_args(&launcher, &meta.settings, java.version),
    };

    state.clear_log(&instance_name)?;

    let process = launch_with_output(&options, base, output_sink(&app, &instance_name)).await?;
    let pid = process.id().unwrap_or_default();
    let started_at = now_secs();

    let info = RunningInfo {
        instance_name: instance_name.clone(),
        pid,
        started_at,
        player_name: player_name.clone(),
        online,
    };
    let watch = Watch {
        instance_name: instance_name.clone(),
        started_at,
        base_dir: base.to_path_buf(),
        record_play_time: launcher.game.record_play_time,
        post_exit: commands.1,
        hook_vars,
    };
    let record = RunningRecord {
        instance_name: instance_name.clone(),
        base_dir: base_dir.clone(),
        pid,
        started_at,
        proc_start: super::process::start_time(pid).unwrap_or(started_at),
        player_name: player_name.clone(),
        online,
        record_play_time: watch.record_play_time,
        post_exit: watch.post_exit.clone(),
        hook_vars: watch.hook_vars.clone(),
    };
    state.add_running(process, info.clone(), record)?;
    drop(sync_guard);
    let _ = running::persist(&state).await;

    let _ = app.emit(EVENT_GAME_START, info);

    record_play(base, &instance_name, 0, true).await;
    let _ = app.emit(super::instance::EVENT_INSTANCES_CHANGED, ());

    watch_process(app, watch);

    Ok(LaunchInfo {
        instance_name,
        pid,
        java_path: path_string(&java.path),
        java_version: java.version,
        player_name,
        online,
    })
}

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

fn jvm_args(
    launcher: &LauncherConfig,
    settings: &InstanceSettings,
    java_version: u32,
) -> Vec<String> {
    let (min, max) = if settings.override_memory {
        (settings.min_memory_mb, settings.max_memory_mb)
    } else {
        (launcher.game.min_memory_mb, launcher.game.max_memory_mb)
    };
    let max = max.max(512);

    let extra = if settings.override_jvm_args {
        settings.jvm_args.as_str()
    } else {
        launcher.game.jvm_args.as_str()
    };

    let mut args = vec![
        format!("-Xmx{}M", max),
        format!("-Xms{}M", min.max(256)),
        "-Dfile.encoding=UTF-8".to_string(),
        "-Dstdout.encoding=UTF-8".to_string(),
        "-Dstderr.encoding=UTF-8".to_string(),
    ];
    if launcher.game.optimize_jvm {
        let zgc = max >= ZGC_MIN_HEAP_MB && total_memory_mb() >= ZGC_MIN_SYSTEM_MB;
        args.extend(tuned_jvm_args(java_version, zgc, extra));
    }
    args.extend(extra.split_whitespace().map(str::to_string));
    args
}

const ZGC_MIN_HEAP_MB: u32 = 4096;
const ZGC_MIN_SYSTEM_MB: u64 = 15 * 1024;

fn total_memory_mb() -> u64 {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    system.total_memory() / 1024 / 1024
}

fn tuned_jvm_args(java_version: u32, zgc: bool, user_args: &str) -> Vec<String> {
    let has = |needle: &str| user_args.split_whitespace().any(|arg| arg.contains(needle));
    let user_picked_gc = user_args
        .split_whitespace()
        .any(|arg| arg.starts_with("-XX:+Use") && arg.ends_with("GC"));

    let mut args = Vec::new();
    if !user_picked_gc {
        match java_version {
            8 => args.push("-XX:+UseG1GC"),
            21 | 22 if zgc => args.extend(["-XX:+UseZGC", "-XX:+ZGenerational"]),
            v if v >= 23 && zgc => args.push("-XX:+UseZGC"),
            _ => {}
        }
    }
    if java_version >= 25 && !has("CompactObjectHeaders") {
        args.push("-XX:+UseCompactObjectHeaders");
    }
    args.into_iter().map(str::to_string).collect()
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

        let _ = state.push_log(&instance_name, kind, line);
    })
}

struct OutputStream {
    instance_name: String,
    last_seq: u64,
}

impl OutputStream {
    fn flush(&mut self, app: &tauri::AppHandle) {
        let state = app.state::<AppState>();
        let Ok(lines) = state.game_log_since(&self.instance_name, self.last_seq) else {
            return;
        };
        let Some(last) = lines.last() else {
            return;
        };
        self.last_seq = last.seq;
        let _ = app.emit(
            EVENT_GAME_OUTPUT,
            GameOutput {
                instance_name: self.instance_name.clone(),
                lines,
            },
        );
    }
}

struct Watch {
    instance_name: String,
    started_at: u64,
    base_dir: PathBuf,
    record_play_time: bool,
    post_exit: String,
    hook_vars: HookVars,
}

pub async fn recover_running(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    for record in running::load_alive().await {
        let instance_name = record.instance_name.clone();
        if state.is_running(&instance_name).unwrap_or(true) {
            continue;
        }
        let watch = Watch {
            instance_name: instance_name.clone(),
            started_at: record.started_at,
            base_dir: PathBuf::from(&record.base_dir),
            record_play_time: record.record_play_time,
            post_exit: record.post_exit.clone(),
            hook_vars: record.hook_vars.clone(),
        };
        let pid = record.pid;
        if state.adopt_running(record).is_err() {
            continue;
        }
        let _ = state.push_log(
            &instance_name,
            "stdout",
            format!(
                "已重新連結到執行中的遊戲（PID {}），啟動器重新開啟前的輸出無法取得。",
                pid
            ),
        );
        watch_process(app.clone(), watch);
    }
    let _ = running::persist(&state).await;
}

const OUTPUT_FLUSH: Duration = Duration::from_millis(100);
const REAP_EVERY: u32 = 8;

fn watch_process(app: tauri::AppHandle, watch: Watch) {
    tauri::async_runtime::spawn(async move {
        let mut stream = OutputStream {
            instance_name: watch.instance_name.clone(),
            last_seq: 0,
        };
        let mut tick: u32 = 0;
        let ended = loop {
            tokio::time::sleep(OUTPUT_FLUSH).await;
            stream.flush(&app);

            tick = tick.wrapping_add(1);
            if tick % REAP_EVERY != 0 {
                continue;
            }
            let state = app.state::<AppState>();
            match state.reap(&watch.instance_name) {
                Ok(Some(ended)) => break ended,
                Ok(None) => continue,
                Err(_) => return,
            }
        };
        let _ = running::persist(&app.state::<AppState>()).await;

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
        let _ = app
            .state::<AppState>()
            .set_crash(&watch.instance_name, crash);
        super::schematic_sync::sync_after_exit(
            &app.state::<AppState>(),
            &watch.base_dir,
            &watch.instance_name,
        )
        .await;

        stream.flush(&app);
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

        crate::background::quit_if_idle(&app, None);
    });
}

#[cfg(test)]
mod tests {
    use super::tuned_jvm_args;

    #[test]
    fn picks_gc_by_java_version() {
        assert_eq!(tuned_jvm_args(8, true, ""), ["-XX:+UseG1GC"]);
        assert!(tuned_jvm_args(17, true, "").is_empty());
        assert_eq!(
            tuned_jvm_args(21, true, ""),
            ["-XX:+UseZGC", "-XX:+ZGenerational"]
        );
        assert_eq!(tuned_jvm_args(23, true, ""), ["-XX:+UseZGC"]);
        assert_eq!(
            tuned_jvm_args(25, true, ""),
            ["-XX:+UseZGC", "-XX:+UseCompactObjectHeaders"]
        );
    }

    #[test]
    fn falls_back_to_default_gc_on_small_systems() {
        assert!(tuned_jvm_args(21, false, "").is_empty());
        assert_eq!(
            tuned_jvm_args(25, false, ""),
            ["-XX:+UseCompactObjectHeaders"]
        );
    }

    #[test]
    fn respects_user_choices() {
        assert!(tuned_jvm_args(21, true, "-XX:+UseShenandoahGC").is_empty());
        assert_eq!(
            tuned_jvm_args(8, true, "-XX:+UseParallelGC"),
            Vec::<String>::new()
        );
        assert_eq!(
            tuned_jvm_args(25, true, "-XX:+UseG1GC -XX:-UseCompactObjectHeaders"),
            Vec::<String>::new()
        );
    }
}
