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
    InstanceLocal, InstanceMeta, InstanceSettings, LocalStore,
};
use crate::store::now_secs;

/// 每複製幾個檔案回報一次進度，免得檔案很多時事件量爆掉。
const PROGRESS_EVERY: usize = 64;


mod curseforge;
mod mmc;
mod modrinth;
mod vanilla;

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

pub(super) struct Candidate {
    kind: &'static str,
    name: &'static str,
    path: PathBuf,
}

pub(super) struct Found {
    info: ForeignInstance,
    game_dir: Option<PathBuf>,
    exclude: Vec<String>,
    icon: String,
    custom_icon: Option<PathBuf>,
    settings: InstanceSettings,
    java_path: Option<String>,
}

enum Root {
    Mmc(PathBuf),
    Vanilla(PathBuf),
    CurseForge(PathBuf),
    Modrinth(PathBuf),
}

fn detect_root(path: &Path) -> Option<Root> {
    modrinth::data_dir(path)
        .map(Root::Modrinth)
        .or_else(|| curseforge::instances_dir(path).map(Root::CurseForge))
        .or_else(|| mmc::instances_dir(path).map(Root::Mmc))
        .or_else(|| vanilla::minecraft_root(path).map(Root::Vanilla))
}

async fn list_root(root: &Root) -> Vec<Found> {
    match root {
        Root::Mmc(dir) => mmc::list(dir).await,
        Root::Vanilla(dir) => vanilla::list(dir).await,
        Root::CurseForge(dir) => curseforge::list(dir).await,
        Root::Modrinth(dir) => modrinth::list(dir).await,
    }
}

async fn read_any(dir: &Path) -> Option<Found> {
    if let Some(found) = mmc::read(dir).await {
        return Some(found);
    }
    if let Some(found) = curseforge::read(dir).await {
        return Some(found);
    }
    if let Some(found) = vanilla::read(dir).await {
        return Some(found);
    }
    modrinth::read(dir).await
}

/// 掃預設安裝位置，列出找得到的啟動器。找不到不算錯誤，回空陣列讓使用者自己指定。
#[tauri::command]
pub async fn detect_foreign_launchers() -> CmdResult<Vec<ForeignLauncher>> {
    let candidates = mmc::known()
        .into_iter()
        .chain(vanilla::known())
        .chain(curseforge::known())
        .chain(modrinth::known());

    let mut found: Vec<ForeignLauncher> = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    for Candidate { kind, name, path } in candidates {
        let Some(root) = detect_root(&path) else {
            continue;
        };
        let path = path_string(&path);
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
            instance_count: list_root(&root).await.len(),
        });
    }
    Ok(found)
}

/// 列出一個啟動器裡的實例。`path` 是啟動器的資料目錄，或直接是放實例的資料夾。
#[tauri::command]
pub async fn list_foreign_instances(path: String) -> CmdResult<Vec<ForeignInstance>> {
    let root = detect_root(Path::new(path.trim())).ok_or_else(|| {
        CmdError::new(
            "notLauncherDir",
            "這個資料夾裡找不到可以匯入的實例，請選 Prism / MultiMC 的資料目錄、.minecraft 資料夾、CurseForge 的 Instances 資料夾或 Modrinth App 的資料目錄",
        )
    })?;

    let mut result: Vec<ForeignInstance> =
        list_root(&root).await.into_iter().map(|f| f.info).collect();
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
        let found = read_any(&source).await;
        let name = found
            .as_ref()
            .map(|f| f.info.name.clone())
            .unwrap_or_else(|| folder_name(&source));

        let result = match found {
            Some(found) => import_one(&app, &base, found, mode, &group).await,
            None => Err(CmdError::new(
                "notLauncherDir",
                "找不到這個實例，可能已經在原本的啟動器裡被刪除",
            )),
        };
        match result {
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
    found: Found,
    mode: ImportMode,
    group: &str,
) -> CmdResult<InstanceView> {
    let Found {
        info,
        game_dir,
        exclude,
        icon,
        custom_icon,
        settings,
        java_path,
    } = found;

    let game_dir = game_dir.ok_or_else(|| {
        CmdError::launcher("這個實例還沒有遊戲目錄（.minecraft），先在原本的啟動器裡啟動一次")
    })?;

    // 連結到自己的資料目錄底下會變成無限巢狀，直接擋掉。
    if mode == ImportMode::Link && game_dir.starts_with(base) {
        return Err(CmdError::launcher(
            "這個實例已經在啟動器的資料目錄裡，不能再建立連結",
        ));
    }

    let id = unique_instance_id(base, &info.name).await?;
    let dir = instance_dir(base, &id)?;
    let target = minecraft_dir(base, &id)?;
    tokio::fs::create_dir_all(&dir).await?;

    // 失敗就把建到一半的資料夾收掉，不要留下半個實例。
    if let Err(e) = place_game_dir(app, &id, &game_dir, &target, mode, &exclude).await {
        let _ = tokio::fs::remove_dir_all(&dir).await;
        return Err(e);
    }

    let mut meta = InstanceMeta {
        name: info.name,
        icon,
        group: group.to_string(),
        notes: info.notes,
        mc_version: info.mc_version.clone(),
        loader: info.loader,
        loader_version: full_loader_version(info.loader, &info.mc_version, &info.loader_version)
            .await,
        created_at: now_secs(),
        last_played: info.last_played,
        play_time_seconds: info.play_time_seconds,
        settings,
        ..InstanceMeta::default()
    };
    if let Some(file) = custom_icon {
        if super::icon::copy_icon(&dir, &file).await.is_ok() {
            meta.icon = super::icon::CUSTOM_ICON.to_string();
        }
    }
    meta.save(&dir).await?;

    if let Some(java_path) = java_path {
        let mut local = LocalStore::load().await?;
        local.set(base, &id, InstanceLocal { java_path });
        local.save().await?;
    }

    build_view(base, &id, &LocalStore::load().await?).await
}

async fn place_game_dir(
    app: &tauri::AppHandle,
    id: &str,
    source: &Path,
    target: &Path,
    mode: ImportMode,
    exclude: &[String],
) -> CmdResult<()> {
    if mode == ImportMode::Link {
        return link_dir(source, target).await;
    }

    let app = app.clone();
    let id = id.to_string();
    let source = source.to_path_buf();
    let target = target.to_path_buf();
    let exclude = exclude.to_vec();

    tokio::task::spawn_blocking(move || {
        let total = count_files(&source, &exclude);
        let mut done = 0;

        copy_tree(&source, &target, &exclude, &mut |_| {
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

fn count_files(root: &Path, exclude: &[String]) -> usize {
    let mut pending = vec![root.to_path_buf()];
    let mut total = 0;

    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if dir == root && is_excluded(&entry, exclude) {
                continue;
            }
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
pub(super) fn copy_tree(
    source: &Path,
    target: &Path,
    exclude: &[String],
    on_file: &mut dyn FnMut(&Path),
) -> std::io::Result<()> {
    let mut pending = vec![(source.to_path_buf(), target.to_path_buf())];

    while let Some((from, to)) = pending.pop() {
        std::fs::create_dir_all(&to)?;
        for entry in std::fs::read_dir(&from)? {
            let entry = entry?;
            if from == source && is_excluded(&entry, exclude) {
                continue;
            }
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

fn is_excluded(entry: &std::fs::DirEntry, exclude: &[String]) -> bool {
    let name = entry.file_name().to_string_lossy().to_string();
    exclude.iter().any(|e| e.eq_ignore_ascii_case(&name))
}

fn folder_name(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "匯入的實例".to_string())
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

#[cfg(windows)]
pub(super) async fn link_dir(source: &Path, link: &Path) -> CmdResult<()> {
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
pub(super) async fn link_dir(source: &Path, link: &Path) -> CmdResult<()> {
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

