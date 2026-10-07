use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use futures::stream::{self, StreamExt};
use hexomc_lib::{
    download_file, fetch_atlauncher_pack_config, fetch_ftb_version_manifest,
    get_atlauncher_pack_versions, install_atlauncher_pack_files, install_ftb_pack_files,
    CurseForgeClient, DownloadTask,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::modpack::{
    curseforge_key, install_local_pack, install_pack, ModpackInstallDto, ModpackPreview,
    PackFormat, PackTarget,
};
use crate::error::{CmdError, CmdResult};
use crate::store::launcher::LauncherConfig;

pub(super) const PAGE_SIZE: usize = 20;
const CACHE_TTL: Duration = Duration::from_secs(30 * 60);
const USER_AGENT: &str = concat!("Nolauncher/", env!("CARGO_PKG_VERSION"));

pub(super) const MODRINTH_API: &str = "https://api.modrinth.com/v2";
const CURSEFORGE_API: &str = "https://api.curseforge.com/v1";
const ATL_API: &str = "https://api.atlauncher.com/v1";
const FTB_API: &str = "https://api.feed-the-beast.com/v1/modpacks/public";

pub(super) const CF_GAME_MINECRAFT: &str = "432";
const CF_CLASS_MODPACKS: &str = "4471";

pub(super) const CF_MAX_RESULTS: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Modrinth,
    CurseForge,
    AtLauncher,
    Ftb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortBy {
    #[default]
    Relevance,
    Downloads,
    Updated,
    Name,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub platform: Platform,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub sort: SortBy,
    #[serde(default)]
    pub mc_version: Option<String>,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackHit {
    pub id: String,
    pub name: String,
    pub author: String,
    pub summary: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub website: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    pub hits: Vec<PackHit>,
    pub total: usize,
    /// None = 沒有下一頁
    pub next_offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteVersion {
    pub id: String,
    pub name: String,
    pub mc_version: String,
    /// release / beta / alpha / archived
    pub kind: String,
    /// unix 秒；平台沒提供就是 0
    pub published: i64,
}

#[tauri::command]
pub async fn search_modpacks(request: SearchRequest) -> CmdResult<SearchPage> {
    let query = request.query.trim();
    let mc = request
        .mc_version
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());

    match request.platform {
        Platform::Modrinth => modrinth_search(query, request.sort, mc, request.offset).await,
        Platform::CurseForge => curseforge_search(query, request.sort, mc, request.offset).await,
        Platform::AtLauncher => Ok(local_search(
            &atl_catalog().await?,
            query,
            request.sort,
            mc,
            request.offset,
        )),
        Platform::Ftb => Ok(local_search(
            &ftb_catalog().await?.entries,
            query,
            request.sort,
            mc,
            request.offset,
        )),
    }
}

#[tauri::command]
pub async fn list_modpack_versions(
    platform: Platform,
    pack_id: String,
) -> CmdResult<Vec<RemoteVersion>> {
    let mut versions = match platform {
        Platform::Modrinth => modrinth_versions(segment(&pack_id)?).await?,
        Platform::CurseForge => curseforge_versions(numeric_id(&pack_id)?).await?,
        Platform::AtLauncher => get_atlauncher_pack_versions(safe_name(&pack_id)?)
            .await?
            .into_iter()
            .map(|v| RemoteVersion {
                id: v.version.clone(),
                name: v.version,
                mc_version: v.minecraft,
                kind: "release".to_string(),
                published: v.published,
            })
            .collect(),
        Platform::Ftb => ftb_pack(numeric_id(&pack_id)?)
            .await?
            .versions
            .iter()
            .filter(|v| !v.private)
            .map(|v| RemoteVersion {
                id: v.id.to_string(),
                name: v.name.clone(),
                mc_version: v.target("minecraft").unwrap_or_default().to_string(),
                kind: v.kind.clone(),
                published: v.updated,
            })
            .collect(),
    };

    versions.sort_by(|a, b| b.published.cmp(&a.published));
    Ok(versions)
}

#[tauri::command]
pub async fn install_remote_modpack(
    app: tauri::AppHandle,
    target: PackTarget,
    platform: Platform,
    pack_id: String,
    version_id: String,
    include_optional: bool,
) -> CmdResult<ModpackInstallDto> {
    match platform {
        Platform::Modrinth => modrinth_install(app, target, segment(&version_id)?).await,
        Platform::CurseForge => {
            curseforge_install(app, target, numeric_id(&pack_id)?, numeric_id(&version_id)?).await
        }
        Platform::AtLauncher => {
            atl_install(
                app,
                target,
                safe_name(&pack_id)?,
                segment(&version_id)?,
                include_optional,
            )
            .await
        }
        Platform::Ftb => {
            ftb_install(
                app,
                target,
                numeric_id(&pack_id)?,
                numeric_id(&version_id)?,
                include_optional,
            )
            .await
        }
    }
}

pub(super) fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .unwrap_or_default()
    })
}

pub(super) async fn get_json<T: DeserializeOwned>(
    request: reqwest::RequestBuilder,
) -> CmdResult<T> {
    Ok(request.send().await?.error_for_status()?.json().await?)
}

pub(super) fn nullable<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

async fn install_downloaded(
    app: tauri::AppHandle,
    target: PackTarget,
    url: &str,
    sha1: Option<&str>,
    extension: &str,
) -> CmdResult<ModpackInstallDto> {
    let dir = PathBuf::from(&target.base_dir)
        .join("temp")
        .join("modpacks");
    tokio::fs::create_dir_all(&dir).await?;
    let path = dir.join(format!("{}.{}", uuid::Uuid::new_v4(), extension));

    let mut task = DownloadTask::new(url, &path);
    if let Some(sha1) = sha1 {
        task = task.with_sha1(sha1);
    }

    let result = match download_file(&task).await {
        Ok(()) => install_local_pack(app, target, path.clone()).await,
        Err(e) => Err(e.into()),
    };
    let _ = tokio::fs::remove_file(&path).await;
    result
}

pub(super) fn segment(value: &str) -> CmdResult<&str> {
    let value = value.trim();
    let ok = !value.is_empty()
        && !value.contains("..")
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ' '));
    if ok {
        Ok(value)
    } else {
        Err(CmdError::launcher(format!("ID 不合法: {:?}", value)))
    }
}

pub(super) fn numeric_id(value: &str) -> CmdResult<u64> {
    value
        .trim()
        .parse()
        .map_err(|_| CmdError::launcher(format!("ID 不合法: {:?}", value)))
}

fn safe_name(value: &str) -> CmdResult<&str> {
    let value = value.trim();
    if value.is_empty() || !value.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(CmdError::launcher(format!(
            "ATLauncher 模組包名稱不合法: {:?}",
            value
        )));
    }
    Ok(value)
}

pub(super) fn iso_to_unix(text: &str) -> i64 {
    let part = |range: std::ops::Range<usize>| text.get(range).and_then(|s| s.parse::<i64>().ok());
    let (Some(year), Some(month), Some(day)) = (part(0..4), part(5..7), part(8..10)) else {
        return 0;
    };
    let seconds = part(11..13).unwrap_or(0) * 3600
        + part(14..16).unwrap_or(0) * 60
        + part(17..19).unwrap_or(0);

    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    (era * 146_097 + day_of_era - 719_468) * 86_400 + seconds
}

#[derive(Debug, Clone)]
struct CatalogEntry {
    hit: PackHit,
    mc_versions: Vec<String>,

    popularity: i64,
    updated: i64,

    keywords: String,
}

fn local_search(
    entries: &[CatalogEntry],
    query: &str,
    sort: SortBy,
    mc: Option<&str>,
    offset: usize,
) -> SearchPage {
    let query = query.to_lowercase();
    let words: Vec<&str> = query.split_whitespace().collect();

    let mut matched: Vec<(&CatalogEntry, u8)> = entries
        .iter()
        .filter(|e| mc.map_or(true, |mc| e.mc_versions.iter().any(|v| v == mc)))
        .filter(|e| words.iter().all(|w| e.keywords.contains(w)))
        .map(|e| {
            let name = e.hit.name.to_lowercase();
            let score = if words.is_empty() {
                0
            } else if name == query {
                2
            } else if words.iter().all(|w| name.contains(w)) {
                1
            } else {
                0
            };
            (e, score)
        })
        .collect();

    match sort {
        SortBy::Relevance => {
            matched.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.popularity.cmp(&a.0.popularity)))
        }
        SortBy::Downloads => matched.sort_by(|a, b| b.0.popularity.cmp(&a.0.popularity)),
        SortBy::Updated => matched.sort_by(|a, b| b.0.updated.cmp(&a.0.updated)),
        SortBy::Name => matched.sort_by(|a, b| {
            a.0.hit
                .name
                .to_lowercase()
                .cmp(&b.0.hit.name.to_lowercase())
        }),
    }

    let total = matched.len();
    SearchPage {
        hits: matched
            .iter()
            .skip(offset)
            .take(PAGE_SIZE)
            .map(|(e, _)| e.hit.clone())
            .collect(),
        total,
        next_offset: Some(offset + PAGE_SIZE).filter(|next| *next < total),
    }
}

struct Cached<T> {
    at: Instant,
    value: Arc<T>,
}

type CacheSlot<T> = Mutex<Option<Cached<T>>>;

fn cache_get<T>(slot: &CacheSlot<T>) -> Option<Arc<T>> {
    let guard = slot.lock().ok()?;
    guard
        .as_ref()
        .filter(|c| c.at.elapsed() < CACHE_TTL)
        .map(|c| c.value.clone())
}

fn cache_put<T>(slot: &CacheSlot<T>, value: T) -> Arc<T> {
    let value = Arc::new(value);
    if let Ok(mut guard) = slot.lock() {
        *guard = Some(Cached {
            at: Instant::now(),
            value: value.clone(),
        });
    }
    value
}

fn dedup_versions(versions: impl Iterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for v in versions {
        if !v.is_empty() && !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

#[derive(Debug, Deserialize)]
struct MrVersion {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    version_number: String,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    version_type: String,
    #[serde(default)]
    date_published: String,
    #[serde(default)]
    files: Vec<MrFile>,
}

#[derive(Debug, Deserialize)]
struct MrFile {
    url: String,
    filename: String,
    #[serde(default)]
    primary: bool,
    #[serde(default)]
    hashes: HashMap<String, String>,
}

async fn modrinth_search(
    query: &str,
    sort: SortBy,
    mc: Option<&str>,
    offset: usize,
) -> CmdResult<SearchPage> {
    #[derive(Deserialize)]
    struct Resp {
        hits: Vec<Hit>,
        total_hits: usize,
    }
    #[derive(Deserialize)]
    struct Hit {
        project_id: String,
        slug: String,
        title: String,
        #[serde(default, deserialize_with = "nullable")]
        author: String,
        #[serde(default, deserialize_with = "nullable")]
        description: String,
        #[serde(default)]
        icon_url: Option<String>,
        #[serde(default)]
        downloads: u64,
    }

    let mut facets = vec![vec!["project_type:modpack".to_string()]];
    if let Some(mc) = mc {
        facets.push(vec![format!("versions:{}", mc)]);
    }
    let facets = serde_json::to_string(&facets)?;

    let index = match sort {
        SortBy::Relevance | SortBy::Name => "relevance",
        SortBy::Downloads => "downloads",
        SortBy::Updated => "updated",
    };
    let offset_text = offset.to_string();
    let limit_text = PAGE_SIZE.to_string();

    let resp: Resp = get_json(http().get(format!("{}/search", MODRINTH_API)).query(&[
        ("query", query),
        ("index", index),
        ("facets", &facets),
        ("offset", &offset_text),
        ("limit", &limit_text),
    ]))
    .await?;

    let next = offset + resp.hits.len();
    Ok(SearchPage {
        next_offset: Some(next).filter(|n| *n < resp.total_hits && !resp.hits.is_empty()),
        total: resp.total_hits,
        hits: resp
            .hits
            .into_iter()
            .map(|h| PackHit {
                website: Some(format!("https://modrinth.com/modpack/{}", h.slug)),
                id: h.project_id,
                name: h.title,
                author: h.author,
                summary: h.description,
                icon_url: h.icon_url.filter(|u| !u.is_empty()),
                downloads: h.downloads,
            })
            .collect(),
    })
}

async fn modrinth_versions(project_id: &str) -> CmdResult<Vec<RemoteVersion>> {
    let versions: Vec<MrVersion> =
        get_json(http().get(format!("{}/project/{}/version", MODRINTH_API, project_id))).await?;

    Ok(versions
        .into_iter()
        .map(|v| RemoteVersion {
            name: if v.name.trim().is_empty() {
                v.version_number
            } else {
                v.name
            },
            mc_version: v.game_versions.last().cloned().unwrap_or_default(),
            kind: v.version_type,
            published: iso_to_unix(&v.date_published),
            id: v.id,
        })
        .collect())
}

async fn modrinth_install(
    app: tauri::AppHandle,
    target: PackTarget,
    version_id: &str,
) -> CmdResult<ModpackInstallDto> {
    let version: MrVersion =
        get_json(http().get(format!("{}/version/{}", MODRINTH_API, version_id))).await?;

    let is_pack = |f: &&MrFile| f.filename.to_ascii_lowercase().ends_with(".mrpack");
    let file = version
        .files
        .iter()
        .filter(is_pack)
        .find(|f| f.primary)
        .or_else(|| version.files.iter().find(is_pack))
        .ok_or_else(|| CmdError::launcher("這個版本沒有 .mrpack 檔案"))?;

    install_downloaded(
        app,
        target,
        &file.url,
        file.hashes.get("sha1").map(String::as_str),
        "mrpack",
    )
    .await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFile {
    id: u64,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    game_versions: Vec<String>,

    #[serde(default)]
    release_type: u32,
    #[serde(default)]
    file_date: String,
    #[serde(default)]
    is_server_pack: bool,
    #[serde(default)]
    hashes: Vec<CfHash>,
}

#[derive(Debug, Deserialize)]
struct CfHash {
    value: String,
    algo: u32,
}

pub(super) async fn curseforge_get(path: &str) -> CmdResult<reqwest::RequestBuilder> {
    Ok(http()
        .get(format!("{}{}", CURSEFORGE_API, path))
        .header("x-api-key", curseforge_key().await?))
}

async fn curseforge_search(
    query: &str,
    sort: SortBy,
    mc: Option<&str>,
    offset: usize,
) -> CmdResult<SearchPage> {
    #[derive(Deserialize)]
    struct Resp {
        data: Vec<CfMod>,
        pagination: Pagination,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Pagination {
        #[serde(default)]
        total_count: usize,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CfMod {
        id: u64,
        name: String,
        #[serde(default, deserialize_with = "nullable")]
        summary: String,
        #[serde(default)]
        download_count: u64,
        #[serde(default)]
        logo: Option<Logo>,
        #[serde(default)]
        authors: Vec<Author>,
        #[serde(default)]
        links: Option<Links>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Logo {
        #[serde(default)]
        thumbnail_url: Option<String>,
    }
    #[derive(Deserialize)]
    struct Author {
        name: String,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Links {
        #[serde(default)]
        website_url: Option<String>,
    }

    if offset + PAGE_SIZE > CF_MAX_RESULTS {
        return Ok(SearchPage {
            hits: Vec::new(),
            total: CF_MAX_RESULTS,
            next_offset: None,
        });
    }

    let (sort_field, sort_order) = match sort {
        SortBy::Relevance => ("2", "desc"),
        SortBy::Downloads => ("6", "desc"),
        SortBy::Updated => ("3", "desc"),
        SortBy::Name => ("4", "asc"),
    };
    let offset_text = offset.to_string();
    let limit_text = PAGE_SIZE.to_string();
    let mut params = vec![
        ("gameId", CF_GAME_MINECRAFT),
        ("classId", CF_CLASS_MODPACKS),
        ("sortField", sort_field),
        ("sortOrder", sort_order),
        ("index", offset_text.as_str()),
        ("pageSize", limit_text.as_str()),
    ];
    if !query.is_empty() {
        params.push(("searchFilter", query));
    }
    if let Some(mc) = mc {
        params.push(("gameVersion", mc));
    }

    let resp: Resp = get_json(curseforge_get("/mods/search").await?.query(&params)).await?;

    let total = resp.pagination.total_count.min(CF_MAX_RESULTS);
    let next = offset + resp.data.len();
    Ok(SearchPage {
        next_offset: Some(next)
            .filter(|n| *n < total && n + PAGE_SIZE <= CF_MAX_RESULTS && !resp.data.is_empty()),
        total,
        hits: resp
            .data
            .into_iter()
            .map(|m| PackHit {
                id: m.id.to_string(),
                name: m.name,
                author: m
                    .authors
                    .into_iter()
                    .map(|a| a.name)
                    .collect::<Vec<_>>()
                    .join(", "),
                summary: m.summary,
                icon_url: m
                    .logo
                    .and_then(|l| l.thumbnail_url)
                    .filter(|u| !u.is_empty()),
                downloads: m.download_count,
                website: m.links.and_then(|l| l.website_url),
            })
            .collect(),
    })
}

async fn curseforge_versions(mod_id: u64) -> CmdResult<Vec<RemoteVersion>> {
    #[derive(Deserialize)]
    struct Resp {
        data: Vec<CfFile>,
    }

    let resp: Resp = get_json(
        curseforge_get(&format!("/mods/{}/files", mod_id))
            .await?
            .query(&[("pageSize", "50")]),
    )
    .await?;

    Ok(resp
        .data
        .into_iter()
        .filter(|f| !f.is_server_pack)
        .map(|f| RemoteVersion {
            id: f.id.to_string(),
            mc_version: f
                .game_versions
                .iter()
                .find(|v| v.starts_with(|c: char| c.is_ascii_digit()))
                .cloned()
                .unwrap_or_default(),
            kind: match f.release_type {
                2 => "beta",
                3 => "alpha",
                _ => "release",
            }
            .to_string(),
            published: iso_to_unix(&f.file_date),
            name: f.display_name,
        })
        .collect())
}

async fn curseforge_install(
    app: tauri::AppHandle,
    target: PackTarget,
    mod_id: u64,
    file_id: u64,
) -> CmdResult<ModpackInstallDto> {
    #[derive(Deserialize)]
    struct Resp {
        data: CfFile,
    }

    let resp: Resp =
        get_json(curseforge_get(&format!("/mods/{}/files/{}", mod_id, file_id)).await?).await?;
    let file = resp.data;
    let Some(url) = file.download_url.as_deref() else {
        return Err(CmdError::new(
            "downloadBlocked",
            "這個模組包的作者不允許第三方下載，請到 CurseForge 網站下載，再用「匯入模組包」安裝",
        ));
    };
    let sha1 = file
        .hashes
        .iter()
        .find(|h| h.algo == 1)
        .map(|h| h.value.as_str());

    install_downloaded(app, target, url, sha1, "zip").await
}

static ATL_CACHE: CacheSlot<Vec<CatalogEntry>> = Mutex::new(None);

#[derive(Debug, Deserialize)]
struct AtlPackList {
    #[serde(default, deserialize_with = "nullable")]
    data: Vec<AtlPack>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AtlPack {
    name: String,
    #[serde(default, deserialize_with = "nullable")]
    safe_name: String,
    #[serde(rename = "type", default, deserialize_with = "nullable")]
    pack_type: String,
    #[serde(default, deserialize_with = "nullable")]
    versions: Vec<AtlVersion>,
    #[serde(default, deserialize_with = "nullable")]
    description: String,
    #[serde(rename = "websiteURL", default)]
    website_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AtlVersion {
    #[serde(default, deserialize_with = "nullable")]
    minecraft: String,
    #[serde(default, deserialize_with = "nullable")]
    published: i64,
}

async fn atl_catalog() -> CmdResult<Arc<Vec<CatalogEntry>>> {
    if let Some(cached) = cache_get(&ATL_CACHE) {
        return Ok(cached);
    }

    let list: AtlPackList = get_json(http().get(format!("{}/packs/full/public", ATL_API))).await?;

    let entries = list
        .data
        .into_iter()
        .filter(|p| p.pack_type == "public" && !p.versions.is_empty() && !p.safe_name.is_empty())
        .map(|p| {
            let updated = p.versions.iter().map(|v| v.published).max().unwrap_or(0);
            CatalogEntry {
                keywords: format!("{} {}", p.name, p.description).to_lowercase(),
                mc_versions: dedup_versions(p.versions.into_iter().map(|v| v.minecraft)),
                // 公開 API 沒有下載數，也沒有圖示網址
                popularity: 0,
                updated,
                hit: PackHit {
                    icon_url: None,
                    id: p.safe_name,
                    name: p.name,
                    author: String::new(),
                    summary: p.description,
                    downloads: 0,
                    website: p.website_url.filter(|u| !u.is_empty()),
                },
            }
        })
        .collect();

    Ok(cache_put(&ATL_CACHE, entries))
}

async fn atl_install(
    app: tauri::AppHandle,
    target: PackTarget,
    safe_name: &str,
    version: &str,
    include_optional: bool,
) -> CmdResult<ModpackInstallDto> {
    let config = fetch_atlauncher_pack_config(safe_name, version).await?;
    let count = config
        .mods
        .iter()
        .filter(|m| m.client && (!m.optional || include_optional))
        .count();
    let preview = ModpackPreview::new(PackFormat::AtLauncher, config.info(safe_name)?, count);

    let safe_name = safe_name.to_string();
    let version = version.to_string();
    install_pack(app, target, preview, move |id, base, progress| async move {
        install_atlauncher_pack_files(&safe_name, &version, &id, &base, include_optional, progress)
            .await
    })
    .await
}

#[derive(Debug, Clone, Deserialize)]
struct FtbPack {
    id: u64,
    name: String,
    #[serde(default, deserialize_with = "nullable")]
    synopsis: String,
    #[serde(default)]
    art: Vec<FtbArt>,
    #[serde(default)]
    authors: Vec<FtbAuthor>,
    #[serde(default)]
    versions: Vec<FtbVersion>,
    #[serde(default)]
    tags: Vec<FtbTag>,
    #[serde(default)]
    installs: u64,
    #[serde(default)]
    updated: i64,
    #[serde(default)]
    private: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct FtbArt {
    url: String,
    #[serde(rename = "type", default)]
    kind: String,
}

#[derive(Debug, Clone, Deserialize)]
struct FtbAuthor {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct FtbTag {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct FtbVersion {
    id: u64,
    name: String,
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    updated: i64,
    #[serde(default)]
    private: bool,
    #[serde(default)]
    targets: Vec<FtbTarget>,
}

#[derive(Debug, Clone, Deserialize)]
struct FtbTarget {
    name: String,
    version: String,
}

impl FtbVersion {
    fn target(&self, name: &str) -> Option<&str> {
        self.targets
            .iter()
            .find(|t| t.name == name)
            .map(|t| t.version.as_str())
    }
}

struct FtbCatalog {
    entries: Vec<CatalogEntry>,
    packs: HashMap<u64, FtbPack>,
}

static FTB_CACHE: CacheSlot<FtbCatalog> = Mutex::new(None);

async fn ftb_catalog() -> CmdResult<Arc<FtbCatalog>> {
    #[derive(Deserialize)]
    struct AllPacks {
        #[serde(default)]
        packs: Vec<u64>,
    }

    if let Some(cached) = cache_get(&FTB_CACHE) {
        return Ok(cached);
    }

    let all: AllPacks = get_json(http().get(format!("{}/modpack/all", FTB_API))).await?;
    let packs: Vec<FtbPack> = stream::iter(all.packs)
        .map(|id| async move {
            get_json::<FtbPack>(http().get(format!("{}/modpack/{}", FTB_API, id)))
                .await
                .ok()
        })
        .buffer_unordered(8)
        .filter_map(|pack| async move { pack })
        .filter(|pack| {
            let visible = !pack.private && pack.versions.iter().any(|v| !v.private);
            async move { visible }
        })
        .collect()
        .await;

    let entries = packs
        .iter()
        .map(|p| CatalogEntry {
            hit: PackHit {
                id: p.id.to_string(),
                name: p.name.clone(),
                author: p
                    .authors
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                summary: p.synopsis.clone(),
                icon_url: p
                    .art
                    .iter()
                    .find(|a| a.kind == "square")
                    .or_else(|| p.art.first())
                    .map(|a| a.url.clone()),
                downloads: p.installs,
                website: None,
            },
            mc_versions: dedup_versions(
                p.versions
                    .iter()
                    .filter_map(|v| v.target("minecraft").map(str::to_string)),
            ),
            popularity: p.installs as i64,
            updated: p.updated,
            keywords: std::iter::once(p.name.as_str())
                .chain(std::iter::once(p.synopsis.as_str()))
                .chain(p.tags.iter().map(|t| t.name.as_str()))
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase(),
        })
        .collect();

    Ok(cache_put(
        &FTB_CACHE,
        FtbCatalog {
            entries,
            packs: packs.into_iter().map(|p| (p.id, p)).collect(),
        },
    ))
}

async fn ftb_pack(pack_id: u64) -> CmdResult<FtbPack> {
    if let Some(pack) = cache_get(&FTB_CACHE).and_then(|c| c.packs.get(&pack_id).cloned()) {
        return Ok(pack);
    }
    get_json(http().get(format!("{}/modpack/{}", FTB_API, pack_id))).await
}

async fn ftb_install(
    app: tauri::AppHandle,
    target: PackTarget,
    pack_id: u64,
    version_id: u64,
    include_optional: bool,
) -> CmdResult<ModpackInstallDto> {
    let pack = ftb_pack(pack_id).await?;

    let manifest = fetch_ftb_version_manifest(pack_id, version_id).await?;
    let count = manifest
        .files
        .iter()
        .filter(|f| !f.serveronly && (!f.optional || include_optional))
        .count();
    let preview = ModpackPreview::new(PackFormat::Ftb, manifest.info(&pack.name)?, count);

    let key = LauncherConfig::load().await?.api.curseforge_api_key;
    let curseforge = Some(key.trim())
        .filter(|k| !k.is_empty())
        .map(CurseForgeClient::new);

    install_pack(app, target, preview, move |id, base, progress| async move {
        install_ftb_pack_files(
            pack_id,
            version_id,
            &id,
            &base,
            curseforge.as_ref(),
            include_optional,
            progress,
        )
        .await
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_dates() {
        assert_eq!(iso_to_unix("1970-01-01T00:00:00Z"), 0);
        assert_eq!(iso_to_unix("2024-02-29T12:00:00.123Z"), 1_709_208_000);
        assert_eq!(iso_to_unix("garbage"), 0);
    }

    #[test]
    fn atl_pack_accepts_null_fields() {
        let json = r#"{
            "error": false,
            "data": [{
                "id": 491,
                "name": "Sky Factory 4",
                "safeName": "SkyFactory4",
                "type": "public",
                "versions": [{ "version": "4.2.4", "minecraft": "1.12.2", "published": 1639485294 }],
                "description": null,
                "websiteURL": null
            }]
        }"#;
        let list: AtlPackList = serde_json::from_str(json).unwrap();
        let pack = &list.data[0];
        assert_eq!(pack.safe_name, "SkyFactory4");
        assert_eq!(pack.description, "");
        assert_eq!(pack.versions[0].minecraft, "1.12.2");
        assert_eq!(pack.versions[0].published, 1639485294);
        assert!(pack.website_url.is_none());
    }

    fn entry(name: &str, popularity: i64, mc: &str) -> CatalogEntry {
        CatalogEntry {
            hit: PackHit {
                id: name.to_string(),
                name: name.to_string(),
                author: String::new(),
                summary: String::new(),
                icon_url: None,
                downloads: 0,
                website: None,
            },
            mc_versions: vec![mc.to_string()],
            popularity,
            updated: 0,
            keywords: name.to_lowercase(),
        }
    }

    #[test]
    fn local_search_ranks_name_matches() {
        let entries = vec![
            entry("Sky Factory", 1, "1.12.2"),
            entry("Skies", 5, "1.21.1"),
            entry("Tech World", 9, "1.12.2"),
        ];

        let page = local_search(&entries, "sky factory", SortBy::Relevance, None, 0);
        assert_eq!(page.total, 1);
        assert_eq!(page.hits[0].name, "Sky Factory");

        let page = local_search(&entries, "", SortBy::Downloads, Some("1.12.2"), 0);
        let names: Vec<_> = page.hits.iter().map(|h| h.name.as_str()).collect();
        assert_eq!(names, ["Tech World", "Sky Factory"]);
        assert_eq!(page.next_offset, None);
    }
}
