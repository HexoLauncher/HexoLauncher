use std::path::{Path, PathBuf};

use base64::Engine;
use hexomc_lib::{detect_resource_packs, detect_shader_packs};
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use super::{minecraft_dir, path_string, validate_file_name as check_name};
use crate::error::{CmdError, CmdResult};
use hexomc_lib::assets::nbt::{self, Nbt};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackDto {
    pub name: String,
    pub path: String,
    pub file_name: String,
    /// true = folder，false = .zip
    pub is_folder: bool,
    pub size: u64,
    pub modified: u64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldDto {
    pub name: String,
    pub folder: String,
    pub path: String,
    pub icon: Option<String>,
    pub game_mode: Option<String>,
    pub hardcore: bool,
    pub cheats: bool,
    pub version: Option<String>,
    pub last_played: u64,
    pub size: u64,
}

fn modified_secs(meta: &std::fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

fn dir_size(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.file_type() {
            Ok(t) if t.is_dir() => dir_size(&entry.path()),
            Ok(t) if t.is_file() => entry.metadata().map(|m| m.len()).unwrap_or_default(),
            _ => 0,
        })
        .sum()
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

fn pack_dto(name: String, path: &Path, is_folder: bool, active: bool) -> PackDto {
    let meta = std::fs::metadata(path).ok();
    PackDto {
        name,
        path: path_string(path),
        file_name: file_name_of(path),
        is_folder,
        size: if is_folder {
            dir_size(path)
        } else {
            meta.as_ref().map(|m| m.len()).unwrap_or_default()
        },
        modified: meta.as_ref().map(modified_secs).unwrap_or_default(),
        active,
    }
}

fn sort_packs(packs: &mut [PackDto]) {
    packs.sort_by(|a, b| {
        b.active
            .cmp(&a.active)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

fn enabled_resource_packs(mc: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(mc.join("options.txt")) else {
        return Vec::new();
    };
    text.lines()
        .find_map(|line| line.strip_prefix("resourcePacks:"))
        .and_then(|list| serde_json::from_str::<Vec<String>>(list.trim()).ok())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|id| id.strip_prefix("file/").map(str::to_string))
        .collect()
}

#[tauri::command]
pub async fn list_resource_packs(
    base_dir: String,
    instance_name: String,
) -> CmdResult<Vec<PackDto>> {
    let dir = minecraft_dir(Path::new(&base_dir), &instance_name)?;
    tokio::task::spawn_blocking(move || {
        let enabled = enabled_resource_packs(&dir);
        let mut packs: Vec<PackDto> = detect_resource_packs(&dir)
            .into_iter()
            .map(|p| {
                let active = enabled.contains(&file_name_of(&p.path));
                pack_dto(p.name, &p.path, p.is_folder, active)
            })
            .collect();
        sort_packs(&mut packs);
        packs
    })
    .await
    .map_err(|e| CmdError::launcher(e.to_string()))
}

const SHADER_PROPERTIES: [&str; 2] = ["config/iris.properties", "config/oculus.properties"];
const OPTIFINE_SHADERS: &str = "optionsshaders.txt";

fn unescape_property(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn escape_property(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        if matches!(c, '\\' | ':' | '=' | '#' | '!') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn property(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (k, v) = line.split_once('=')?;
        (k.trim() == key).then(|| unescape_property(v.trim()))
    })
}

fn set_property(text: &str, key: &str, value: &str) -> String {
    let line = format!("{}={}", key, value);
    let mut found = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|l| match l.split_once('=') {
            Some((k, _)) if k.trim() == key => {
                found = true;
                line.clone()
            }
            _ => l.to_string(),
        })
        .collect();
    if !found {
        lines.push(line);
    }
    lines.join("\n") + "\n"
}

fn active_shader(mc: &Path) -> Option<String> {
    for file in SHADER_PROPERTIES {
        if let Ok(text) = std::fs::read_to_string(mc.join(file)) {
            if property(&text, "enableShaders").as_deref() == Some("false") {
                return None;
            }
            return property(&text, "shaderPack").filter(|s| !s.is_empty());
        }
    }
    let text = std::fs::read_to_string(mc.join(OPTIFINE_SHADERS)).ok()?;
    property(&text, "shaderPack").filter(|s| !s.is_empty() && s != "OFF" && s != "(internal)")
}

#[tauri::command]
pub async fn list_shader_packs(base_dir: String, instance_name: String) -> CmdResult<Vec<PackDto>> {
    let dir = minecraft_dir(Path::new(&base_dir), &instance_name)?;
    tokio::task::spawn_blocking(move || {
        let active = active_shader(&dir);
        let mut packs: Vec<PackDto> = detect_shader_packs(&dir)
            .into_iter()
            .map(|p| {
                let is_active = active.as_deref() == Some(file_name_of(&p.path).as_str());
                pack_dto(p.name, &p.path, p.is_folder, is_active)
            })
            .collect();
        sort_packs(&mut packs);
        packs
    })
    .await
    .map_err(|e| CmdError::launcher(e.to_string()))
}

#[tauri::command]
pub async fn set_active_shader(
    base_dir: String,
    instance_name: String,
    file_name: Option<String>,
) -> CmdResult<()> {
    if let Some(name) = &file_name {
        check_name(name)?;
    }
    let mc = minecraft_dir(Path::new(&base_dir), &instance_name)?;

    let mut targets: Vec<PathBuf> = SHADER_PROPERTIES
        .iter()
        .chain(std::iter::once(&OPTIFINE_SHADERS))
        .map(|f| mc.join(f))
        .filter(|p| p.exists())
        .collect();
    if targets.is_empty() {
        let oculus = std::fs::read_dir(mc.join("mods")).is_ok_and(|entries| {
            entries.flatten().any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .contains("oculus")
            })
        });
        targets.push(mc.join(SHADER_PROPERTIES[usize::from(oculus)]));
    }

    for path in targets {
        let text = tokio::fs::read_to_string(&path).await.unwrap_or_default();
        let text = if path.ends_with(OPTIFINE_SHADERS) {
            set_property(&text, "shaderPack", file_name.as_deref().unwrap_or("OFF"))
        } else {
            let text = match &file_name {
                Some(name) => set_property(&text, "shaderPack", &escape_property(name)),
                None => text,
            };
            set_property(
                &text,
                "enableShaders",
                if file_name.is_some() { "true" } else { "false" },
            )
        };
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, text).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_pack(
    base_dir: String,
    instance_name: String,
    kind: String,
    file_name: String,
) -> CmdResult<()> {
    check_name(&file_name)?;
    if kind != "resourcepacks" && kind != "shaderpacks" {
        return Err(CmdError::launcher(format!("未知的類型: {}", kind)));
    }
    let path = minecraft_dir(Path::new(&base_dir), &instance_name)?
        .join(&kind)
        .join(&file_name);
    let meta = tokio::fs::symlink_metadata(&path)
        .await
        .map_err(|_| CmdError::launcher(format!("找不到檔案: {}", file_name)))?;
    if meta.is_dir() {
        tokio::fs::remove_dir_all(&path).await?;
    } else {
        tokio::fs::remove_file(&path).await?;
    }
    Ok(())
}

fn int(value: &Nbt) -> Option<i64> {
    match value {
        Nbt::Byte(v) => Some(*v as i64),
        Nbt::Short(v) => Some(*v as i64),
        Nbt::Int(v) => Some(*v as i64),
        Nbt::Long(v) => Some(*v),
        _ => None,
    }
}

fn read_world(path: &Path) -> Option<WorldDto> {
    let bytes = std::fs::read(path.join("level.dat")).ok()?;
    let folder = file_name_of(path);
    let root = nbt::parse(&bytes);
    let data = root.as_ref().and_then(|r| r.get("Data"));
    let get = |key: &str| data.and_then(|d| d.get(key));

    Some(WorldDto {
        name: get("LevelName")
            .and_then(Nbt::as_str)
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(&folder)
            .to_string(),
        path: path_string(path),
        icon: std::fs::read(path.join("icon.png")).ok().map(|bytes| {
            format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            )
        }),
        game_mode: get("GameType").and_then(int).map(|mode| {
            match mode {
                0 => "survival",
                1 => "creative",
                2 => "adventure",
                3 => "spectator",
                _ => "unknown",
            }
            .to_string()
        }),
        hardcore: get("hardcore").and_then(int).unwrap_or_default() != 0,
        cheats: get("allowCommands").and_then(int).unwrap_or_default() != 0,
        version: get("Version")
            .and_then(|v| v.get("Name"))
            .and_then(Nbt::as_str)
            .map(str::to_string),
        last_played: get("LastPlayed")
            .and_then(int)
            .map(|ms| (ms / 1000).max(0) as u64)
            .unwrap_or_default(),
        size: dir_size(path),
        folder,
    })
}

#[tauri::command]
pub async fn list_worlds(base_dir: String, instance_name: String) -> CmdResult<Vec<WorldDto>> {
    let saves = minecraft_dir(Path::new(&base_dir), &instance_name)?.join("saves");
    tokio::task::spawn_blocking(move || {
        let Ok(entries) = std::fs::read_dir(&saves) else {
            return Vec::new();
        };
        let mut worlds: Vec<WorldDto> = entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter_map(|e| read_world(&e.path()))
            .collect();
        worlds.sort_by(|a, b| b.last_played.cmp(&a.last_played));
        worlds
    })
    .await
    .map_err(|e| CmdError::launcher(e.to_string()))
}

fn world_dir(base_dir: &str, instance_name: &str, folder: &str) -> CmdResult<PathBuf> {
    check_name(folder)?;
    let path = minecraft_dir(Path::new(base_dir), instance_name)?
        .join("saves")
        .join(folder);
    if !path.join("level.dat").exists() {
        return Err(CmdError::launcher(format!("找不到世界: {}", folder)));
    }
    Ok(path)
}

#[tauri::command]
pub async fn open_world_folder(
    base_dir: String,
    instance_name: String,
    folder: String,
) -> CmdResult<()> {
    let path = world_dir(&base_dir, &instance_name, &folder)?;
    open::that(&path).map_err(|e| CmdError::launcher(format!("無法開啟資料夾: {}", e)))
}

#[tauri::command]
pub async fn delete_world(
    base_dir: String,
    instance_name: String,
    folder: String,
) -> CmdResult<()> {
    let path = world_dir(&base_dir, &instance_name, &folder)?;
    tokio::fs::remove_dir_all(&path).await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    /// 修改時間（unix 秒）
    pub modified: u64,
}

fn list_files(dir: &Path, accept: impl Fn(&str) -> bool) -> Vec<FileEntry> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut files: Vec<FileEntry> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            if !accept(&name.to_ascii_lowercase()) {
                return None;
            }
            let meta = entry.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            Some(FileEntry {
                name,
                path: path_string(&entry.path()),
                size: meta.len(),
                modified: modified_secs(&meta),
            })
        })
        .collect();

    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    files
}

#[tauri::command]
pub async fn list_screenshots(
    base_dir: String,
    instance_name: String,
) -> CmdResult<Vec<FileEntry>> {
    let dir = minecraft_dir(Path::new(&base_dir), &instance_name)?.join("screenshots");
    Ok(list_files(&dir, |name| {
        name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg")
    }))
}

#[tauri::command]
pub async fn list_instance_logs(
    base_dir: String,
    instance_name: String,
) -> CmdResult<Vec<FileEntry>> {
    let dir = minecraft_dir(Path::new(&base_dir), &instance_name)?.join("logs");
    Ok(list_files(&dir, |name| {
        name.ends_with(".log") || name.ends_with(".txt")
    }))
}

const LOG_TAIL_BYTES: u64 = 512 * 1024;

#[tauri::command]
pub async fn read_instance_log(
    base_dir: String,
    instance_name: String,
    file_name: String,
) -> CmdResult<String> {
    check_name(&file_name)?;

    let path = minecraft_dir(Path::new(&base_dir), &instance_name)?
        .join("logs")
        .join(&file_name);

    let mut file = tokio::fs::File::open(&path).await?;
    let len = file.metadata().await?.len();
    file.seek(std::io::SeekFrom::Start(len.saturating_sub(LOG_TAIL_BYTES)))
        .await?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).await?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerDto {
    pub name: String,
    pub address: String,

    pub icon: Option<String>,
    pub accept_textures: bool,
}

#[tauri::command]
pub async fn list_servers(base_dir: String, instance_name: String) -> CmdResult<Vec<ServerDto>> {
    let path = minecraft_dir(Path::new(&base_dir), &instance_name)?.join("servers.dat");

    let Ok(bytes) = tokio::fs::read(&path).await else {
        return Ok(Vec::new());
    };
    let Some(root) = nbt::parse(&bytes) else {
        return Ok(Vec::new());
    };
    let Some(servers) = root.get("servers").and_then(Nbt::as_list) else {
        return Ok(Vec::new());
    };

    Ok(servers
        .iter()
        .map(|entry| ServerDto {
            name: entry
                .get("name")
                .and_then(Nbt::as_str)
                .unwrap_or_default()
                .to_string(),
            address: entry
                .get("ip")
                .and_then(Nbt::as_str)
                .unwrap_or_default()
                .to_string(),
            icon: entry
                .get("icon")
                .and_then(Nbt::as_str)
                .filter(|s| !s.is_empty())
                .map(|s| format!("data:image/png;base64,{}", s)),
            accept_textures: matches!(entry.get("acceptTextures"), Some(Nbt::Byte(1))),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hexo-assets-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn tag(out: &mut Vec<u8>, id: u8, key: &str) {
        out.push(id);
        out.extend((key.len() as u16).to_be_bytes());
        out.extend(key.as_bytes());
    }

    #[test]
    fn property_round_trip_keeps_other_lines() {
        let text = "#comment\nenableShaders=false\nmaxShadowRenderDistance=32\n";
        let text = set_property(text, "shaderPack", &escape_property("BSL v8.2:x.zip"));
        let text = set_property(&text, "enableShaders", "true");
        assert!(text.starts_with("#comment\nenableShaders=true\nmaxShadowRenderDistance=32\n"));
        assert_eq!(
            property(&text, "shaderPack").as_deref(),
            Some("BSL v8.2:x.zip")
        );
    }

    #[test]
    fn active_shader_respects_disabled_flag() {
        let mc = temp_dir("shader");
        std::fs::create_dir_all(mc.join("config")).unwrap();
        std::fs::write(
            mc.join("config/iris.properties"),
            "shaderPack=Complementary.zip\nenableShaders=true\n",
        )
        .unwrap();
        assert_eq!(active_shader(&mc).as_deref(), Some("Complementary.zip"));

        std::fs::write(
            mc.join("config/iris.properties"),
            "shaderPack=Complementary.zip\nenableShaders=false\n",
        )
        .unwrap();
        assert_eq!(active_shader(&mc), None);

        std::fs::remove_file(mc.join("config/iris.properties")).unwrap();
        std::fs::write(mc.join(OPTIFINE_SHADERS), "shaderPack=OFF\n").unwrap();
        assert_eq!(active_shader(&mc), None);
        let _ = std::fs::remove_dir_all(&mc);
    }

    #[test]
    fn enabled_resource_packs_reads_options() {
        let mc = temp_dir("rp");
        std::fs::write(
            mc.join("options.txt"),
            "version:3465\nresourcePacks:[\"vanilla\",\"file/Faithful 32x.zip\"]\n",
        )
        .unwrap();
        assert_eq!(enabled_resource_packs(&mc), ["Faithful 32x.zip"]);
        let _ = std::fs::remove_dir_all(&mc);
    }

    #[test]
    fn read_world_parses_level_dat() {
        let saves = temp_dir("world");
        let world = saves.join("New World (1)");
        std::fs::create_dir_all(&world).unwrap();

        let mut dat = Vec::new();
        tag(&mut dat, 10, "");
        tag(&mut dat, 10, "Data");
        tag(&mut dat, 8, "LevelName");
        dat.extend((5u16).to_be_bytes());
        dat.extend(b"Hello");
        tag(&mut dat, 3, "GameType");
        dat.extend(1i32.to_be_bytes());
        tag(&mut dat, 1, "hardcore");
        dat.push(1);
        tag(&mut dat, 4, "LastPlayed");
        dat.extend(1_700_000_000_123i64.to_be_bytes());
        tag(&mut dat, 10, "Version");
        tag(&mut dat, 8, "Name");
        dat.extend((6u16).to_be_bytes());
        dat.extend(b"1.21.4");
        dat.extend([0, 0, 0]);
        std::fs::write(world.join("level.dat"), &dat).unwrap();

        let info = read_world(&world).unwrap();
        assert_eq!(info.name, "Hello");
        assert_eq!(info.folder, "New World (1)");
        assert_eq!(info.game_mode.as_deref(), Some("creative"));
        assert!(info.hardcore);
        assert!(!info.cheats);
        assert_eq!(info.version.as_deref(), Some("1.21.4"));
        assert_eq!(info.last_played, 1_700_000_000);
        assert_eq!(info.size, dat.len() as u64);
        let _ = std::fs::remove_dir_all(&saves);
    }
}
