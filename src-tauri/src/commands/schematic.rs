use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};

use base64::Engine;
use serde::Serialize;

use super::blueprint::safe_file_name;
use super::{instance_dir, minecraft_dir, path_string};
use crate::error::{CmdError, CmdResult};
use crate::store::store_dir;

const MAX_PREVIEW_BYTES: u64 = 64 * 1024 * 1024;

pub fn block_assets_dir() -> CmdResult<PathBuf> {
    Ok(store_dir()?.join("block-assets"))
}

fn schematics_dir(base_dir: &str, instance_name: &str) -> CmdResult<PathBuf> {
    Ok(minecraft_dir(Path::new(base_dir), instance_name)?.join("schematics"))
}

fn is_schematic(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".litematic") || lower.ends_with(".schem")
}

fn join_relative(root: &Path, rel: &str) -> CmdResult<PathBuf> {
    let rel_path = Path::new(rel);
    let ok = !rel.is_empty()
        && rel_path
            .components()
            .all(|c| matches!(c, Component::Normal(_)));
    if !ok {
        return Err(CmdError::launcher(format!("路徑不合法: {}", rel)));
    }
    Ok(root.join(rel_path))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchematicFile {
    pub name: String,
    /// 相對 `schematics/`，用 `/` 分隔（Litematica 可以放子資料夾）。
    pub rel_path: String,
    pub path: String,
    pub format: &'static str,
    pub size: u64,
    pub modified: u64,
}

fn collect(root: &Path, dir: &Path, depth: usize, out: &mut Vec<SchematicFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            if depth < 6 {
                collect(root, &path, depth + 1, out);
            }
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !meta.is_file() || !is_schematic(&name) {
            continue;
        }
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("/");
        out.push(SchematicFile {
            format: if name.to_ascii_lowercase().ends_with(".schem") {
                "schem"
            } else {
                "litematic"
            },
            name,
            rel_path,
            path: path_string(&path),
            size: meta.len(),
            modified: meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or_default(),
        });
    }
}

#[tauri::command]
pub async fn list_schematics(
    base_dir: String,
    instance_name: String,
) -> CmdResult<Vec<SchematicFile>> {
    let root = schematics_dir(&base_dir, &instance_name)?;
    let mut files = Vec::new();
    collect(&root, &root, 0, &mut files);
    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(files)
}

#[tauri::command]
pub async fn delete_schematic(
    base_dir: String,
    instance_name: String,
    rel_path: String,
) -> CmdResult<()> {
    let root = schematics_dir(&base_dir, &instance_name)?;
    let path = join_relative(&root, &rel_path)?;
    if !is_schematic(&rel_path) {
        return Err(CmdError::launcher(format!("不是藍圖檔: {}", rel_path)));
    }
    tokio::fs::remove_file(&path).await?;
    Ok(())
}

#[tauri::command]
pub async fn import_schematic(
    base_dir: String,
    instance_name: String,
    source: String,
) -> CmdResult<SchematicFile> {
    let source = PathBuf::from(source);
    let name = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if !is_schematic(&name) {
        return Err(CmdError::launcher("只能匯入 .litematic 或 .schem"));
    }

    let root = schematics_dir(&base_dir, &instance_name)?;
    tokio::fs::create_dir_all(&root).await?;
    let file_name = safe_file_name(&name, "schematic.litematic");
    let mut target = root.join(&file_name);
    if target.exists() {
        let (stem, ext) = file_name.rsplit_once('.').unwrap_or((&file_name, ""));
        target = (2..)
            .map(|n| root.join(format!("{} ({}).{}", stem, n, ext)))
            .find(|p| !p.exists())
            .unwrap_or(target);
    }
    tokio::fs::copy(&source, &target).await?;

    let mut found = Vec::new();
    collect(&root, &root, 0, &mut found);
    found
        .into_iter()
        .find(|f| Path::new(&f.path) == target)
        .ok_or_else(|| CmdError::launcher("複製後找不到檔案"))
}

#[tauri::command]
pub async fn read_schematic(path: String) -> CmdResult<String> {
    if !path.to_ascii_lowercase().ends_with(".litematic") {
        return Err(CmdError::launcher("目前只能預覽 .litematic"));
    }
    let meta = tokio::fs::metadata(&path).await?;
    if meta.len() > MAX_PREVIEW_BYTES {
        return Err(CmdError::launcher("檔案太大，沒辦法預覽"));
    }
    let bytes = tokio::fs::read(&path).await?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

fn is_block_asset(path: &str) -> bool {
    if path == "pack.mcmeta" {
        return true;
    }
    let Some(rest) = path.strip_prefix("assets/") else {
        return false;
    };
    let Some((_, rest)) = rest.split_once('/') else {
        return false;
    };
    rest.starts_with("blockstates/")
        || rest.starts_with("models/block/")
        || (rest.starts_with("textures/block/")
            && (rest.ends_with(".png") || rest.ends_with(".mcmeta")))
}

fn slim_from_zip(source: &Path) -> CmdResult<Vec<u8>> {
    let file = std::fs::File::open(source)?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(file))
        .map_err(|e| CmdError::new("zip", e.to_string()))?;
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let entry = archive
            .by_index_raw(i)
            .map_err(|e| CmdError::new("zip", e.to_string()))?;
        if entry.is_dir() || !is_block_asset(entry.name()) {
            continue;
        }
        writer
            .raw_copy_file(entry)
            .map_err(|e| CmdError::new("zip", e.to_string()))?;
    }
    let cursor = writer
        .finish()
        .map_err(|e| CmdError::new("zip", e.to_string()))?;
    Ok(cursor.into_inner())
}

fn add_folder(
    writer: &mut zip::ZipWriter<Cursor<Vec<u8>>>,
    root: &Path,
    dir: &Path,
) -> CmdResult<()> {
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for entry in std::fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            add_folder(writer, root, &path)?;
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("/");
        if !is_block_asset(&rel) {
            continue;
        }
        let mut data = Vec::new();
        std::fs::File::open(&path)?.read_to_end(&mut data)?;
        writer
            .start_file(rel, options)
            .map_err(|e| CmdError::new("zip", e.to_string()))?;
        writer.write_all(&data)?;
    }
    Ok(())
}

fn slim_from_folder(source: &Path) -> CmdResult<Vec<u8>> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    add_folder(&mut writer, source, source)?;
    let cursor = writer
        .finish()
        .map_err(|e| CmdError::new("zip", e.to_string()))?;
    Ok(cursor.into_inner())
}

async fn slim_pack(source: PathBuf, prefix: &str) -> CmdResult<String> {
    let meta = tokio::fs::metadata(&source).await?;
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or_default();

    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    meta.len().hash(&mut hasher);
    modified.hash(&mut hasher);
    let dir = block_assets_dir()?;
    let target = dir.join(format!("{}-{:016x}.zip", prefix, hasher.finish()));
    if target.exists() {
        return Ok(path_string(&target));
    }

    let is_dir = meta.is_dir();
    let source_for_task = source.clone();
    let bytes = tokio::task::spawn_blocking(move || {
        if is_dir {
            slim_from_folder(&source_for_task)
        } else {
            slim_from_zip(&source_for_task)
        }
    })
    .await
    .map_err(|e| CmdError::launcher(e.to_string()))??;

    tokio::fs::create_dir_all(&dir).await?;
    let tmp = target.with_extension("zip.tmp");
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(&tmp, &target).await?;
    Ok(path_string(&target))
}

#[tauri::command]
pub async fn local_block_assets(
    base_dir: String,
    version: String,
    prefer: Option<String>,
) -> CmdResult<Option<String>> {
    let jar_name = format!("{}.jar", safe_file_name(&version, "_"));
    let base = PathBuf::from(&base_dir);

    let mut candidates = Vec::new();
    if let Some(name) = prefer.as_deref().filter(|n| !n.is_empty()) {
        candidates.push(instance_dir(&base, name)?.join(&jar_name));
    }
    if let Ok(entries) = std::fs::read_dir(base.join("instance")) {
        candidates.extend(entries.flatten().map(|e| e.path().join(&jar_name)));
    }

    for jar in candidates {
        if jar.is_file() {
            return slim_pack(jar, &format!("vanilla-{}", safe_file_name(&version, "_")))
                .await
                .map(Some);
        }
    }
    Ok(None)
}

#[tauri::command]
pub async fn pack_block_assets(path: String) -> CmdResult<String> {
    slim_pack(PathBuf::from(path), "pack").await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_cannot_escape() {
        let root = Path::new("/base/schematics");
        assert_eq!(
            join_relative(root, "farms/sugar.litematic").unwrap(),
            root.join("farms/sugar.litematic")
        );
        for bad in ["", "../x.litematic", "a/../../x.litematic", "/etc/passwd"] {
            assert!(join_relative(root, bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn keeps_only_block_assets() {
        assert!(is_block_asset("pack.mcmeta"));
        assert!(is_block_asset("assets/minecraft/blockstates/stone.json"));
        assert!(is_block_asset("assets/minecraft/models/block/stone.json"));
        assert!(is_block_asset("assets/minecraft/textures/block/stone.png"));
        assert!(is_block_asset(
            "assets/create/textures/block/gear.png.mcmeta"
        ));
        assert!(!is_block_asset("assets/minecraft/textures/item/apple.png"));
        assert!(!is_block_asset("assets/minecraft/sounds.json"));
        assert!(!is_block_asset("net/minecraft/client/Main.class"));
    }

    #[test]
    fn slims_zip_and_folder_packs() {
        let dir = std::env::temp_dir().join(format!("nolauncher-slim-{}", uuid::Uuid::new_v4()));
        let folder = dir.join("pack");
        std::fs::create_dir_all(folder.join("assets/minecraft/textures/block")).unwrap();
        std::fs::create_dir_all(folder.join("assets/minecraft/sounds")).unwrap();
        std::fs::write(folder.join("pack.mcmeta"), "{}").unwrap();
        std::fs::write(
            folder.join("assets/minecraft/textures/block/stone.png"),
            [1, 2, 3],
        )
        .unwrap();
        std::fs::write(folder.join("assets/minecraft/sounds/a.ogg"), [9]).unwrap();

        let names = |bytes: Vec<u8>| {
            let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
            let mut names: Vec<String> = (0..archive.len())
                .map(|i| archive.by_index(i).unwrap().name().to_string())
                .collect();
            names.sort();
            names
        };
        let expected = vec![
            "assets/minecraft/textures/block/stone.png".to_string(),
            "pack.mcmeta".to_string(),
        ];

        let from_folder = slim_from_folder(&folder).unwrap();
        assert_eq!(names(from_folder.clone()), expected);

        // 用抽出來的 zip 再抽一次，走 zip 的路徑（raw_copy_file）
        let zip_path = dir.join("pack.zip");
        std::fs::write(&zip_path, &from_folder).unwrap();
        assert_eq!(names(slim_from_zip(&zip_path).unwrap()), expected);

        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod real_jar {
    #[test]
    #[ignore = "需要本機的 client jar：NOLAUNCHER_TEST_JAR=/path/to/1.21.1.jar"]
    fn slims_real_client_jar() {
        let jar = std::env::var("NOLAUNCHER_TEST_JAR").expect("NOLAUNCHER_TEST_JAR");
        let started = std::time::Instant::now();
        let bytes = super::slim_from_zip(std::path::Path::new(&jar)).unwrap();
        let archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.clone())).unwrap();
        println!(
            "{} entries, {:.1} MB, {:?}",
            archive.len(),
            bytes.len() as f64 / 1e6,
            started.elapsed()
        );
        assert!(archive.len() > 1000);
    }
}
