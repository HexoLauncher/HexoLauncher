use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use hexomc_lib::{analyze_crash, find_crash_files, CrashCause};
use serde::Serialize;
use tokio::io::AsyncReadExt;

use super::{minecraft_dir, path_string};
use crate::error::{CmdError, CmdResult};
use crate::state::AppState;

const MAX_REPORT_BYTES: u64 = 4 * 1024 * 1024;
const LOG_TAIL: usize = 300;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashAnalysis {
    pub instance_name: String,
    pub exit_code: Option<i32>,
    pub causes: Vec<CrashCause>,
    pub crash_report: Option<String>,
    pub jvm_report: Option<String>,
    pub log_tail: Vec<String>,
}

pub async fn analyze(
    state: &AppState,
    base_dir: &Path,
    instance_name: &str,
    started_at: u64,
    exit_code: Option<i32>,
) -> Option<CrashAnalysis> {
    let since = UNIX_EPOCH + Duration::from_secs(started_at.saturating_sub(5));
    let game_dir = minecraft_dir(base_dir, instance_name).ok()?;
    let files = tokio::task::spawn_blocking(move || find_crash_files(&game_dir, since))
        .await
        .ok()?;

    let failed = matches!(exit_code, Some(code) if code != 0);
    if !failed && files.crash_report.is_none() && files.jvm_report.is_none() {
        return None;
    }

    let lines: Vec<String> = state
        .game_log(instance_name)
        .unwrap_or_default()
        .into_iter()
        .map(|l| l.line)
        .collect();
    let crash_text = read_capped(files.crash_report.as_deref()).await;
    let jvm_text = read_capped(files.jvm_report.as_deref()).await;
    let causes = analyze_crash(&lines.join("\n"), crash_text.as_deref(), jvm_text.as_deref());

    Some(CrashAnalysis {
        instance_name: instance_name.to_string(),
        exit_code,
        causes,
        crash_report: files.crash_report.as_deref().map(path_string),
        jvm_report: files.jvm_report.as_deref().map(path_string),
        log_tail: lines[lines.len().saturating_sub(LOG_TAIL)..].to_vec(),
    })
}

async fn read_capped(path: Option<&Path>) -> Option<String> {
    let file = tokio::fs::File::open(path?).await.ok()?;
    let mut bytes = Vec::new();
    file.take(MAX_REPORT_BYTES)
        .read_to_end(&mut bytes)
        .await
        .ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

#[tauri::command]
pub fn crash_analysis(
    state: tauri::State<'_, AppState>,
    instance_name: String,
) -> CmdResult<Option<CrashAnalysis>> {
    state.crash(&instance_name)
}

#[tauri::command]
pub fn open_crash_file(
    state: tauri::State<'_, AppState>,
    instance_name: String,
    kind: String,
) -> CmdResult<()> {
    let crash = state
        .crash(&instance_name)?
        .ok_or_else(|| CmdError::launcher("沒有這個實例的崩潰紀錄"))?;
    let path = match kind.as_str() {
        "crashReport" => crash.crash_report,
        "jvmReport" => crash.jvm_report,
        other => return Err(CmdError::launcher(format!("未知的檔案: {}", other))),
    }
    .ok_or_else(|| CmdError::launcher("這次崩潰沒有產生這個檔案"))?;
    open::that(PathBuf::from(path)).map_err(|e| CmdError::launcher(format!("無法開啟檔案: {}", e)))
}
