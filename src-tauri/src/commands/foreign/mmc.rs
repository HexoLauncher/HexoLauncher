use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{count_mods, folder_name, Candidate, Found, ForeignInstance};
use crate::commands::{path_string, LoaderKind};
use crate::store::instance_meta::{InstanceSettings, DEFAULT_ICON};

/// 預設會去找的啟動器資料目錄名稱（相對系統的資料目錄）。
const KNOWN: [(&str, &str, &str); 4] = [
    ("prism", "Prism Launcher", "PrismLauncher"),
    ("polymc", "PolyMC", "PolyMC"),
    ("multimc", "MultiMC", "MultiMC"),
    ("multimc", "MultiMC", "multimc"),
];

pub(super) fn known() -> Vec<Candidate> {
    let mut found = Vec::new();
    for (kind, name, folder) in KNOWN {
        for root in search_roots() {
            found.push(Candidate {
                kind,
                name,
                path: root.join(folder),
            });
        }
    }
    found
}

fn search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(data) = dirs::data_dir() {
        roots.push(data);
    }
    if let Some(home) = dirs::home_dir() {
        // Linux 的 Flatpak 版本
        roots.push(home.join(".var/app/org.prismlauncher.PrismLauncher/data"));
        roots.push(home.join(".var/app/org.polymc.PolyMC/data"));
    }
    roots
}

/// `path` 本身就是 `instances`，或是底下有 `instances` 的啟動器資料目錄。
pub(super) fn instances_dir(path: &Path) -> Option<PathBuf> {
    let nested = path.join("instances");
    if nested.is_dir() {
        return Some(nested);
    }
    if path.file_name().is_some_and(|n| n == "instances") && path.is_dir() {
        return Some(path.to_path_buf());
    }
    None
}

fn is_foreign_instance(dir: &Path) -> bool {
    dir.is_dir() && (dir.join("instance.cfg").is_file() || dir.join("mmc-pack.json").is_file())
}

/// 舊版 MultiMC 用 `minecraft/`，之後才改成 `.minecraft/`。
fn foreign_game_dir(dir: &Path) -> Option<PathBuf> {
    [".minecraft", "minecraft"]
        .into_iter()
        .map(|name| dir.join(name))
        .find(|candidate| candidate.is_dir())
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
    if !is_foreign_instance(dir) {
        return None;
    }

    let cfg = read_cfg(dir).await;
    let pack = read_pack(dir).await;
    let game_dir = foreign_game_dir(dir);
    let icon_key = cfg.get("iconKey").unwrap_or_default();
    let java_path = cfg
        .flag("OverrideJavaLocation")
        .then(|| cfg.get("JavaPath"))
        .flatten()
        .filter(|p| !p.is_empty());

    Some(Found {
        info: ForeignInstance {
            path: path_string(dir),
            name: cfg
                .get("name")
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| folder_name(dir)),
            notes: unescape(cfg.get("notes").unwrap_or_default()),
            mc_version: pack.mc_version,
            loader: pack.loader,
            loader_version: pack.loader_version,
            unsupported_loader: pack.unsupported,
            mod_count: game_dir.as_deref().map(count_mods).unwrap_or(0),
            // MultiMC 記的是毫秒
            last_played: cfg.number("lastLaunchTime").unwrap_or(0) / 1000,
            play_time_seconds: cfg.number("totalTimePlayed").unwrap_or(0),
        },
        game_dir,
        exclude: Vec::new(),
        icon: icon_for(&icon_key),
        custom_icon: custom_icon_file(dir, &icon_key),
        settings: settings_from(&cfg),
        java_path,
    })
}

/// Prism 的圖示是自己一套，只有名字剛好跟我們的圖示對得上的才沿用。
fn icon_for(key: &str) -> String {
    match key {
        "grass" | "dirt" | "stone" | "planks" | "brick" | "gold" | "iron" | "diamond"
        | "netherrack" | "tnt" | "creeper" => key.to_string(),
        _ => DEFAULT_ICON.to_string(),
    }
}

fn custom_icon_file(source: &Path, key: &str) -> Option<PathBuf> {
    if key.is_empty() || key == "default" || key.contains(['/', '\\']) || key.contains("..") {
        return None;
    }
    let icons = source.parent()?.parent()?.join("icons");
    std::fs::read_dir(icons)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_file() && p.file_stem().is_some_and(|s| s == key))
}

fn settings_from(cfg: &Cfg) -> InstanceSettings {
    InstanceSettings {
        override_memory: cfg.flag("OverrideMemory"),
        min_memory_mb: cfg.number("MinMemAlloc").unwrap_or(512) as u32,
        max_memory_mb: cfg.number("MaxMemAlloc").unwrap_or(4096) as u32,
        override_jvm_args: cfg.flag("OverrideJavaArgs"),
        jvm_args: cfg.get("JvmArgs").unwrap_or_default(),
        override_commands: cfg.flag("OverrideCommands"),
        pre_launch: cfg.get("PreLaunchCommand").unwrap_or_default(),
        post_exit: cfg.get("PostExitCommand").unwrap_or_default(),
    }
}

/// `instance.cfg` 是很陽春的 `key=value`，值不加引號、也不會跨行。
#[derive(Debug, Default)]
struct Cfg(HashMap<String, String>);

impl Cfg {
    fn parse(text: &str) -> Self {
        let mut map = HashMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                map.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
        Self(map)
    }

    fn get(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }

    fn flag(&self, key: &str) -> bool {
        self.0
            .get(key)
            .is_some_and(|v| v.eq_ignore_ascii_case("true"))
    }

    fn number(&self, key: &str) -> Option<u64> {
        self.0.get(key)?.parse().ok()
    }
}

/// `instance.cfg` 是 Qt 的 INI，換行之類的字元在值裡是用反斜線跳脫的。
fn unescape(value: String) -> String {
    const BACKSLASH: char = '\\';
    if !value.contains(BACKSLASH) {
        return value;
    }

    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != BACKSLASH {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => out.push(BACKSLASH),
        }
    }
    out
}

async fn read_cfg(dir: &Path) -> Cfg {
    match tokio::fs::read_to_string(dir.join("instance.cfg")).await {
        Ok(text) => Cfg::parse(&text),
        Err(_) => Cfg::default(),
    }
}

#[derive(Debug, Default)]
struct Pack {
    mc_version: String,
    loader: LoaderKind,
    loader_version: String,
    unsupported: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MmcPack {
    #[serde(default)]
    components: Vec<MmcComponent>,
}

#[derive(Debug, Deserialize)]
struct MmcComponent {
    uid: String,
    #[serde(default)]
    version: String,
}

async fn read_pack(dir: &Path) -> Pack {
    let Ok(text) = tokio::fs::read_to_string(dir.join("mmc-pack.json")).await else {
        return Pack::default();
    };
    let Ok(pack) = serde_json::from_str::<MmcPack>(&text) else {
        return Pack::default();
    };

    let mut result = Pack::default();
    for component in pack.components {
        match component.uid.as_str() {
            "net.minecraft" => result.mc_version = component.version,
            "net.fabricmc.fabric-loader" => {
                result.loader = LoaderKind::Fabric;
                result.loader_version = component.version;
            }
            "net.minecraftforge" => {
                result.loader = LoaderKind::Forge;
                result.loader_version = component.version;
            }
            "net.neoforged" => {
                result.loader = LoaderKind::NeoForge;
                result.loader_version = component.version;
            }
            "org.quiltmc.quilt-loader" => result.unsupported = Some("Quilt".to_string()),
            "com.mumfrey.liteloader" => result.unsupported = Some("LiteLoader".to_string()),
            _ => {}
        }
    }
    result
}
