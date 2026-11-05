use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::Emitter;

use super::modpack::file_sha1;
use super::{minecraft_dir, path_string};
use crate::error::{CmdError, CmdResult};

pub const EVENT_MANUAL_DOWNLOAD_FOUND: &str = "nolauncher://manual-download-found";

const POLL: Duration = Duration::from_millis(1500);
const GIVE_UP_AFTER: Duration = Duration::from_secs(60 * 60);

static WATCHES: Mutex<Option<HashMap<String, u64>>> = Mutex::new(None);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchItem {
    pub dest: String,
    pub file_name: String,
    pub sha1: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchStarted {
    pub downloads_dir: String,
    pub done: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualDownloadFound {
    pub instance_name: String,
    pub dest: String,
}

struct Pending {
    dest: String,
    target: PathBuf,
    file_name: String,
    sha1: Option<String>,
}

#[tauri::command]
pub async fn watch_manual_downloads(
    app: tauri::AppHandle,
    base_dir: String,
    instance_name: String,
    items: Vec<WatchItem>,
) -> CmdResult<WatchStarted> {
    let downloads =
        dirs::download_dir().ok_or_else(|| CmdError::launcher("找不到系統的「下載」資料夾"))?;
    let mc_dir = minecraft_dir(Path::new(&base_dir), &instance_name)?;

    let mut pending = Vec::new();
    let mut done = Vec::new();
    for item in items {
        let target = mc_dir.join(relative_dest(&item.dest)?);
        if is_in_place(&target, item.sha1.as_deref()).await {
            done.push(item.dest);
            continue;
        }
        pending.push(Pending {
            dest: item.dest,
            target,
            file_name: item.file_name,
            sha1: item.sha1,
        });
    }

    let generation = next_generation(&instance_name);
    if !pending.is_empty() {
        let watch_dir = downloads.clone();
        tauri::async_runtime::spawn(watch(app, instance_name, generation, watch_dir, pending));
    }

    Ok(WatchStarted {
        downloads_dir: path_string(&downloads),
        done,
    })
}

#[tauri::command]
pub fn stop_manual_downloads(instance_name: String) {
    next_generation(&instance_name);
}

fn next_generation(instance_name: &str) -> u64 {
    let Ok(mut watches) = WATCHES.lock() else {
        return 0;
    };
    let entry = watches
        .get_or_insert_with(HashMap::new)
        .entry(instance_name.to_string())
        .or_insert(0);
    *entry += 1;
    *entry
}

fn is_current(instance_name: &str, generation: u64) -> bool {
    WATCHES
        .lock()
        .ok()
        .and_then(|w| w.as_ref().and_then(|m| m.get(instance_name).copied()))
        == Some(generation)
}

fn relative_dest(dest: &str) -> CmdResult<PathBuf> {
    let path = Path::new(dest);
    let safe = path.components().count() > 0
        && path.components().all(|c| matches!(c, Component::Normal(_)));
    if safe {
        Ok(path.to_path_buf())
    } else {
        Err(CmdError::launcher(format!("目的地路徑不合法: {}", dest)))
    }
}

async fn is_in_place(target: &Path, sha1: Option<&str>) -> bool {
    if !target.is_file() {
        return false;
    }
    match sha1 {
        Some(expected) => file_sha1(target)
            .await
            .is_ok_and(|actual| actual.eq_ignore_ascii_case(expected)),
        None => true,
    }
}

async fn watch(
    app: tauri::AppHandle,
    instance_name: String,
    generation: u64,
    downloads: PathBuf,
    mut pending: Vec<Pending>,
) {
    let started = Instant::now();
    let mut sizes: HashMap<PathBuf, u64> = HashMap::new();
    let mut rejected: HashMap<PathBuf, u64> = HashMap::new();

    while !pending.is_empty() && started.elapsed() < GIVE_UP_AFTER {
        tokio::time::sleep(POLL).await;
        if !is_current(&instance_name, generation) {
            return;
        }

        let files = list_files(&downloads);
        let mut seen = HashMap::new();
        let mut index = 0;
        while index < pending.len() {
            let item = &pending[index];
            let mut moved = false;
            for (path, size) in files
                .iter()
                .filter(|(p, _)| matches_name(p, &item.file_name))
            {
                seen.insert(path.clone(), *size);
                let stable = *size > 0 && sizes.get(path) == Some(size);
                if !stable || rejected.get(path) == Some(size) {
                    continue;
                }
                if !is_in_place(path, item.sha1.as_deref()).await {
                    rejected.insert(path.clone(), *size);
                    continue;
                }
                if move_file(path, &item.target).await.is_ok() {
                    moved = true;
                    break;
                }
            }
            if moved {
                let item = pending.remove(index);
                let _ = app.emit(
                    EVENT_MANUAL_DOWNLOAD_FOUND,
                    ManualDownloadFound {
                        instance_name: instance_name.clone(),
                        dest: item.dest,
                    },
                );
            } else {
                index += 1;
            }
        }
        sizes = seen;
    }
}

fn list_files(dir: &Path) -> Vec<(PathBuf, u64)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            meta.is_file().then(|| (entry.path(), meta.len()))
        })
        .collect()
}

fn matches_name(path: &Path, expected: &str) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if name.eq_ignore_ascii_case(expected) {
        return true;
    }
    let (stem, ext) = match expected.rsplit_once('.') {
        Some((stem, ext)) => (stem, format!(".{}", ext)),
        None => (expected, String::new()),
    };
    let Some(rest) = name
        .strip_prefix(stem)
        .and_then(|r| r.strip_suffix(ext.as_str()))
    else {
        return false;
    };
    rest.strip_prefix(" (")
        .and_then(|r| r.strip_suffix(')'))
        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

async fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if let Some(parent) = to.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    if tokio::fs::rename(from, to).await.is_ok() {
        return Ok(());
    }
    tokio::fs::copy(from, to).await?;
    tokio::fs::remove_file(from).await
}

#[cfg(test)]
mod tests {
    use super::{matches_name, relative_dest};
    use std::path::Path;

    #[test]
    fn matches_exact_and_browser_duplicate_names() {
        let expected = "jei-1.20.1-15.2.0.jar";
        assert!(matches_name(Path::new("jei-1.20.1-15.2.0.jar"), expected));
        assert!(matches_name(Path::new("JEI-1.20.1-15.2.0.JAR"), expected));
        assert!(matches_name(
            Path::new("jei-1.20.1-15.2.0 (2).jar"),
            expected
        ));
        assert!(!matches_name(
            Path::new("jei-1.20.1-15.2.0.jar.crdownload"),
            expected
        ));
        assert!(!matches_name(
            Path::new("jei-1.20.1-15.2.0 (x).jar"),
            expected
        ));
        assert!(!matches_name(Path::new("jei-1.20.1-15.2.1.jar"), expected));
    }

    #[test]
    fn dest_must_stay_inside_minecraft_dir() {
        assert!(relative_dest("mods/jei.jar").is_ok());
        assert!(relative_dest("../mods/jei.jar").is_err());
        assert!(relative_dest("C:/Windows/jei.jar").is_err());
        assert!(relative_dest("/etc/jei.jar").is_err());
        assert!(relative_dest("").is_err());
    }
}
