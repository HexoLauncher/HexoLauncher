use std::path::{Path, PathBuf};

use base64::Engine;
use hexomc_lib::{
    detect_maps, detect_resource_packs, detect_servers, detect_shader_packs, dir_size, GameMode,
    ServerEntry,
};
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use super::{minecraft_dir, path_string, validate_file_name as check_name};
use crate::error::{CmdError, CmdResult};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackDto {
    pub icon: Option<String>,
    pub name: String,
    pub path: String,
    pub file_name: String,
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
    pub game_mode: Option<GameMode>,
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

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

fn pack_icon(path: &Path, is_folder: bool) -> Option<String> {
    const MAX_BYTES: u64 = 4 * 1024 * 1024;
    for name in [
        "pack.png",
        "icon.png",
        "shaders/pack.png",
        "shaders/icon.png",
    ] {
        let bytes = if is_folder {
            std::fs::File::open(path.join(name)).ok().and_then(|file| {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(
                    &mut std::io::Read::take(file, MAX_BYTES + 1),
                    &mut bytes,
                )
                .ok()?;
                (bytes.len() as u64 <= MAX_BYTES).then_some(bytes)
            })
        } else {
            hexomc_lib::read_mod_icon(path, name, MAX_BYTES)
        };
        if let Some(bytes) = bytes.filter(|bytes| bytes.starts_with(b"\x89PNG\r\n\x1a\n")) {
            return Some(format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            ));
        }
    }
    None
}

fn pack_dto(name: String, path: &Path, is_folder: bool, active: bool) -> PackDto {
    let meta = std::fs::metadata(path).ok();
    PackDto {
        icon: pack_icon(path, is_folder),
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

fn png_data_url(base64: &str) -> String {
    format!("data:image/png;base64,{}", base64)
}

#[tauri::command]
pub async fn list_worlds(base_dir: String, instance_name: String) -> CmdResult<Vec<WorldDto>> {
    let mc = minecraft_dir(Path::new(&base_dir), &instance_name)?;
    tokio::task::spawn_blocking(move || {
        detect_maps(&mc)
            .into_iter()
            .map(|world| WorldDto {
                name: world.name,
                folder: world.folder,
                path: path_string(&world.path),
                icon: world.icon.map(|bytes| {
                    png_data_url(&base64::engine::general_purpose::STANDARD.encode(bytes))
                }),
                game_mode: world.game_mode,
                hardcore: world.hardcore,
                cheats: world.cheats,
                version: world.version,
                last_played: world.last_played,
                size: world.size,
            })
            .collect()
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

#[tauri::command]
pub async fn list_servers(base_dir: String, instance_name: String) -> CmdResult<Vec<ServerEntry>> {
    let mc = minecraft_dir(Path::new(&base_dir), &instance_name)?;
    let mut servers = tokio::task::spawn_blocking(move || detect_servers(&mc))
        .await
        .map_err(|e| CmdError::launcher(e.to_string()))?;
    for server in &mut servers {
        server.icon = server.icon.as_deref().map(png_data_url);
    }
    Ok(servers)
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
}
