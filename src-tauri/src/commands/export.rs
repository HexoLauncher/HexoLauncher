use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Emitter;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::instance::{build_view, InstanceView, EVENT_INSTANCES_CHANGED};
use super::{instance_dir, path_string};
use crate::error::{CmdError, CmdResult};
use crate::store::instance_meta::{InstanceMeta, LocalStore, META_FILE};

const SKIP_ALWAYS: [&str; 2] = ["instance_config.json", "natives"];

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub base_dir: String,
    pub instance_name: String,

    pub target_path: String,
    pub include_saves: bool,
    pub include_screenshots: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub file_count: usize,
    pub bytes: u64,
}

#[tauri::command]
pub async fn export_instance(request: ExportRequest) -> CmdResult<ExportResult> {
    let ExportRequest {
        base_dir,
        instance_name,
        target_path,
        include_saves,
        include_screenshots,
    } = request;

    let source = instance_dir(Path::new(&base_dir), &instance_name)?;
    if !source.exists() {
        return Err(CmdError::new(
            "instanceNotFound",
            format!("找不到 instance: {}", instance_name),
        ));
    }
    if !target_path.to_ascii_lowercase().ends_with(".zip") {
        return Err(CmdError::launcher("匯出的檔名要以 .zip 結尾"));
    }

    let target = PathBuf::from(&target_path);
    tokio::task::spawn_blocking(move || {
        write_archive(&source, &target, include_saves, include_screenshots)
    })
    .await
    .map_err(|e| CmdError::launcher(format!("匯出中斷: {}", e)))?
}

fn write_archive(
    source: &Path,
    target: &Path,
    include_saves: bool,
    include_screenshots: bool,
) -> CmdResult<ExportResult> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = File::create(target)?;
    let mut zip = ZipWriter::new(BufWriter::new(file));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let mut file_count = 0;
    let mut buffer = Vec::new();

    let mut pending = vec![(source.to_path_buf(), String::new())];
    let mut visited = std::collections::HashSet::new();

    while let Some((dir, prefix)) = pending.pop() {
        if let Ok(real) = std::fs::canonicalize(&dir) {
            if !visited.insert(real) {
                continue;
            }
        }
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = format!("{}{}", prefix, entry.file_name().to_string_lossy());
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            let is_dir = meta.is_dir();

            if skip(&name, is_dir, include_saves, include_screenshots) {
                continue;
            }

            if is_dir {
                zip.add_directory(format!("{}/", name), options)
                    .map_err(zip_error)?;
                pending.push((path, format!("{}/", name)));
                continue;
            }

            buffer.clear();
            File::open(&path)
                .and_then(|mut file| file.read_to_end(&mut buffer))
                .map_err(|e| CmdError::launcher(format!("無法讀取 {}: {}", name, e)))?;
            zip.start_file(&name, options).map_err(zip_error)?;
            zip.write_all(&buffer)?;
            file_count += 1;
        }
    }

    zip.finish().map_err(zip_error)?;

    Ok(ExportResult {
        path: path_string(target),
        file_count,
        bytes: std::fs::metadata(target).map(|m| m.len()).unwrap_or(0),
    })
}

fn skip(name: &str, is_dir: bool, include_saves: bool, include_screenshots: bool) -> bool {
    if SKIP_ALWAYS.contains(&name) {
        return true;
    }
    // 根目錄的 client jar（`1.21.8.jar` 之類的）
    if !is_dir && !name.contains('/') && name.ends_with(".jar") {
        return true;
    }
    if name.ends_with(".tmp") {
        return true;
    }

    let mc = name.strip_prefix(".minecraft/").unwrap_or("");
    let section = mc.split('/').next().unwrap_or("");
    match section {
        "logs" | "crash-reports" => true,
        "saves" => !include_saves,
        "screenshots" => !include_screenshots,
        _ => false,
    }
}

fn zip_error(e: zip::result::ZipError) -> CmdError {
    CmdError::new("zip", e.to_string())
}

#[tauri::command]
pub async fn import_instance(
    app: tauri::AppHandle,
    base_dir: String,
    zip_path: String,
    name: Option<String>,
) -> CmdResult<InstanceView> {
    let base = PathBuf::from(&base_dir);
    let archive = PathBuf::from(&zip_path);

    let display_name = match name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) {
        Some(name) => name,
        None => read_name(&archive)?.unwrap_or_else(|| {
            archive
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "匯入的實例".to_string())
        }),
    };

    let id = super::instance::unique_instance_id(&base, &display_name).await?;
    let target = instance_dir(&base, &id)?;

    {
        let target = target.clone();
        let archive = archive.clone();
        tokio::task::spawn_blocking(move || extract(&archive, &target))
            .await
            .map_err(|e| CmdError::launcher(format!("匯入中斷: {}", e)))??;
    }

    let mut meta = InstanceMeta::load(&target, &id).await?;
    meta.name = display_name;
    meta.save(&target).await?;

    let _ = app.emit(EVENT_INSTANCES_CHANGED, ());
    build_view(&base, &id, &LocalStore::load().await?).await
}

fn read_name(archive: &Path) -> CmdResult<Option<String>> {
    let file = File::open(archive)?;
    let mut zip = ZipArchive::new(BufReader::new(file)).map_err(zip_error)?;

    let Ok(mut entry) = zip.by_name(META_FILE) else {
        return Ok(None);
    };
    let mut text = String::new();
    entry.read_to_string(&mut text)?;

    let meta: InstanceMeta = serde_json::from_str(&text)?;
    Ok(Some(meta.name).filter(|n| !n.trim().is_empty()))
}

fn extract(archive: &Path, target: &Path) -> CmdResult<()> {
    let file = File::open(archive)?;
    let mut zip = ZipArchive::new(BufReader::new(file)).map_err(zip_error)?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(zip_error)?;

        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let path = target.join(relative);

        if entry.is_dir() {
            std::fs::create_dir_all(&path)?;
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = BufWriter::new(File::create(&path)?);
        std::io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn follows_linked_game_folder() {
        let root = std::env::temp_dir().join(format!("nolauncher-export-{}", uuid::Uuid::new_v4()));
        let external = root.join("prism/.minecraft");
        std::fs::create_dir_all(external.join("mods")).unwrap();
        std::fs::write(external.join("mods/a.jar"), "mod").unwrap();
        std::fs::create_dir_all(external.join("logs")).unwrap();
        std::fs::write(external.join("logs/latest.log"), "log").unwrap();
        let source = root.join("instance/a");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join(META_FILE), "{}").unwrap();
        super::super::foreign::link_dir(&external, &source.join(".minecraft"))
            .await
            .unwrap();

        let target = root.join("out.zip");
        let result = write_archive(&source, &target, false, false).unwrap();
        assert_eq!(result.file_count, 2);
        let mut archive = ZipArchive::new(File::open(&target).unwrap()).unwrap();
        assert!(archive.by_name(".minecraft/mods/a.jar").is_ok());
        assert!(archive.by_name(".minecraft/logs/latest.log").is_err());

        super::super::foreign::unlink(&source.join(".minecraft")).await.unwrap();
        let _ = std::fs::remove_dir_all(&root);
    }
}
