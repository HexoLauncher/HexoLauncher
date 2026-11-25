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
/// 精簡素材包的格式版本；`is_block_asset` 收的檔案變了就加一，舊快取才會重做。
const SLIM_VERSION: u32 = 2;

pub fn block_assets_dir() -> CmdResult<PathBuf> {
    Ok(store_dir()?.join("block-assets"))
}

fn schematics_dir(base_dir: &str, instance_name: &str) -> CmdResult<PathBuf> {
    Ok(minecraft_dir(Path::new(base_dir), instance_name)?.join("schematics"))
}

fn is_schematic(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".litematic") || lower.ends_with(".schem") || lower.ends_with(".nbt")
}

fn is_previewable(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".litematic") || lower.ends_with(".nbt")
}

fn join_relative(root: &Path, rel: &str) -> CmdResult<PathBuf> {
    let rel_path = Path::new(rel);
    let ok = !rel.is_empty()
        && rel_path
            .components()
            .all(|c| matches!(c, Component::Normal(_)));
    if !ok {
        return Err(CmdError::launcher_code("invalid_path", &[("path", &rel)]));
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
            format: {
                let lower = name.to_ascii_lowercase();
                if lower.ends_with(".schem") {
                    "schem"
                } else if lower.ends_with(".nbt") {
                    "nbt"
                } else {
                    "litematic"
                }
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
        return Err(CmdError::launcher_code(
            "not_schematic",
            &[("path", &rel_path)],
        ));
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
        return Err(CmdError::launcher_code("import_schematic_format", &[]));
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
        .ok_or_else(|| CmdError::launcher_code("copied_missing", &[]))
}

#[tauri::command]
pub async fn read_schematic(path: String) -> CmdResult<String> {
    if !is_previewable(&path) {
        return Err(CmdError::launcher_code("preview_litematic_only", &[]));
    }
    let meta = tokio::fs::metadata(&path).await?;
    if meta.len() > MAX_PREVIEW_BYTES {
        return Err(CmdError::launcher_code("file_too_big_preview", &[]));
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
    let geo = rest.strip_prefix("geo/").is_some_and(|g| {
        g.ends_with(".geo.json") && !g.contains("entity/") && !g.contains("item/")
    });
    geo || rest.starts_with("blockstates/")
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
    SLIM_VERSION.hash(&mut hasher);
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModBlockAssets {
    /// 這個模組提供的方塊命名空間，例如 `create`。
    pub namespaces: Vec<String>,
    pub name: String,
    /// 精簡過、只剩方塊素材的 zip。
    pub path: String,
    /// `instance` 或 `modrinth`。
    pub source: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModBlockAssetsResult {
    pub found: Vec<ModBlockAssets>,
    /// 本機和 Modrinth 都找不到的命名空間。
    pub missing: Vec<String>,
}

fn is_namespace(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b'.'))
}

fn jar_namespaces(jar: &Path, wanted: &[String]) -> Vec<String> {
    let Ok(file) = std::fs::File::open(jar) else {
        return Vec::new();
    };
    let Ok(archive) = zip::ZipArchive::new(std::io::BufReader::new(file)) else {
        return Vec::new();
    };
    wanted
        .iter()
        .filter(|ns| {
            let prefix = format!("assets/{ns}/blockstates/");
            archive.file_names().any(|n| n.starts_with(&prefix))
        })
        .cloned()
        .collect()
}

fn find_local_mods(
    base_dir: &Path,
    instances: &[String],
    wanted: &[String],
) -> Vec<(PathBuf, Vec<String>)> {
    let mut left: Vec<String> = wanted.to_vec();
    let mut found = Vec::new();
    for name in instances {
        if left.is_empty() {
            break;
        }
        let Ok(dir) = minecraft_dir(base_dir, name) else {
            continue;
        };
        let Ok(entries) = std::fs::read_dir(dir.join("mods")) else {
            continue;
        };
        let mut jars: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("jar")))
            .collect();
        jars.sort();
        for jar in jars {
            let provides = jar_namespaces(&jar, &left);
            if provides.is_empty() {
                continue;
            }
            left.retain(|ns| !provides.contains(ns));
            found.push((jar, provides));
            if left.is_empty() {
                break;
            }
        }
    }
    found
}

#[derive(serde::Deserialize)]
struct ModrinthVersion {
    version_number: String,
    files: Vec<ModrinthFile>,
}

#[derive(serde::Deserialize)]
struct ModrinthFile {
    url: String,
    #[serde(default)]
    primary: bool,
    hashes: ModrinthHashes,
}

#[derive(serde::Deserialize)]
struct ModrinthHashes {
    sha1: String,
}

/// 命名空間和 Modrinth 專案名稱差很多、搜尋也找不到的常見模組。
const MODRINTH_ALIASES: &[(&str, &str)] = &[
    ("sfm", "super-factory-manager"),
    ("extendedae", "extended-ae"),
    ("ae2", "ae2"),
    ("mekanism", "mekanism"),
    ("immersiveengineering", "immersive-engineering"),
    ("botania", "botania"),
    ("thermal", "thermal-expansion"),
];

#[derive(serde::Deserialize)]
struct ModrinthSearch {
    hits: Vec<ModrinthHit>,
}

#[derive(serde::Deserialize)]
struct ModrinthHit {
    slug: String,
}

async fn modrinth_candidates(namespace: &str, mc_version: Option<&str>) -> Vec<String> {
    use super::modpack_browse::{get_json, http};

    let mut slugs: Vec<String> = MODRINTH_ALIASES
        .iter()
        .filter(|(ns, _)| *ns == namespace)
        .map(|(_, slug)| slug.to_string())
        .collect();
    slugs.push(namespace.to_string());
    slugs.push(namespace.replace('_', "-"));
    let mut facets = vec![vec!["project_type:mod".to_string()]];
    if let Some(v) = mc_version.filter(|v| !v.is_empty()) {
        facets.push(vec![format!("versions:{v}")]);
    }
    let facets = serde_json::to_string(&facets).unwrap_or_default();
    for query in [namespace.replace('_', " "), namespace.to_string()] {
        let request = http().get("https://api.modrinth.com/v2/search").query(&[
            ("query", query.as_str()),
            ("limit", "5"),
            ("facets", facets.as_str()),
        ]);
        if let Ok(found) = get_json::<ModrinthSearch>(request).await {
            slugs.extend(found.hits.into_iter().map(|h| h.slug));
        }
    }
    let mut seen = std::collections::HashSet::new();
    slugs.retain(|s| seen.insert(s.clone()));
    slugs
}

async fn remote_jar_has_blocks(url: &str, namespace: &str) -> Option<bool> {
    use super::modpack_browse::http;

    let range = |r: String| async move {
        let response = http()
            .get(url)
            .header(reqwest::header::RANGE, r)
            .send()
            .await
            .ok()?;
        if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return None;
        }
        response.bytes().await.ok()
    };

    let tail = range("bytes=-65558".into()).await?;
    let eocd = tail
        .windows(4)
        .rposition(|w| w == [0x50, 0x4b, 0x05, 0x06])?;
    let field = |at: usize| {
        tail.get(eocd + at..eocd + at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let (size, offset) = (field(12)?, field(16)?);
    if size == u32::MAX || offset == u32::MAX || size == 0 {
        return None;
    }
    let directory = range(format!(
        "bytes={}-{}",
        offset,
        offset as u64 + size as u64 - 1
    ))
    .await?;
    let prefix = format!("assets/{namespace}/blockstates/");
    Some(
        directory
            .windows(prefix.len())
            .any(|w| w == prefix.as_bytes()),
    )
}

/// 從 Modrinth 找提供 `namespace` 方塊的模組並下載，只留下方塊素材。
/// 專案名稱不一定等於命名空間（`ars_nouveau` 是 `ars-nouveau`），所以會試別名、
/// slug 和搜尋結果，並先讀 jar 目錄確認真的有這個命名空間的方塊。
/// 每個專案優先選 `mc_version` 的最新版，沒有就用最新版。
async fn modrinth_mod(
    namespace: &str,
    mc_version: Option<&str>,
) -> CmdResult<Option<ModBlockAssets>> {
    for slug in modrinth_candidates(namespace, mc_version).await {
        if let Some(found) = modrinth_project(namespace, &slug, mc_version).await? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

async fn modrinth_project(
    namespace: &str,
    slug: &str,
    mc_version: Option<&str>,
) -> CmdResult<Option<ModBlockAssets>> {
    use super::modpack_browse::{get_json, http};

    if !is_namespace(&slug.replace('-', "_")) {
        return Ok(None);
    }
    let url = format!("https://api.modrinth.com/v2/project/{slug}/version");
    let mut versions: Vec<ModrinthVersion> = Vec::new();
    if let Some(v) = mc_version.filter(|v| !v.is_empty()) {
        let filter = serde_json::to_string(&[v]).unwrap_or_default();
        match get_json(http().get(&url).query(&[("game_versions", filter)])).await {
            Ok(list) => versions = list,
            // 沒有這個專案。
            Err(_) => return Ok(None),
        }
    }
    if versions.is_empty() {
        versions = match get_json(http().get(&url)).await {
            Ok(list) => list,
            Err(_) => return Ok(None),
        };
    }
    let Some((number, file)) = versions.into_iter().find_map(|v| {
        let i = v.files.iter().position(|f| f.primary).unwrap_or(0);
        v.files.into_iter().nth(i).map(|f| (v.version_number, f))
    }) else {
        return Ok(None);
    };
    if !file.hashes.sha1.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(None);
    }

    let dir = block_assets_dir()?;
    let target = dir.join(format!("modrinth-v{SLIM_VERSION}-{}.zip", file.hashes.sha1));
    if !target.exists() {
        if remote_jar_has_blocks(&file.url, namespace).await == Some(false) {
            return Ok(None);
        }
        let bytes = http()
            .get(&file.url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        tokio::fs::create_dir_all(&dir).await?;
        let jar = target.with_extension("jar.tmp");
        tokio::fs::write(&jar, &bytes).await?;
        let jar_for_task = jar.clone();
        let slim = tokio::task::spawn_blocking(move || slim_from_zip(&jar_for_task))
            .await
            .map_err(|e| CmdError::launcher(e.to_string()));
        let _ = tokio::fs::remove_file(&jar).await;
        let tmp = target.with_extension("zip.tmp");
        tokio::fs::write(&tmp, slim??).await?;
        tokio::fs::rename(&tmp, &target).await?;
    }

    let namespaces = jar_namespaces(&target, &[namespace.to_string()]);
    if namespaces.is_empty() {
        return Ok(None);
    }
    Ok(Some(ModBlockAssets {
        namespaces,
        name: format!("{slug} {number} (Modrinth)"),
        path: path_string(&target),
        source: "modrinth",
    }))
}

#[tauri::command]
pub async fn mod_block_assets(
    base_dir: String,
    instances: Vec<String>,
    namespaces: Vec<String>,
    mc_version: Option<String>,
    download: bool,
) -> CmdResult<ModBlockAssetsResult> {
    let wanted: Vec<String> = namespaces
        .into_iter()
        .filter(|ns| is_namespace(ns))
        .collect();
    let base = PathBuf::from(&base_dir);
    let wanted_for_task = wanted.clone();
    let local =
        tokio::task::spawn_blocking(move || find_local_mods(&base, &instances, &wanted_for_task))
            .await
            .map_err(|e| CmdError::launcher(e.to_string()))?;

    let mut found = Vec::new();
    let mut missing: Vec<String> = wanted.clone();
    for (jar, provides) in local {
        let name = jar
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let path = slim_pack(jar, "mod").await?;
        missing.retain(|ns| !provides.contains(ns));
        found.push(ModBlockAssets {
            namespaces: provides,
            name,
            path,
            source: "instance",
        });
    }

    if download {
        let mut still = Vec::new();
        for ns in missing {
            match modrinth_mod(&ns, mc_version.as_deref()).await {
                Ok(Some(assets)) => found.push(assets),
                _ => still.push(ns),
            }
        }
        missing = still;
    }
    Ok(ModBlockAssetsResult { found, missing })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_jar(path: &Path, files: &[&str]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for name in files {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"{}").unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn finds_mods_by_block_namespace_in_instance_order() {
        let base = std::env::temp_dir().join(format!("nolauncher-mods-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let mods = |instance: &str| base.join("instance").join(instance).join(".minecraft/mods");
        write_jar(
            &mods("a").join("create-old.jar"),
            &["assets/create/blockstates/belt.json"],
        );
        write_jar(
            &mods("b").join("create.jar"),
            &["assets/create/blockstates/belt.json"],
        );
        write_jar(
            &mods("b").join("jei.jar"),
            &["assets/jei/textures/gui/x.png"],
        );
        write_jar(
            &mods("b").join("other.jar.disabled"),
            &["assets/farm/blockstates/x.json"],
        );

        let wanted = vec!["create".to_string(), "farm".to_string(), "jei".to_string()];
        let found = find_local_mods(&base, &["b".into(), "a".into()], &wanted);
        assert_eq!(
            found,
            vec![(mods("b").join("create.jar"), vec!["create".to_string()])]
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    #[ignore = "連到 Modrinth，並寫進 block-assets 快取"]
    async fn downloads_create_from_modrinth() {
        let assets = modrinth_mod("create", Some("1.21.1"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(assets.namespaces, ["create"]);
        assert!(assets.name.starts_with("create "), "{}", assets.name);
        assert!(Path::new(&assets.path).is_file());
        assert!(modrinth_mod("no-such-mod-namespace-xyz", None)
            .await
            .unwrap()
            .is_none());
        for ns in ["ars_nouveau", "sfm", "extendedae"] {
            let found = modrinth_mod(ns, Some("1.21.1")).await.unwrap();
            let found = found.unwrap_or_else(|| panic!("{ns} not found"));
            assert_eq!(found.namespaces, [ns], "{}", found.name);
            println!("{ns} -> {}", found.name);
        }
    }

    #[test]
    fn namespaces_are_url_safe() {
        assert!(is_namespace("create"));
        assert!(is_namespace("create_new-age.x"));
        for bad in ["", "Create", "a/b", "a?b", "../x"] {
            assert!(!is_namespace(bad), "{bad}");
        }
    }

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
