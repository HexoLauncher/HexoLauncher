use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use hexomc_lib::ProgressFn;
use serde::Serialize;
use tauri::Emitter;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use super::instance::EVENT_INSTANCES_CHANGED;
use super::{instance_dir, minecraft_dir, validate_file_name, validate_instance_name};
use crate::error::{CmdError, CmdResult};
use crate::state::AppState;

const MINECRAFT_ENTRIES: &[&str] = &[
    "mods",
    "config",
    "defaultconfigs",
    "kubejs",
    "scripts",
    "options.txt",
];
const MINECRAFT_PREFIX: &str = "minecraft/";
const INSTANCE_PREFIX: &str = "instance/";
const AUTO_KEEP: usize = 10;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupDto {
    pub file_name: String,
    pub reason: String,
    pub created_at: String,
    pub size: u64,
}

pub fn backups_dir(base: &Path, instance_name: &str) -> CmdResult<PathBuf> {
    validate_instance_name(instance_name)?;
    Ok(base.join("backups").join(instance_name))
}

fn validate_reason(reason: &str) -> CmdResult<()> {
    let ok = !reason.is_empty()
        && reason
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-');
    if ok {
        Ok(())
    } else {
        Err(CmdError::launcher(format!("備份原因不合法: {reason}")))
    }
}

fn add_file<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    path: &Path,
    name: &str,
) -> CmdResult<()> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let method = if matches!(ext.as_str(), "jar" | "zip" | "disabled") {
        CompressionMethod::Stored
    } else {
        CompressionMethod::Deflated
    };
    let options = SimpleFileOptions::default().compression_method(method);
    zip.start_file(name, options).map_err(zip_err)?;
    let mut file = std::fs::File::open(path)?;
    std::io::copy(&mut file, zip)?;
    Ok(())
}

fn collect(path: &Path, name: &str, out: &mut Vec<(PathBuf, String)>) -> CmdResult<()> {
    let meta = std::fs::symlink_metadata(path)?;
    if meta.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let child = format!("{}/{}", name, entry.file_name().to_string_lossy());
            collect(&entry.path(), &child, out)?;
        }
    } else if meta.is_file() {
        out.push((path.to_path_buf(), name.to_string()));
    }
    Ok(())
}

fn zip_err(e: zip::result::ZipError) -> CmdError {
    CmdError::new("zip", e.to_string())
}

fn write_backup(
    instance: &Path,
    mc: &Path,
    dest: &Path,
    progress: Option<&ProgressFn>,
) -> CmdResult<()> {
    let mut files = Vec::new();
    for entry in MINECRAFT_ENTRIES {
        let path = mc.join(entry);
        if path.exists() {
            collect(&path, &format!("{MINECRAFT_PREFIX}{entry}"), &mut files)?;
        }
    }
    for entry in std::fs::read_dir(instance)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_file() && name.to_ascii_lowercase().ends_with(".json") {
            files.push((path, format!("{INSTANCE_PREFIX}{name}")));
        }
    }

    let file = std::fs::File::create(dest)?;
    let mut zip = zip::ZipWriter::new(std::io::BufWriter::new(file));
    let total = files.len();
    for (index, (path, name)) in files.iter().enumerate() {
        if let Some(report) = progress {
            report(index, total, "正在備份目前的模組與設定");
        }
        add_file(&mut zip, path, name)?;
    }
    zip.finish().map_err(zip_err)?;
    if let Some(report) = progress {
        report(total, total, "正在備份目前的模組與設定");
    }
    Ok(())
}

fn prune(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut auto: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".zip") && !n.ends_with("_manual.zip"))
        .collect();
    auto.sort();
    let excess = auto.len().saturating_sub(AUTO_KEEP);
    for name in auto.into_iter().take(excess) {
        let _ = std::fs::remove_file(dir.join(name));
    }
}

pub async fn create(base: &Path, instance_name: &str, reason: &str) -> CmdResult<String> {
    create_with_progress(base, instance_name, reason, None).await
}

pub async fn create_with_progress(
    base: &Path,
    instance_name: &str,
    reason: &str,
    progress: Option<ProgressFn>,
) -> CmdResult<String> {
    validate_reason(reason)?;
    let instance = instance_dir(base, instance_name)?;
    let mc = minecraft_dir(base, instance_name)?;
    let dir = backups_dir(base, instance_name)?;
    let dest = dir.join(format!("{}_{}.zip", super::tools::timestamp(), reason));
    let task_dest = dest.clone();
    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(task_dest.parent().unwrap_or(Path::new(".")))?;
        let result = write_backup(&instance, &mc, &task_dest, progress.as_ref());
        if result.is_err() {
            let _ = std::fs::remove_file(&task_dest);
        }
        result
    })
    .await
    .map_err(|e| CmdError::launcher(e.to_string()))??;
    prune(&dir);
    Ok(dest
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default())
}

fn parse_name(file_name: &str) -> Option<(String, String)> {
    let stem = file_name.strip_suffix(".zip")?;
    let (stamp, reason) = stem.split_once('_').and_then(|(date, rest)| {
        let (time, reason) = rest.split_once('_')?;
        Some((format!("{date}_{time}"), reason.to_string()))
    })?;
    Some((stamp, reason))
}

fn guard_not_running(state: &AppState, instance_name: &str) -> CmdResult<()> {
    if state.is_running(instance_name)? {
        return Err(CmdError::launcher("遊戲正在執行，請先關閉遊戲"));
    }
    Ok(())
}

#[tauri::command]
pub async fn create_backup(
    app: tauri::AppHandle,
    base_dir: String,
    instance_name: String,
    reason: Option<String>,
) -> CmdResult<String> {
    let progress = super::install::progress_emitter(&app, &instance_name);
    create_with_progress(
        Path::new(&base_dir),
        &instance_name,
        reason.as_deref().unwrap_or("manual"),
        Some(progress),
    )
    .await
}

#[tauri::command]
pub async fn list_backups(base_dir: String, instance_name: String) -> CmdResult<Vec<BackupDto>> {
    let dir = backups_dir(Path::new(&base_dir), &instance_name)?;
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(Vec::new());
    };
    let mut list: Vec<BackupDto> = entries
        .flatten()
        .filter_map(|e| {
            let file_name = e.file_name().to_string_lossy().to_string();
            let (created_at, reason) = parse_name(&file_name)?;
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            Some(BackupDto {
                file_name,
                reason,
                created_at,
                size,
            })
        })
        .collect();
    list.sort_by(|a, b| b.file_name.cmp(&a.file_name));
    Ok(list)
}

#[tauri::command]
pub async fn delete_backup(
    base_dir: String,
    instance_name: String,
    file_name: String,
) -> CmdResult<()> {
    validate_file_name(&file_name)?;
    let path = backups_dir(Path::new(&base_dir), &instance_name)?.join(&file_name);
    if !path.is_file() {
        return Err(CmdError::launcher(format!("找不到備份: {file_name}")));
    }
    tokio::fs::remove_file(path).await?;
    Ok(())
}

fn extract(archive: &Path, instance: &Path, mc: &Path) -> CmdResult<()> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(file)).map_err(zip_err)?;

    for entry in MINECRAFT_ENTRIES {
        let path = mc.join(entry);
        if let Ok(meta) = std::fs::symlink_metadata(&path) {
            if meta.is_dir() {
                std::fs::remove_dir_all(&path)?;
            } else {
                std::fs::remove_file(&path)?;
            }
        }
    }

    for i in 0..zip.len() {
        let mut item = zip.by_index(i).map_err(zip_err)?;
        let Some(enclosed) = item.enclosed_name() else {
            continue;
        };
        let name = enclosed.to_string_lossy().replace('\\', "/");
        let target = if let Some(rest) = name.strip_prefix(MINECRAFT_PREFIX) {
            mc.join(rest)
        } else if let Some(rest) = name.strip_prefix(INSTANCE_PREFIX) {
            instance.join(rest)
        } else {
            continue;
        };
        if item.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::File::create(&target)?;
        let mut buf = Vec::new();
        item.read_to_end(&mut buf)?;
        out.write_all(&buf)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn restore_backup(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    base_dir: String,
    instance_name: String,
    file_name: String,
) -> CmdResult<String> {
    validate_file_name(&file_name)?;
    guard_not_running(&state, &instance_name)?;
    let base = Path::new(&base_dir);
    let archive = backups_dir(base, &instance_name)?.join(&file_name);
    if !archive.is_file() {
        return Err(CmdError::launcher(format!("找不到備份: {file_name}")));
    }
    super::modpack::ensure_not_linked(base, &instance_name).await?;

    let safety = create(base, &instance_name, "pre-restore").await?;

    let instance = instance_dir(base, &instance_name)?;
    let mc = minecraft_dir(base, &instance_name)?;
    tokio::task::spawn_blocking(move || extract(&archive, &instance, &mc))
        .await
        .map_err(|e| CmdError::launcher(e.to_string()))??;

    let _ = app.emit(EVENT_INSTANCES_CHANGED, ());
    Ok(safety)
}
