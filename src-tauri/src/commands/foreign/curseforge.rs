use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{count_mods, folder_name, Candidate, ForeignInstance, Found};
use crate::commands::{path_string, LoaderKind};
use crate::store::instance_meta::{InstanceSettings, DEFAULT_ICON};

const MANIFEST: &str = "minecraftinstance.json";

pub(super) fn known() -> Vec<Candidate> {
    [dirs::home_dir(), dirs::document_dir()]
        .into_iter()
        .flatten()
        .map(|dir| Candidate {
            kind: "curseforge",
            name: "CurseForge",
            path: dir.join("curseforge").join("minecraft").join("Instances"),
        })
        .collect()
}

pub(super) fn instances_dir(path: &Path) -> Option<PathBuf> {
    [
        path.to_path_buf(),
        path.join("Instances"),
        path.join("minecraft").join("Instances"),
    ]
    .into_iter()
    .find(|dir| has_instances(dir))
}

fn has_instances(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|e| e.path().join(MANIFEST).is_file())
    })
}

pub(super) async fn list(instances: &Path) -> Vec<Found> {
    let mut result = Vec::new();
    let Ok(mut entries) = tokio::fs::read_dir(instances).await else {
        return result;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Some(found) = read(&entry.path()).await {
            result.push(found);
        }
    }
    result
}

pub(super) async fn read(dir: &Path) -> Option<Found> {
    let text = tokio::fs::read_to_string(dir.join(MANIFEST)).await.ok()?;
    let manifest: Value = serde_json::from_str(&text).ok()?;
    let text_of = |value: Option<&Value>| {
        value
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };

    let base_loader = manifest.get("baseModLoader");
    let mc_version = text_of(manifest.get("gameVersion"))
        .or_else(|| text_of(base_loader.and_then(|l| l.get("minecraftVersion"))))
        .unwrap_or_default();
    let (loader, loader_version, unsupported_loader) =
        parse_loader(text_of(base_loader.and_then(|l| l.get("name"))).as_deref(), &mc_version);

    let jvm_args = text_of(manifest.get("javaArgsOverride")).unwrap_or_default();
    let custom_icon = text_of(manifest.get("profileImagePath"))
        .map(PathBuf::from)
        .filter(|p| p.is_file());

    Some(Found {
        info: ForeignInstance {
            path: path_string(dir),
            name: text_of(manifest.get("name")).unwrap_or_else(|| folder_name(dir)),
            notes: String::new(),
            mc_version,
            loader,
            loader_version,
            mod_count: count_mods(dir),
            last_played: 0,
            play_time_seconds: 0,
            unsupported_loader,
        },
        game_dir: Some(dir.to_path_buf()),
        exclude: vec![MANIFEST.to_string()],
        icon: DEFAULT_ICON.to_string(),
        custom_icon,
        settings: InstanceSettings {
            override_jvm_args: !jvm_args.is_empty(),
            jvm_args,
            ..InstanceSettings::default()
        },
        java_path: None,
    })
}

fn parse_loader(name: Option<&str>, mc_version: &str) -> (LoaderKind, String, Option<String>) {
    let Some((kind, version)) = name.and_then(|n| n.split_once('-')) else {
        return (LoaderKind::Vanilla, String::new(), None);
    };
    let version = version
        .strip_suffix(&format!("-{}", mc_version))
        .unwrap_or(version);
    let version = version
        .strip_prefix(&format!("{}-", mc_version))
        .unwrap_or(version)
        .to_string();
    match kind.to_ascii_lowercase().as_str() {
        "forge" => (LoaderKind::Forge, version, None),
        "neoforge" => (LoaderKind::NeoForge, version, None),
        "fabric" => (LoaderKind::Fabric, version, None),
        other => (LoaderKind::Vanilla, String::new(), Some(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_names() {
        assert_eq!(
            parse_loader(Some("forge-47.2.0"), "1.20.1"),
            (LoaderKind::Forge, "47.2.0".into(), None)
        );
        assert_eq!(
            parse_loader(Some("fabric-0.14.21-1.20.1"), "1.20.1"),
            (LoaderKind::Fabric, "0.14.21".into(), None)
        );
        assert_eq!(
            parse_loader(Some("neoforge-21.1.77"), "1.21.1"),
            (LoaderKind::NeoForge, "21.1.77".into(), None)
        );
        assert_eq!(
            parse_loader(Some("quilt-0.26.0"), "1.21.1"),
            (LoaderKind::Vanilla, String::new(), Some("quilt".into()))
        );
        assert_eq!(parse_loader(None, "1.21.1"), (LoaderKind::Vanilla, String::new(), None));
    }
}
