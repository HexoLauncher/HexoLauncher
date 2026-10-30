use std::collections::{BTreeMap, BTreeSet};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use super::foreign::{copy_tree, is_link, unlink};
use super::{instance_dir, path_string};
use crate::error::{CmdError, CmdResult};
use crate::state::AppState;
use crate::store::launcher::LauncherConfig;

const SCHEMATICS: &str = "schematics";
const MANIFEST: &str = "schematic-sync.json";
const RESTORING: &str = "schematics.sync-restore";
const EXTENSIONS: [&str; 4] = ["litematic", "schem", "schematic", "nbt"];

pub fn shared_dir(base_dir: &Path) -> PathBuf {
    base_dir.join(SCHEMATICS)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncFailure {
    pub instance: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchematicSyncReport {
    pub enabled: bool,
    pub shared_dir: String,
    pub changed: Vec<String>,
    pub pending: Vec<String>,
    pub failed: Vec<SyncFailure>,
    pub renamed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    Unchanged,
    Changed { renamed: usize },
}

#[tauri::command]
pub async fn set_schematic_sync(
    state: tauri::State<'_, AppState>,
    base_dir: String,
    enabled: bool,
) -> CmdResult<SchematicSyncReport> {
    let _sync = state.schematic_sync.lock().await;

    let mut config = LauncherConfig::load().await?;
    config.schematics.sync = enabled;
    config.save().await?;

    sync_all(&state, Path::new(&base_dir), enabled).await
}

pub async fn sync_on_startup(state: &AppState) {
    let _sync = state.schematic_sync.lock().await;
    let Ok(config) = LauncherConfig::load().await else {
        return;
    };
    if !config.schematics.sync {
        return;
    }
    if let Ok(base) = config.base_dir() {
        let _ = sync_all(state, &base, true).await;
    }
}

pub async fn sync_after_exit(state: &AppState, base_dir: &Path, instance_name: &str) {
    let _sync = state.schematic_sync.lock().await;
    if let Ok(config) = LauncherConfig::load().await {
        let _ = apply(base_dir, instance_name, config.schematics.sync).await;
    }
}

async fn sync_all(state: &AppState, base: &Path, enabled: bool) -> CmdResult<SchematicSyncReport> {
    let shared = shared_dir(base);
    let mut report = SchematicSyncReport {
        enabled,
        shared_dir: path_string(&shared),
        ..SchematicSyncReport::default()
    };

    let names = instance_names(base).await?;
    let mut had_links = false;
    for name in &names {
        had_links |= legacy_link(base, name).await;
    }

    for name in names {
        if state.is_running(&name)? {
            if needs_change(base, &name, enabled).await {
                report.pending.push(name);
            }
            continue;
        }
        match apply(base, &name, enabled).await {
            Ok(Outcome::Unchanged) => {}
            Ok(Outcome::Changed { renamed }) => {
                report.renamed += renamed;
                report.changed.push(name);
            }
            Err(e) => report.failed.push(SyncFailure {
                instance: name,
                message: e.message,
            }),
        }
    }

    if had_links && report.pending.is_empty() && report.failed.is_empty() {
        let shared = shared.clone();
        let _ = tokio::task::spawn_blocking(move || remove_extras(&shared)).await;
    }
    Ok(report)
}

pub async fn apply(base_dir: &Path, instance_name: &str, enabled: bool) -> CmdResult<Outcome> {
    let dir = instance_dir(base_dir, instance_name)?;
    let mc = dir.join(".minecraft");
    let manifest = dir.join(MANIFEST);
    if !mc.is_dir() {
        return Ok(Outcome::Unchanged);
    }

    let shared = shared_dir(base_dir);
    let restored = restore_link(&mc, &shared).await?;

    if !enabled {
        let removed = tokio::fs::remove_file(&manifest).await.is_ok();
        return Ok(if restored || removed {
            Outcome::Changed { renamed: 0 }
        } else {
            Outcome::Unchanged
        });
    }

    let local = mc.join(SCHEMATICS);
    let synced = tokio::task::spawn_blocking(move || sync_files(&local, &shared, &manifest))
        .await
        .map_err(|e| CmdError::launcher(e.to_string()))??;
    Ok(if restored || synced.changed {
        Outcome::Changed {
            renamed: synced.renamed,
        }
    } else {
        Outcome::Unchanged
    })
}

async fn instance_names(base_dir: &Path) -> CmdResult<Vec<String>> {
    let mut names = Vec::new();
    let mut entries = match tokio::fs::read_dir(base_dir.join("instance")).await {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(names),
        Err(e) => return Err(e.into()),
    };
    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_dir() {
            names.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    names.sort();
    Ok(names)
}

async fn legacy_link(base_dir: &Path, instance_name: &str) -> bool {
    let Ok(dir) = instance_dir(base_dir, instance_name) else {
        return false;
    };
    let target = dir.join(".minecraft").join(SCHEMATICS);
    tokio::fs::symlink_metadata(&target)
        .await
        .is_ok_and(|meta| is_link(&meta))
}

async fn needs_change(base_dir: &Path, instance_name: &str, enabled: bool) -> bool {
    let Ok(dir) = instance_dir(base_dir, instance_name) else {
        return false;
    };
    dir.join(".minecraft").is_dir()
        && (enabled
            || dir.join(MANIFEST).is_file()
            || legacy_link(base_dir, instance_name).await)
}

async fn linked_to(path: &Path, shared: &Path) -> bool {
    let Ok(meta) = tokio::fs::symlink_metadata(path).await else {
        return false;
    };
    if !is_link(&meta) {
        return false;
    }
    match (
        tokio::fs::canonicalize(path).await,
        tokio::fs::canonicalize(shared).await,
    ) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

async fn restore_link(mc: &Path, shared: &Path) -> CmdResult<bool> {
    let target = mc.join(SCHEMATICS);
    let restoring = mc.join(RESTORING);
    let has_restoring = tokio::fs::symlink_metadata(&restoring).await.is_ok();

    if !linked_to(&target, shared).await {
        if !has_restoring {
            return Ok(false);
        }
        if tokio::fs::symlink_metadata(&target).await.is_err() {
            tokio::fs::rename(&restoring, &target).await?;
            return Ok(true);
        }
        tokio::fs::remove_dir_all(&restoring).await?;
        return Ok(false);
    }

    if has_restoring {
        tokio::fs::remove_dir_all(&restoring).await?;
    }
    let from = shared.to_path_buf();
    let to = restoring.clone();
    tokio::task::spawn_blocking(move || {
        if from.is_dir() {
            copy_tree(&from, &to, &[], &mut |_| {})
        } else {
            std::fs::create_dir_all(&to)
        }
    })
    .await
    .map_err(|e| CmdError::launcher(e.to_string()))??;

    unlink(&target).await?;
    tokio::fs::rename(&restoring, &target).await?;
    Ok(true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct Stamp {
    size: u64,
    modified: u64,
}

#[derive(Debug, Default)]
struct Synced {
    changed: bool,
    renamed: usize,
}

fn is_synced_file(name: &str) -> bool {
    if name.starts_with('.') {
        return false;
    }
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| EXTENSIONS.iter().any(|e| ext.eq_ignore_ascii_case(e)))
}

fn stamp(path: &Path) -> std::io::Result<Stamp> {
    let meta = std::fs::metadata(path)?;
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default();
    Ok(Stamp {
        size: meta.len(),
        modified,
    })
}

fn scan(root: &Path) -> std::io::Result<BTreeMap<String, Stamp>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = pending.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };
        for entry in entries {
            let entry = entry?;
            let kind = entry.file_type()?;
            let name = entry.file_name().to_string_lossy().to_string();
            let rel = format!("{}{}", prefix, name);
            if kind.is_dir() && !name.starts_with('.') {
                pending.push((entry.path(), format!("{}/", rel)));
            } else if kind.is_file() && is_synced_file(&name) {
                files.insert(rel, stamp(&entry.path())?);
            }
        }
    }
    Ok(files)
}

fn read_manifest(path: &Path) -> BTreeMap<String, Stamp> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_manifest(path: &Path, files: &BTreeMap<String, Stamp>) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(files)?)?;
    std::fs::rename(&tmp, path)
}

fn sync_files(local: &Path, shared: &Path, manifest: &Path) -> std::io::Result<Synced> {
    std::fs::create_dir_all(local)?;
    std::fs::create_dir_all(shared)?;

    let last = read_manifest(manifest);
    let mine = scan(local)?;
    let theirs = scan(shared)?;
    let mut next = BTreeMap::new();
    let mut synced = Synced::default();

    let all: BTreeSet<&String> = mine.keys().chain(theirs.keys()).collect();
    for rel in all {
        let here = local.join(rel);
        let there = shared.join(rel);
        let before = last.get(rel);

        match (mine.get(rel), theirs.get(rel)) {
            (Some(a), Some(b)) => {
                if a == b || same_content(&here, &there)? {
                    next.insert(rel.clone(), *a);
                    continue;
                }
                synced.changed = true;
                let local_changed = before != Some(a);
                let shared_changed = before != Some(b);
                if local_changed && !shared_changed {
                    next.insert(rel.clone(), copy_file(&here, &there)?);
                } else if shared_changed && !local_changed {
                    next.insert(rel.clone(), copy_file(&there, &here)?);
                } else {
                    let alt = unique_path(&there, |p| {
                        std::fs::symlink_metadata(local.join(relative(shared, p))).is_ok()
                    });
                    let alt_rel = relative(shared, &alt);
                    copy_file(&here, &alt)?;
                    next.insert(alt_rel.clone(), copy_file(&alt, &local.join(&alt_rel))?);
                    next.insert(rel.clone(), copy_file(&there, &here)?);
                    synced.renamed += 1;
                }
            }
            (Some(a), None) => {
                synced.changed = true;
                if before == Some(a) {
                    std::fs::remove_file(&here)?;
                } else {
                    next.insert(rel.clone(), copy_file(&here, &there)?);
                }
            }
            (None, Some(b)) => {
                synced.changed = true;
                if before == Some(b) {
                    std::fs::remove_file(&there)?;
                } else {
                    next.insert(rel.clone(), copy_file(&there, &here)?);
                }
            }
            (None, None) => {}
        }
    }

    if next != last {
        write_manifest(manifest, &next)?;
    }
    Ok(synced)
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn copy_file(from: &Path, to: &Path) -> std::io::Result<Stamp> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let name = to
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let tmp = to.with_file_name(format!(".{}.sync-tmp", name));
    let written = std::fs::File::open(from).and_then(|mut input| {
        let mut output = std::fs::File::create(&tmp)?;
        std::io::copy(&mut input, &mut output)?;
        if let Ok(modified) = std::fs::metadata(from).and_then(|m| m.modified()) {
            output.set_modified(modified)?;
        }
        output.sync_all()
    });
    if let Err(e) = written.and_then(|_| std::fs::rename(&tmp, to)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    stamp(to)
}

fn remove_extras(root: &Path) -> std::io::Result<()> {
    let mut dirs = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                pending.push(entry.path());
                dirs.push(entry.path());
            } else if !is_synced_file(&entry.file_name().to_string_lossy()) {
                std::fs::remove_file(entry.path())?;
            }
        }
    }
    for dir in dirs.iter().rev() {
        let _ = std::fs::remove_dir(dir);
    }
    Ok(())
}

fn same_content(a: &Path, b: &Path) -> std::io::Result<bool> {
    if std::fs::metadata(a)?.len() != std::fs::metadata(b)?.len() {
        return Ok(false);
    }
    Ok(std::fs::read(a)? == std::fs::read(b)?)
}

fn unique_path(path: &Path, taken: impl Fn(&Path) -> bool) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem.to_string(), format!(".{}", ext)),
        _ => (name.clone(), String::new()),
    };
    (2..)
        .map(|n| path.with_file_name(format!("{} ({}){}", stem, n, ext)))
        .find(|p| std::fs::symlink_metadata(p).is_err() && !taken(p))
        .unwrap_or_else(|| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_base() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nolauncher-sync-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, data: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, data).unwrap();
    }

    fn schematics(base: &Path, name: &str) -> PathBuf {
        base.join("instance").join(name).join(".minecraft").join(SCHEMATICS)
    }

    #[tokio::test]
    async fn syncs_only_schematic_files() {
        let base = temp_base();
        write(&schematics(&base, "a").join("house.litematic"), "house");
        write(&schematics(&base, "a").join("readme.txt"), "a only");
        write(&schematics(&base, "b").join("farms/farm.nbt"), "farm");
        std::fs::create_dir_all(base.join("instance/c/.minecraft")).unwrap();

        for name in ["a", "b", "c", "a", "b", "c"] {
            apply(&base, name, true).await.unwrap();
        }
        for name in ["a", "b", "c"] {
            let dir = schematics(&base, name);
            assert!(!is_link(&std::fs::symlink_metadata(&dir).unwrap()));
            assert!(dir.join("house.litematic").is_file());
            assert!(dir.join("farms/farm.nbt").is_file());
        }
        assert!(!shared_dir(&base).join("readme.txt").exists());
        assert!(!schematics(&base, "b").join("readme.txt").exists());
        assert_eq!(apply(&base, "c", true).await.unwrap(), Outcome::Unchanged);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn propagates_edits_and_deletions() {
        let base = temp_base();
        write(&schematics(&base, "a").join("x.litematic"), "v1");
        std::fs::create_dir_all(base.join("instance/b/.minecraft")).unwrap();
        for name in ["a", "b"] {
            apply(&base, name, true).await.unwrap();
        }

        std::thread::sleep(std::time::Duration::from_millis(20));
        write(&schematics(&base, "b").join("x.litematic"), "version 2");
        apply(&base, "b", true).await.unwrap();
        apply(&base, "a", true).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(schematics(&base, "a").join("x.litematic")).unwrap(),
            "version 2"
        );

        std::fs::remove_file(schematics(&base, "a").join("x.litematic")).unwrap();
        apply(&base, "a", true).await.unwrap();
        apply(&base, "b", true).await.unwrap();
        assert!(!shared_dir(&base).join("x.litematic").exists());
        assert!(!schematics(&base, "b").join("x.litematic").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn conflicting_files_are_kept_under_new_names() {
        let base = temp_base();
        write(&schematics(&base, "a").join("clash.litematic"), "mine");
        write(&schematics(&base, "b").join("clash.litematic"), "theirs");

        apply(&base, "a", true).await.unwrap();
        assert_eq!(
            apply(&base, "b", true).await.unwrap(),
            Outcome::Changed { renamed: 1 }
        );
        apply(&base, "a", true).await.unwrap();

        for name in ["a", "b"] {
            let dir = schematics(&base, name);
            assert_eq!(std::fs::read_to_string(dir.join("clash.litematic")).unwrap(), "mine");
            assert_eq!(
                std::fs::read_to_string(dir.join("clash (2).litematic")).unwrap(),
                "theirs"
            );
        }

        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn linked_game_folders_are_synced() {
        let base = temp_base();
        let external = base.join("prism/.minecraft");
        write(&external.join("schematics/farm.litematic"), "farm");
        std::fs::create_dir_all(base.join("instance/a")).unwrap();
        super::super::foreign::link_dir(&external, &base.join("instance/a/.minecraft"))
            .await
            .unwrap();
        std::fs::create_dir_all(base.join("instance/b/.minecraft")).unwrap();

        apply(&base, "a", true).await.unwrap();
        apply(&base, "b", true).await.unwrap();
        assert!(shared_dir(&base).join("farm.litematic").is_file());
        assert!(schematics(&base, "b").join("farm.litematic").is_file());

        unlink(&base.join("instance/a/.minecraft")).await.unwrap();
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn legacy_links_are_turned_back_into_folders() {
        let base = temp_base();
        write(&shared_dir(&base).join("old.litematic"), "old");
        write(&shared_dir(&base).join("pack/config.txt"), "pack");
        let mc = base.join("instance/a/.minecraft");
        std::fs::create_dir_all(&mc).unwrap();
        super::super::foreign::link_dir(&shared_dir(&base), &mc.join(SCHEMATICS))
            .await
            .unwrap();

        assert!(matches!(
            apply(&base, "a", true).await.unwrap(),
            Outcome::Changed { .. }
        ));
        let dir = schematics(&base, "a");
        assert!(!is_link(&std::fs::symlink_metadata(&dir).unwrap()));
        assert!(dir.join("old.litematic").is_file());
        assert!(dir.join("pack/config.txt").is_file());

        remove_extras(&shared_dir(&base)).unwrap();
        assert!(shared_dir(&base).join("old.litematic").is_file());
        assert!(!shared_dir(&base).join("pack").exists());

        let _ = std::fs::remove_dir_all(&base);
    }
}
