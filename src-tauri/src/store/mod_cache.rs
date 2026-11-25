use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use hexomc_lib::{read_mod_info, ModInfo};
use serde::{Deserialize, Serialize};

use super::store_dir;

const CACHE_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FileStamp {
    size: u64,
    modified: u64,
}

impl FileStamp {
    fn of(meta: &fs::Metadata) -> Self {
        Self {
            size: meta.len(),
            modified: meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_nanos() as u64)
                .unwrap_or(0),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct CachedMod {
    stamp: FileStamp,
    info: ModInfo,
}

#[derive(Default, Serialize, Deserialize)]
struct ModCache {
    version: u32,
    mods: HashMap<String, CachedMod>,
}

pub fn is_mod_file(path: &Path) -> bool {
    let name = path.to_string_lossy().to_ascii_lowercase();
    name.ends_with(".jar") || name.ends_with(".jar.disabled")
}

pub type Fingerprint = Vec<(String, FileStamp)>;

pub fn fingerprint(mods_dir: &Path) -> Fingerprint {
    let mut files: Fingerprint = mod_files(mods_dir)
        .into_iter()
        .map(|(name, _, stamp)| (name, stamp))
        .collect();
    files.sort();
    files
}

fn mod_files(mods_dir: &Path) -> Vec<(String, PathBuf, FileStamp)> {
    let Ok(entries) = fs::read_dir(mods_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let meta = entry.metadata().ok()?;
            (meta.is_file() && is_mod_file(&path)).then(|| {
                let name = entry.file_name().to_string_lossy().into_owned();
                (name, path, FileStamp::of(&meta))
            })
        })
        .collect()
}

fn cache_path(mods_dir: &Path) -> Option<PathBuf> {
    let mut hasher = DefaultHasher::new();
    mods_dir.hash(&mut hasher);
    Some(
        store_dir()
            .ok()?
            .join("cache")
            .join("mods")
            .join(format!("{:016x}.json", hasher.finish())),
    )
}

fn read_cache(path: &Path) -> ModCache {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<ModCache>(&bytes).ok())
        .filter(|cache| cache.version == CACHE_VERSION)
        .unwrap_or_default()
}

fn write_cache(path: &Path, cache: &ModCache) {
    let Some(parent) = path.parent() else { return };
    let Ok(json) = serde_json::to_vec(cache) else {
        return;
    };
    let temp = path.with_extension("json.tmp");
    if fs::create_dir_all(parent).is_ok() && fs::write(&temp, json).is_ok() {
        let _ = fs::rename(&temp, path);
    }
}

pub fn scan_mods(mods_dir: &Path) -> Vec<ModInfo> {
    let path = cache_path(mods_dir);
    let mut old = path.as_deref().map(read_cache).unwrap_or_default();
    let mut fresh = ModCache {
        version: CACHE_VERSION,
        mods: HashMap::new(),
    };
    let mut changed = false;

    for (name, file, stamp) in mod_files(mods_dir) {
        let cached = old
            .mods
            .remove(&name)
            .filter(|cached| cached.stamp == stamp);
        let entry = match cached {
            Some(entry) => entry,
            None => {
                changed = true;
                let Some(info) = read_mod_info(&file) else {
                    continue;
                };
                CachedMod { stamp, info }
            }
        };
        fresh.mods.insert(name, entry);
    }

    if changed || !old.mods.is_empty() {
        if let Some(path) = path {
            write_cache(&path, &fresh);
        }
    }
    fresh.mods.into_values().map(|entry| entry.info).collect()
}
