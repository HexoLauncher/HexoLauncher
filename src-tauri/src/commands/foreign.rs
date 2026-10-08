use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Emitter;

use super::install::InstallProgress;
use super::instance::{build_view, unique_instance_id, InstanceView, EVENT_INSTANCES_CHANGED};
use super::{
    full_loader_version, instance_dir, minecraft_dir, path_string, LoaderKind,
    EVENT_INSTALL_PROGRESS,
};
use crate::error::{CmdError, CmdResult};
use crate::store::instance_meta::{
    InstanceLocal, InstanceMeta, InstanceSettings, LocalStore, DEFAULT_ICON,
};
use crate::store::now_secs;

/// 每複製幾個檔案回報一次進度，免得檔案很多時事件量爆掉。
const PROGRESS_EVERY: usize = 64;


/// 預設會去找的啟動器資料目錄名稱（相對系統的資料目錄）。
const KNOWN: [(&str, &str, &str); 4] = [
    ("prism", "Prism Launcher", "PrismLauncher"),
    ("polymc", "PolyMC", "PolyMC"),
    ("multimc", "MultiMC", "MultiMC"),
    ("multimc", "MultiMC", "multimc"),
];

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportMode {
    /// 把遊戲目錄整份複製過來，兩邊各玩各的。
    Copy,
    /// 建立連結指向原本的遊戲目錄，兩邊共用同一份存檔與模組。
    Link,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeignLauncher {
    pub kind: String,
    pub name: String,

    /// 啟動器的資料目錄
    pub path: String,

    pub instance_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeignInstance {
    /// 實例資料夾的完整路徑，匯入時要原封不動傳回來。
    pub path: String,
    pub name: String,
    pub notes: String,
    pub mc_version: String,
    pub loader: LoaderKind,
    pub loader_version: String,

    pub mod_count: usize,
    pub last_played: u64,
    pub play_time_seconds: u64,

    /// 我們不支援的模組載入器（Quilt 之類的），匯入後要自己換掉。
    pub unsupported_loader: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeignImportRequest {
    pub base_dir: String,

    /// 要匯入的實例資料夾，來自 `list_foreign_instances`。
    pub paths: Vec<String>,
    pub mode: ImportMode,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeignImportFailure {
    pub path: String,
    pub name: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeignImportResult {
    pub imported: Vec<InstanceView>,
    pub failed: Vec<ForeignImportFailure>,
}

/// 掃預設安裝位置，列出找得到的啟動器。找不到不算錯誤，回空陣列讓使用者自己指定。
#[tauri::command]
pub async fn detect_foreign_launchers() -> CmdResult<Vec<ForeignLauncher>> {
    let roots = search_roots();
    let mut found: Vec<ForeignLauncher> = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    for (kind, name, folder) in KNOWN {
        for root in &roots {
            let dir = root.join(folder);
            let Some(instances) = instances_dir(&dir) else {
                continue;
            };
            let path = path_string(&dir);
            // Windows 的檔名不分大小寫，`MultiMC` 跟 `multimc` 會是同一個資料夾。
            let key = if cfg!(windows) {
                path.to_lowercase()
            } else {
                path.clone()
            };
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);

            found.push(ForeignLauncher {
                kind: kind.to_string(),
                name: name.to_string(),
                path,
                instance_count: count_instances(&instances).await,
            });
        }
    }
    Ok(found)
}

/// 列出一個啟動器裡的實例。`path` 可以是啟動器的資料目錄，也可以直接是 `instances` 資料夾。
#[tauri::command]
pub async fn list_foreign_instances(path: String) -> CmdResult<Vec<ForeignInstance>> {
    let root = PathBuf::from(path.trim());
    let instances = instances_dir(&root).ok_or_else(|| {
        CmdError::new(
            "notLauncherDir",
            "這個資料夾裡找不到 MultiMC 格式的實例，請選啟動器的資料目錄或它的 instances 資料夾",
        )
    })?;

    let mut entries = tokio::fs::read_dir(&instances).await?;
    let mut result = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        if !entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        if let Some(instance) = read_foreign(&entry.path()).await {
            result.push(instance);
        }
    }

    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(result)
}

#[tauri::command]
pub async fn import_foreign_instances(
    app: tauri::AppHandle,
    request: ForeignImportRequest,
) -> CmdResult<ForeignImportResult> {
    let ForeignImportRequest {
        base_dir,
        paths,
        mode,
        group,
    } = request;

    let base = PathBuf::from(&base_dir);
    let group = group.unwrap_or_default().trim().to_string();

    let mut imported = Vec::new();
    let mut failed = Vec::new();

    for path in paths {
        let source = PathBuf::from(&path);
        let name = read_foreign(&source)
            .await
            .map(|i| i.name)
            .unwrap_or_else(|| folder_name(&source));

        match import_one(&app, &base, &source, mode, &group).await {
            Ok(view) => imported.push(view),
            Err(e) => failed.push(ForeignImportFailure {
                path,
                name,
                message: e.message,
            }),
        }
    }

    if !imported.is_empty() {
        let _ = app.emit(EVENT_INSTANCES_CHANGED, ());
    }
    Ok(ForeignImportResult { imported, failed })
}

async fn import_one(
    app: &tauri::AppHandle,
    base: &Path,
    source: &Path,
    mode: ImportMode,
    group: &str,
) -> CmdResult<InstanceView> {
    let foreign = read_foreign(source)
        .await
        .ok_or_else(|| CmdError::new("notLauncherDir", "這不是 MultiMC 格式的實例資料夾"))?;

    let game_dir = foreign_game_dir(source).ok_or_else(|| {
        CmdError::launcher("這個實例還沒有遊戲目錄（.minecraft），先在原本的啟動器裡啟動一次")
    })?;

    // 連結到自己的資料目錄底下會變成無限巢狀，直接擋掉。
    if mode == ImportMode::Link && game_dir.starts_with(base) {
        return Err(CmdError::launcher(
            "這個實例已經在啟動器的資料目錄裡，不能再建立連結",
        ));
    }

    let id = unique_instance_id(base, &foreign.name).await?;
    let dir = instance_dir(base, &id)?;
    let target = minecraft_dir(base, &id)?;
    tokio::fs::create_dir_all(&dir).await?;

    // 失敗就把建到一半的資料夾收掉，不要留下半個實例。
    if let Err(e) = place_game_dir(app, &id, &game_dir, &target, mode).await {
        let _ = tokio::fs::remove_dir_all(&dir).await;
        return Err(e);
    }

    let cfg = read_cfg(source).await;
    let icon_key = cfg.get("iconKey").unwrap_or_default();
    let mut meta = InstanceMeta {
        name: foreign.name,
        icon: icon_for(&icon_key),
        group: group.to_string(),
        notes: foreign.notes,
        mc_version: foreign.mc_version.clone(),
        loader: foreign.loader,
        loader_version: full_loader_version(
            foreign.loader,
            &foreign.mc_version,
            &foreign.loader_version,
        )
        .await,
        created_at: now_secs(),
        last_played: foreign.last_played,
        play_time_seconds: foreign.play_time_seconds,
        settings: settings_from(&cfg),
        ..InstanceMeta::default()
    };
    if let Some(file) = custom_icon_file(source, &icon_key) {
        if super::icon::copy_icon(&dir, &file).await.is_ok() {
            meta.icon = super::icon::CUSTOM_ICON.to_string();
        }
    }
    meta.save(&dir).await?;

    if cfg.flag("OverrideJavaLocation") {
        if let Some(java_path) = cfg.get("JavaPath").filter(|p| !p.is_empty()) {
            let mut local = LocalStore::load().await?;
            local.set(base, &id, InstanceLocal { java_path });
            local.save().await?;
        }
    }

    build_view(base, &id, &LocalStore::load().await?).await
}

async fn place_game_dir(
    app: &tauri::AppHandle,
    id: &str,
    source: &Path,
    target: &Path,
    mode: ImportMode,
) -> CmdResult<()> {
    if mode == ImportMode::Link {
        return link_dir(source, target).await;
    }

    let app = app.clone();
    let id = id.to_string();
    let source = source.to_path_buf();
    let target = target.to_path_buf();

    tokio::task::spawn_blocking(move || {
        let total = count_files(&source);
        let mut done = 0;

        copy_tree(&source, &target, &mut |_| {
            done += 1;
            if done % PROGRESS_EVERY == 0 || done == total {
                let _ = app.emit(
                    EVENT_INSTALL_PROGRESS,
                    InstallProgress {
                        instance_name: id.clone(),
                        done,
                        total,
                        message: "正在複製遊戲目錄".to_string(),
                    },
                );
            }
        })
    })
    .await
    .map_err(|e| CmdError::launcher(format!("複製中斷: {}", e)))?
    .map_err(CmdError::from)
}

fn count_files(root: &Path) -> usize {
    let mut pending = vec![root.to_path_buf()];
    let mut total = 0;

    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => pending.push(entry.path()),
                Ok(kind) if kind.is_file() => total += 1,
                _ => {}
            }
        }
    }
    total
}

/// 只複製一般檔案與資料夾；來源裡的連結不跟著展開，免得整個檔案系統被拖進來。
fn copy_tree(source: &Path, target: &Path, on_file: &mut dyn FnMut(&Path)) -> std::io::Result<()> {
    let mut pending = vec![(source.to_path_buf(), target.to_path_buf())];

    while let Some((from, to)) = pending.pop() {
        std::fs::create_dir_all(&to)?;
        for entry in std::fs::read_dir(&from)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            let child_to = to.join(entry.file_name());

            if kind.is_dir() {
                pending.push((entry.path(), child_to));
            } else if kind.is_file() {
                std::fs::copy(entry.path(), &child_to)?;
                on_file(&child_to);
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
async fn link_dir(source: &Path, link: &Path) -> CmdResult<()> {
    // 沒開「開發人員模式」就沒有建立符號連結的權限；目錄連接點（junction）不用權限，
    // 對遊戲目錄來說效果一樣，所以失敗時就退而求其次。
    if std::os::windows::fs::symlink_dir(source, link).is_ok() {
        return Ok(());
    }

    let mut command = tokio::process::Command::new("cmd");
    command.args(["/C", "mklink", "/J"]).arg(link).arg(source);
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command
        .output()
        .await
        .map_err(|e| CmdError::launcher(format!("無法建立連結: {}", e)))?;

    if !output.status.success() {
        return Err(CmdError::launcher(format!(
            "無法建立連結: {}",
            String::from_utf8_lossy(&output.stdout).trim()
        )));
    }
    Ok(())
}

#[cfg(not(windows))]
async fn link_dir(source: &Path, link: &Path) -> CmdResult<()> {
    std::os::unix::fs::symlink(source, link)
        .map_err(|e| CmdError::launcher(format!("無法建立連結: {}", e)))
}

/// 這個路徑本身是不是連結。
///
/// Windows 上 `FileType::is_symlink` 只認符號連結，不認目錄連接點（junction），
/// 但我們兩種都會建，所以直接看 reparse point 屬性。刪實例時要靠這個判斷，
/// 否則會順著連結把原本啟動器的遊戲目錄一起刪掉。
pub(super) fn is_link(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
        meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

/// 拆掉連結本身，不碰它指向的東西。Windows 的目錄連結要用 `remove_dir`，
/// Unix 的符號連結要用 `remove_file`。
pub(super) async fn unlink(path: &Path) -> CmdResult<()> {
    if tokio::fs::remove_dir(path).await.is_ok() {
        return Ok(());
    }
    Ok(tokio::fs::remove_file(path).await?)
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
fn instances_dir(path: &Path) -> Option<PathBuf> {
    let nested = path.join("instances");
    if nested.is_dir() {
        return Some(nested);
    }
    if path.file_name().is_some_and(|n| n == "instances") && path.is_dir() {
        return Some(path.to_path_buf());
    }
    None
}

async fn count_instances(instances: &Path) -> usize {
    let Ok(mut entries) = tokio::fs::read_dir(instances).await else {
        return 0;
    };

    let mut count = 0;
    while let Ok(Some(entry)) = entries.next_entry().await {
        if is_foreign_instance(&entry.path()) {
            count += 1;
        }
    }
    count
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

fn folder_name(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "匯入的實例".to_string())
}

async fn read_foreign(dir: &Path) -> Option<ForeignInstance> {
    if !is_foreign_instance(dir) {
        return None;
    }

    let cfg = read_cfg(dir).await;
    let pack = read_pack(dir).await;
    let game_dir = foreign_game_dir(dir);

    Some(ForeignInstance {
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
    })
}

fn count_mods(game_dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(game_dir.join("mods")) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .ends_with(".jar")
        })
        .count()
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
