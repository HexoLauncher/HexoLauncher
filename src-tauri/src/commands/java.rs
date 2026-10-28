use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use hexomc_lib::{
    download_java_with_progress, pick_java, probe_java_runtime, scan_java as scan_runtimes,
    JavaInfo, JavaRuntime, ProgressFn,
};
use serde::Serialize;
use tauri::Emitter;

use super::{path_string, EVENT_JAVA_PROGRESS};
use crate::error::{CmdError, CmdResult};
use crate::store::launcher::LauncherConfig;

const SCAN_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaDto {
    pub path: String,
    pub version: u32,
}

impl From<JavaInfo> for JavaDto {
    fn from(info: JavaInfo) -> Self {
        Self {
            path: path_string(&info.path),
            version: info.version,
        }
    }
}

#[tauri::command]
pub async fn detect_java(required_version: Option<u32>) -> CmdResult<Option<JavaDto>> {
    Ok(resolve_java(required_version.unwrap_or(21), None)
        .await?
        .map(JavaDto::from))
}

#[tauri::command]
pub async fn detect_java_candidates() -> Vec<JavaRuntime> {
    scan_java(true).await.as_ref().clone()
}

#[tauri::command]
pub async fn test_java(path: String) -> CmdResult<JavaDto> {
    let path = PathBuf::from(&path);
    if !path.is_file() {
        return Err(CmdError::launcher(format!(
            "檔案不存在: {}",
            path.display()
        )));
    }
    probe_java_runtime(&path)
        .await
        .map(|r| JavaDto::from(r.info()))
        .ok_or_else(|| CmdError::launcher("這個檔案不是可用的 java 執行檔"))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaProgress {
    pub version: u32,
    pub done: usize,
    pub total: usize,
    pub message: String,
}

#[tauri::command]
pub async fn install_java(
    app: tauri::AppHandle,
    base_dir: String,
    version: u32,
) -> CmdResult<JavaDto> {
    let path =
        download_java_with_progress(version, Path::new(&base_dir), java_progress(app, version))
            .await?;

    invalidate_java_scan();

    Ok(probe_java_runtime(&path)
        .await
        .map(|r| JavaDto::from(r.info()))
        .unwrap_or(JavaDto {
            path: path_string(&path),
            version,
        }))
}

fn java_progress(app: tauri::AppHandle, version: u32) -> ProgressFn {
    let last: Mutex<(Option<Instant>, String)> = Mutex::new((None, String::new()));
    Arc::new(move |done, total, message| {
        let Ok(mut last) = last.lock() else {
            return;
        };
        let finished = total > 0 && done >= total;
        let recent = last
            .0
            .is_some_and(|at| at.elapsed() < Duration::from_millis(100));
        if last.1 == message && recent && !finished {
            return;
        }
        *last = (Some(Instant::now()), message.to_string());
        let _ = app.emit(
            EVENT_JAVA_PROGRESS,
            JavaProgress {
                version,
                done,
                total,
                message: message.to_string(),
            },
        );
    })
}

#[tauri::command]
pub async fn ensure_java(
    app: tauri::AppHandle,
    base_dir: String,
    required_version: u32,
    allow_download: Option<bool>,
) -> CmdResult<JavaDto> {
    if let Some(info) = resolve_java(required_version, None).await? {
        return Ok(info.into());
    }
    if !allow_download.unwrap_or(false) {
        return Err(java_not_found(required_version));
    }
    install_java(app, base_dir, required_version).await
}

pub async fn resolve_java(
    required_version: u32,
    explicit_path: Option<&str>,
) -> CmdResult<Option<JavaInfo>> {
    if let Some(raw) = explicit_path.filter(|p| !p.is_empty()) {
        let path = PathBuf::from(raw);
        if !path.exists() {
            return Err(CmdError::launcher(format!("指定的 Java 不存在: {}", raw)));
        }

        let info = probe_java_runtime(&path)
            .await
            .map(|r| r.info())
            .unwrap_or(JavaInfo {
                path,
                version: required_version,
            });
        if info.version < required_version {
            return Err(CmdError::new(
                "javaTooOld",
                format!(
                    "指定的 Java {} 版本太舊，這個實例至少需要 Java {}：{}",
                    info.version, required_version, raw
                ),
            ));
        }
        return Ok(Some(info));
    }

    let runtimes = scan_java(false).await;
    Ok(pick_java(&runtimes, required_version).map(JavaRuntime::info))
}

pub async fn resolve_instance_java(
    required_version: u32,
    instance_path: &str,
    default_path: &str,
) -> CmdResult<Option<JavaInfo>> {
    let instance_path = instance_path.trim();
    if !instance_path.is_empty() {
        return resolve_java(required_version, Some(instance_path)).await;
    }

    let default_path = default_path.trim();
    if !default_path.is_empty() {
        match resolve_java(required_version, Some(default_path)).await {
            Err(e) if e.kind == "javaTooOld" => {}
            other => return other,
        }
    }

    resolve_java(required_version, None).await
}

pub fn java_not_found(required_version: u32) -> CmdError {
    CmdError::new("javaNotFound", format!("找不到 Java {}", required_version))
}

pub async fn pick_java_for_tools() -> Option<JavaInfo> {
    let runtimes = scan_java(false).await;
    [21, 17, 8]
        .into_iter()
        .find_map(|version| runtimes.iter().find(|r| r.version == version))
        .map(JavaRuntime::info)
}

static SCAN_CACHE: Mutex<Option<(Instant, Arc<Vec<JavaRuntime>>)>> = Mutex::new(None);

pub fn invalidate_java_scan() {
    if let Ok(mut cache) = SCAN_CACHE.lock() {
        *cache = None;
    }
}

async fn scan_java(refresh: bool) -> Arc<Vec<JavaRuntime>> {
    if !refresh {
        let cached = SCAN_CACHE.lock().ok().and_then(|cache| {
            cache
                .as_ref()
                .filter(|(at, _)| at.elapsed() < SCAN_CACHE_TTL)
                .map(|(_, list)| list.clone())
        });
        if let Some(list) = cached {
            return list;
        }
    }

    let base_dir = LauncherConfig::load()
        .await
        .ok()
        .and_then(|config| config.base_dir().ok());
    let found = Arc::new(scan_runtimes(base_dir.as_deref()).await);
    if let Ok(mut cache) = SCAN_CACHE.lock() {
        *cache = Some((Instant::now(), found.clone()));
    }
    found
}
