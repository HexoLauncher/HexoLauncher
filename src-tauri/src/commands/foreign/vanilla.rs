use std::path::{Path, PathBuf};

use hexomc_lib::{HexoError, HmclVersionJson};
use serde_json::Value;

use super::{count_mods, Candidate, ForeignInstance, Found};
use crate::commands::{path_string, LoaderKind};
use crate::store::instance_meta::{InstanceSettings, DEFAULT_ICON};

const SHARED: [&str; 3] = ["versions", "libraries", "assets"];
const ISOLATION_MARKERS: [&str; 5] = ["mods", "saves", "config", "resourcepacks", "options.txt"];

pub(super) fn known() -> Vec<Candidate> {
    let mut found = Vec::new();
    if let Some(data) = dirs::data_dir() {
        for folder in [".minecraft", "minecraft"] {
            found.push(Candidate {
                kind: "minecraft",
                name: "Minecraft / HMCL",
                path: data.join(folder),
            });
        }
    }
    if let Some(home) = dirs::home_dir() {
        found.push(Candidate {
            kind: "minecraft",
            name: "Minecraft / HMCL",
            path: home.join(".minecraft"),
        });
    }
    found
}

pub(super) fn minecraft_root(path: &Path) -> Option<PathBuf> {
    if path.join("versions").is_dir() {
        return Some(path.to_path_buf());
    }
    if path.file_name().is_some_and(|n| n == "versions") && path.is_dir() {
        return path.parent().map(Path::to_path_buf);
    }
    let nested = path.join(".minecraft");
    nested.join("versions").is_dir().then_some(nested)
}

pub(super) async fn list(root: &Path) -> Vec<Found> {
    let mut result = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(root.join("versions")).await else {
        return result;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Some(found) = read_version(root, &entry.path()).await {
            result.push(found);
        }
    }
    result
}

pub(super) async fn read(dir: &Path) -> Option<Found> {
    let versions = dir.parent()?;
    if versions.file_name()? != "versions" {
        return None;
    }
    read_version(versions.parent()?, dir).await
}

async fn read_version(root: &Path, dir: &Path) -> Option<Found> {
    let id = dir.file_name()?.to_string_lossy().to_string();
    let text = tokio::fs::read_to_string(dir.join(format!("{}.json", id)))
        .await
        .ok()?;
    let version: HmclVersionJson = serde_json::from_str(&text).ok()?;

    let mc_version = version.mc_version().unwrap_or_default();
    let (loader, loader_version, unsupported_loader) = match version.loader(&mc_version) {
        Ok((loader, version)) => (LoaderKind::from(&loader), version.unwrap_or_default(), None),
        Err(HexoError::UnsupportedLoader(name)) => (LoaderKind::Vanilla, String::new(), Some(name)),
        Err(_) => (LoaderKind::Vanilla, String::new(), None),
    };

    let cfg = read_hmcl_cfg(dir).await;
    let game_dir = game_dir(root, dir, cfg.as_ref());
    let exclude = if game_dir == root {
        SHARED.iter().map(|s| s.to_string()).collect()
    } else {
        version_files(dir, &id)
    };

    Some(Found {
        info: ForeignInstance {
            path: path_string(dir),
            name: id,
            notes: String::new(),
            mc_version,
            loader,
            loader_version,
            mod_count: count_mods(&game_dir),
            last_played: 0,
            play_time_seconds: 0,
            unsupported_loader,
        },
        game_dir: Some(game_dir),
        exclude,
        icon: DEFAULT_ICON.to_string(),
        custom_icon: None,
        settings: cfg.as_ref().map(settings_from).unwrap_or_default(),
        java_path: cfg.as_ref().and_then(java_path_from),
    })
}

async fn read_hmcl_cfg(dir: &Path) -> Option<Value> {
    let text = tokio::fs::read_to_string(dir.join("hmclversion.cfg"))
        .await
        .ok()?;
    let cfg: Value = serde_json::from_str(&text).ok()?;
    let uses_global = cfg
        .get("usesGlobal")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    (!uses_global).then_some(cfg)
}

fn game_dir(root: &Path, dir: &Path, cfg: Option<&Value>) -> PathBuf {
    let kind = cfg.and_then(|c| c.get("gameDirType")).and_then(|t| match t {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => match s.as_str() {
            "ROOT_FOLDER" => Some(0),
            "VERSION_FOLDER" => Some(1),
            "CUSTOM" => Some(2),
            _ => None,
        },
        _ => None,
    });
    match kind {
        Some(0) => return root.to_path_buf(),
        Some(1) => return dir.to_path_buf(),
        Some(2) => {
            let custom = cfg
                .and_then(|c| c.get("gameDir"))
                .and_then(Value::as_str)
                .map(PathBuf::from)
                .filter(|p| p.is_absolute() && p.is_dir());
            if let Some(custom) = custom {
                return custom;
            }
        }
        _ => {}
    }
    if ISOLATION_MARKERS.iter().any(|n| dir.join(n).exists()) {
        dir.to_path_buf()
    } else {
        root.to_path_buf()
    }
}

fn version_files(dir: &Path, id: &str) -> Vec<String> {
    let mut names = vec![
        format!("{}.jar", id),
        format!("{}.json", id),
        "hmclversion.cfg".to_string(),
    ];
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("natives") || name.ends_with("-natives") {
                names.push(name);
            }
        }
    }
    names
}

fn settings_from(cfg: &Value) -> InstanceSettings {
    let number = |key: &str| cfg.get(key).and_then(Value::as_u64).map(|v| v as u32);
    let max = number("maxMemory");
    let jvm_args = cfg
        .get("javaArgs")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let defaults = InstanceSettings::default();
    InstanceSettings {
        override_memory: max.is_some(),
        min_memory_mb: number("minMemory").unwrap_or(defaults.min_memory_mb),
        max_memory_mb: max.unwrap_or(defaults.max_memory_mb),
        override_jvm_args: !jvm_args.is_empty(),
        jvm_args,
        ..defaults
    }
}

fn java_path_from(cfg: &Value) -> Option<String> {
    cfg.get("javaDir")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|p| !p.is_empty() && Path::new(p).is_file())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reads_isolated_and_shared_versions() {
        let root = std::env::temp_dir().join(format!("nolauncher-vanilla-{}", uuid::Uuid::new_v4()));
        let forge = root.join("versions/1.20.1-forge");
        let plain = root.join("versions/1.21.1");
        std::fs::create_dir_all(forge.join("mods")).unwrap();
        std::fs::create_dir_all(&plain).unwrap();
        std::fs::write(forge.join("mods/a.jar"), "").unwrap();
        std::fs::write(
            forge.join("1.20.1-forge.json"),
            r#"{"id":"1.20.1-forge","inheritsFrom":"1.20.1","arguments":{"game":["--fml.forgeVersion","47.2.0"]}}"#,
        )
        .unwrap();
        std::fs::write(forge.join("1.20.1-forge.jar"), "").unwrap();
        std::fs::write(plain.join("1.21.1.json"), r#"{"id":"1.21.1"}"#).unwrap();

        assert_eq!(minecraft_root(&root), Some(root.clone()));
        let mut found = list(&root).await;
        found.sort_by(|a, b| a.info.name.cmp(&b.info.name));
        assert_eq!(found.len(), 2);

        let modded = &found[0];
        assert_eq!(modded.info.mc_version, "1.20.1");
        assert_eq!(modded.info.loader, LoaderKind::Forge);
        assert_eq!(modded.info.loader_version, "47.2.0");
        assert_eq!(modded.info.mod_count, 1);
        assert_eq!(modded.game_dir.as_deref(), Some(forge.as_path()));
        assert!(modded.exclude.contains(&"1.20.1-forge.jar".to_string()));

        let vanilla = &found[1];
        assert_eq!(vanilla.info.loader, LoaderKind::Vanilla);
        assert_eq!(vanilla.game_dir.as_deref(), Some(root.as_path()));
        assert!(vanilla.exclude.contains(&"versions".to_string()));

        assert!(read(&forge).await.is_some());
        let _ = std::fs::remove_dir_all(&root);
    }
}
