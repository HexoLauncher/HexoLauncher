use std::collections::HashMap;
use std::path::Path;

use hexomc_lib::{download_file, DownloadTask};
use serde::{Deserialize, Serialize};

use super::modpack_browse::{
    curseforge_get, get_json, http, iso_to_unix, nullable, numeric_id, segment, PackHit, Platform,
    SearchPage, SortBy, CF_GAME_MINECRAFT, CF_MAX_RESULTS, MODRINTH_API, PAGE_SIZE,
};
use super::{minecraft_dir, path_string, LoaderKind};
use crate::error::{CmdError, CmdResult};

pub mod install;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentKind {
    Mod,
    ResourcePack,
    Shader,
}

impl ContentKind {
    fn folder(self) -> &'static str {
        match self {
            ContentKind::Mod => "mods",
            ContentKind::ResourcePack => "resourcepacks",
            ContentKind::Shader => "shaderpacks",
        }
    }

    fn modrinth_type(self) -> &'static str {
        match self {
            ContentKind::Mod => "mod",
            ContentKind::ResourcePack => "resourcepack",
            ContentKind::Shader => "shader",
        }
    }

    fn curseforge_class(self) -> &'static str {
        match self {
            ContentKind::Mod => "6",
            ContentKind::ResourcePack => "12",
            ContentKind::Shader => "6552",
        }
    }

    fn uses_loader(self) -> bool {
        matches!(self, ContentKind::Mod)
    }

    fn extension(self) -> &'static str {
        match self {
            ContentKind::Mod => "jar",
            _ => "zip",
        }
    }
}

fn loader_name(loader: LoaderKind) -> Option<&'static str> {
    match loader {
        LoaderKind::Vanilla => None,
        LoaderKind::Fabric => Some("fabric"),
        LoaderKind::Forge => Some("forge"),
        LoaderKind::NeoForge => Some("neoforge"),
    }
}

fn cf_loader_type(loader: LoaderKind) -> Option<&'static str> {
    match loader {
        LoaderKind::Vanilla => None,
        LoaderKind::Forge => Some("1"),
        LoaderKind::Fabric => Some("4"),
        LoaderKind::NeoForge => Some("6"),
    }
}

fn unsupported() -> CmdError {
    CmdError::new(
        "unsupportedPlatform",
        "ATLauncher 與 FTB 只提供模組包，沒辦法單獨下載模組或材質",
    )
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchRequest {
    pub platform: Platform,
    pub kind: ContentKind,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub sort: SortBy,
    #[serde(default)]
    pub mc_version: Option<String>,
    #[serde(default)]
    pub loader: Option<LoaderKind>,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentVersion {
    pub id: String,
    pub name: String,
    pub mc_version: String,
    pub kind: String,
    pub published: i64,
    pub file_name: String,
    pub loaders: Vec<String>,
    pub downloadable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledContent {
    pub file_name: String,
    pub path: String,
}

#[tauri::command]
pub async fn search_content(request: ContentSearchRequest) -> CmdResult<SearchPage> {
    let query = request.query.trim();
    let mc = request
        .mc_version
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let loader = request.loader.filter(|_| request.kind.uses_loader());

    match request.platform {
        Platform::Modrinth => {
            modrinth_search(
                request.kind,
                query,
                request.sort,
                mc,
                loader,
                request.offset,
            )
            .await
        }
        Platform::CurseForge => {
            curseforge_search(
                request.kind,
                query,
                request.sort,
                mc,
                loader,
                request.offset,
            )
            .await
        }
        _ => Err(unsupported()),
    }
}

#[tauri::command]
pub async fn list_content_versions(
    platform: Platform,
    kind: ContentKind,
    project_id: String,
    mc_version: Option<String>,
    loader: Option<LoaderKind>,
) -> CmdResult<Vec<ContentVersion>> {
    let mc = mc_version
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let loader = loader.filter(|_| kind.uses_loader());

    let mut versions = match platform {
        Platform::Modrinth => modrinth_versions(segment(&project_id)?, mc, loader).await?,
        Platform::CurseForge => curseforge_versions(numeric_id(&project_id)?, mc, loader).await?,
        _ => return Err(unsupported()),
    };

    versions.sort_by(|a, b| b.published.cmp(&a.published));
    Ok(versions)
}

#[tauri::command]
pub async fn install_content(
    base_dir: String,
    instance_name: String,
    platform: Platform,
    kind: ContentKind,
    project_id: String,
    version_id: String,
) -> CmdResult<InstalledContent> {
    if kind == ContentKind::Mod {
        return Err(CmdError::new("installPlanRequired", "Prepare a mod installation plan first."));
    }
    let file = match platform {
        Platform::Modrinth => modrinth_file(segment(&version_id)?).await?,
        Platform::CurseForge => {
            curseforge_file(numeric_id(&project_id)?, numeric_id(&version_id)?).await?
        }
        _ => return Err(unsupported()),
    };

    let dir = minecraft_dir(Path::new(&base_dir), &instance_name)?.join(kind.folder());
    tokio::fs::create_dir_all(&dir).await?;

    let file_name = safe_file_name(&file.name, kind);
    let path = dir.join(&file_name);

    let mut task = DownloadTask::new(&file.url, &path);
    if let Some(sha1) = file.sha1.as_deref() {
        task = task.with_sha1(sha1);
    }
    download_file(&task).await?;

    Ok(InstalledContent {
        path: path_string(&path),
        file_name,
    })
}

struct RemoteFile {
    url: String,
    name: String,
    sha1: Option<String>,
}

fn safe_file_name(raw: &str, kind: ContentKind) -> String {
    let name = raw
        .rsplit(['/', '\\', ':'])
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches('.');

    if name.is_empty() {
        format!("download.{}", kind.extension())
    } else {
        name.to_string()
    }
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
    loaders: Vec<String>,
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

impl MrVersion {
    fn primary_file(&self) -> Option<&MrFile> {
        self.files
            .iter()
            .find(|f| f.primary)
            .or_else(|| self.files.first())
    }
}

async fn modrinth_search(
    kind: ContentKind,
    query: &str,
    sort: SortBy,
    mc: Option<&str>,
    loader: Option<LoaderKind>,
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

    let mut facets = vec![vec![format!("project_type:{}", kind.modrinth_type())]];
    if let Some(mc) = mc {
        facets.push(vec![format!("versions:{}", mc)]);
    }
    if let Some(name) = loader.and_then(loader_name) {
        facets.push(vec![format!("categories:{}", name)]);
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

    let kind_path = kind.modrinth_type();
    let next = offset + resp.hits.len();
    Ok(SearchPage {
        next_offset: Some(next).filter(|n| *n < resp.total_hits && !resp.hits.is_empty()),
        total: resp.total_hits,
        hits: resp
            .hits
            .into_iter()
            .map(|h| PackHit {
                website: Some(format!("https://modrinth.com/{}/{}", kind_path, h.slug)),
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

async fn modrinth_versions(
    project_id: &str,
    mc: Option<&str>,
    loader: Option<LoaderKind>,
) -> CmdResult<Vec<ContentVersion>> {
    let mut request = http().get(format!("{}/project/{}/version", MODRINTH_API, project_id));
    if let Some(mc) = mc {
        request = request.query(&[("game_versions", serde_json::to_string(&[mc])?)]);
    }
    if let Some(name) = loader.and_then(loader_name) {
        request = request.query(&[("loaders", serde_json::to_string(&[name])?)]);
    }

    let versions: Vec<MrVersion> = get_json(request).await?;

    Ok(versions
        .into_iter()
        .map(|v| ContentVersion {
            file_name: v
                .primary_file()
                .map(|f| f.filename.clone())
                .unwrap_or_default(),
            downloadable: v.primary_file().is_some(),
            name: if v.name.trim().is_empty() {
                v.version_number
            } else {
                v.name
            },
            mc_version: v.game_versions.last().cloned().unwrap_or_default(),
            kind: v.version_type,
            published: iso_to_unix(&v.date_published),
            loaders: v.loaders,
            id: v.id,
        })
        .collect())
}

async fn modrinth_file(version_id: &str) -> CmdResult<RemoteFile> {
    let version: MrVersion =
        get_json(http().get(format!("{}/version/{}", MODRINTH_API, version_id))).await?;

    let file = version
        .primary_file()
        .ok_or_else(|| CmdError::launcher("這個版本沒有可以下載的檔案"))?;

    Ok(RemoteFile {
        url: file.url.clone(),
        name: file.filename.clone(),
        sha1: file.hashes.get("sha1").cloned(),
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFile {
    id: u64,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    file_name: String,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    release_type: u32,
    #[serde(default)]
    file_date: String,
    #[serde(default)]
    hashes: Vec<CfHash>,
}

#[derive(Debug, Deserialize)]
struct CfHash {
    value: String,
    algo: u32,
}

impl CfFile {
    fn mc_version(&self) -> String {
        self.game_versions
            .iter()
            .find(|v| v.starts_with(|c: char| c.is_ascii_digit()))
            .cloned()
            .unwrap_or_default()
    }

    fn loaders(&self) -> Vec<String> {
        self.game_versions
            .iter()
            .filter(|v| !v.starts_with(|c: char| c.is_ascii_digit()))
            .map(|v| v.to_lowercase())
            .collect()
    }

    fn sha1(&self) -> Option<String> {
        self.hashes
            .iter()
            .find(|h| h.algo == 1)
            .map(|h| h.value.clone())
    }
}

async fn curseforge_search(
    kind: ContentKind,
    query: &str,
    sort: SortBy,
    mc: Option<&str>,
    loader: Option<LoaderKind>,
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
        ("classId", kind.curseforge_class()),
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
    if let Some(loader) = loader.and_then(cf_loader_type) {
        params.push(("modLoaderType", loader));
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

async fn curseforge_versions(
    mod_id: u64,
    mc: Option<&str>,
    loader: Option<LoaderKind>,
) -> CmdResult<Vec<ContentVersion>> {
    #[derive(Deserialize)]
    struct Resp {
        data: Vec<CfFile>,
    }

    let mut params = vec![("pageSize", "50")];
    if let Some(mc) = mc {
        params.push(("gameVersion", mc));
    }
    if let Some(loader) = loader.and_then(cf_loader_type) {
        params.push(("modLoaderType", loader));
    }

    let resp: Resp = get_json(
        curseforge_get(&format!("/mods/{}/files", mod_id))
            .await?
            .query(&params),
    )
    .await?;

    Ok(resp
        .data
        .into_iter()
        .map(|f| ContentVersion {
            id: f.id.to_string(),
            mc_version: f.mc_version(),
            loaders: f.loaders(),
            kind: match f.release_type {
                2 => "beta",
                3 => "alpha",
                _ => "release",
            }
            .to_string(),
            published: iso_to_unix(&f.file_date),
            downloadable: f.download_url.is_some(),
            file_name: f.file_name,
            name: f.display_name,
        })
        .collect())
}

async fn curseforge_file(mod_id: u64, file_id: u64) -> CmdResult<RemoteFile> {
    #[derive(Deserialize)]
    struct Resp {
        data: CfFile,
    }

    let resp: Resp =
        get_json(curseforge_get(&format!("/mods/{}/files/{}", mod_id, file_id)).await?).await?;
    let file = resp.data;

    let Some(url) = file.download_url.clone() else {
        return Err(CmdError::new(
            "downloadBlocked",
            "作者不允許第三方下載這個檔案，請到 CurseForge 網站下載，再放進實例資料夾",
        ));
    };

    Ok(RemoteFile {
        sha1: file.sha1(),
        name: file.file_name,
        url,
    })
}

const MAX_DESCRIPTION: usize = 40000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryImage {
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentLink {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DescriptionFormat {
    Markdown,
    Html,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentDetail {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub description: String,
    pub description_format: DescriptionFormat,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub follows: u64,
    pub categories: Vec<String>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub license: Option<String>,
    /// unix 秒；平台沒提供就是 0
    pub updated: i64,
    pub gallery: Vec<GalleryImage>,
    pub links: Vec<ContentLink>,
}

#[tauri::command]
pub async fn get_content_detail(
    platform: Platform,
    kind: ContentKind,
    project_id: String,
) -> CmdResult<ContentDetail> {
    match platform {
        Platform::Modrinth => modrinth_detail(kind, segment(&project_id)?).await,
        Platform::CurseForge => curseforge_detail(numeric_id(&project_id)?).await,
        _ => Err(unsupported()),
    }
}

fn push_link(links: &mut Vec<ContentLink>, label: &str, url: Option<String>) {
    if let Some(url) = url.filter(|u| !u.trim().is_empty()) {
        links.push(ContentLink {
            label: label.to_string(),
            url,
        });
    }
}

fn dedup(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for value in values {
        if !value.is_empty() && !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

async fn modrinth_detail(kind: ContentKind, project_id: &str) -> CmdResult<ContentDetail> {
    #[derive(Deserialize)]
    struct Project {
        id: String,
        #[serde(default)]
        slug: String,
        #[serde(default)]
        title: String,
        #[serde(default, deserialize_with = "nullable")]
        description: String,
        #[serde(default, deserialize_with = "nullable")]
        body: String,
        #[serde(default)]
        icon_url: Option<String>,
        #[serde(default)]
        downloads: u64,
        #[serde(default)]
        followers: u64,
        #[serde(default)]
        categories: Vec<String>,
        #[serde(default)]
        game_versions: Vec<String>,
        #[serde(default)]
        loaders: Vec<String>,
        #[serde(default)]
        license: Option<License>,
        #[serde(default, deserialize_with = "nullable")]
        updated: String,
        #[serde(default)]
        gallery: Vec<Gallery>,
        #[serde(default)]
        issues_url: Option<String>,
        #[serde(default)]
        source_url: Option<String>,
        #[serde(default)]
        wiki_url: Option<String>,
        #[serde(default)]
        discord_url: Option<String>,
    }
    #[derive(Deserialize)]
    struct License {
        #[serde(default)]
        id: String,
        #[serde(default)]
        name: Option<String>,
    }
    #[derive(Deserialize)]
    struct Gallery {
        url: String,
        #[serde(default)]
        title: Option<String>,
    }

    let project: Project =
        get_json(http().get(format!("{}/project/{}", MODRINTH_API, project_id))).await?;

    let mut links = Vec::new();
    push_link(
        &mut links,
        "Modrinth",
        Some(format!(
            "https://modrinth.com/{}/{}",
            kind.modrinth_type(),
            project.slug
        )),
    );
    push_link(&mut links, "原始碼", project.source_url);
    push_link(&mut links, "問題回報", project.issues_url);
    push_link(&mut links, "Wiki", project.wiki_url);
    push_link(&mut links, "Discord", project.discord_url);

    Ok(ContentDetail {
        id: project.id,
        name: project.title,
        summary: project.description,
        description: truncate(project.body),
        description_format: DescriptionFormat::Markdown,
        icon_url: project.icon_url.filter(|u| !u.is_empty()),
        downloads: project.downloads,
        follows: project.followers,
        categories: project.categories,
        game_versions: project.game_versions,
        loaders: project.loaders,
        license: project.license.and_then(|l| {
            l.name
                .filter(|n| !n.trim().is_empty())
                .or(Some(l.id))
                .filter(|s| !s.trim().is_empty())
        }),
        updated: iso_to_unix(&project.updated),
        gallery: project
            .gallery
            .into_iter()
            .map(|g| GalleryImage {
                url: g.url,
                title: g.title.unwrap_or_default(),
            })
            .collect(),
        links,
    })
}

fn cf_loader_label(id: u32) -> Option<&'static str> {
    match id {
        1 => Some("forge"),
        4 => Some("fabric"),
        5 => Some("quilt"),
        6 => Some("neoforge"),
        _ => None,
    }
}

async fn curseforge_detail(mod_id: u64) -> CmdResult<ContentDetail> {
    #[derive(Deserialize)]
    struct ModResp {
        data: Project,
    }
    #[derive(Deserialize)]
    struct DescResp {
        #[serde(default, deserialize_with = "nullable")]
        data: String,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Project {
        id: u64,
        #[serde(default)]
        name: String,
        #[serde(default, deserialize_with = "nullable")]
        summary: String,
        #[serde(default)]
        download_count: u64,
        #[serde(default)]
        thumbs_up_count: u64,
        #[serde(default)]
        logo: Option<Logo>,
        #[serde(default)]
        categories: Vec<Category>,
        #[serde(default)]
        screenshots: Vec<Screenshot>,
        #[serde(default)]
        links: Option<Links>,
        #[serde(default, deserialize_with = "nullable")]
        date_modified: String,
        #[serde(default)]
        latest_files_indexes: Vec<FileIndex>,
    }
    #[derive(Deserialize)]
    struct Logo {
        #[serde(default)]
        url: Option<String>,
    }
    #[derive(Deserialize)]
    struct Category {
        #[serde(default)]
        name: String,
    }
    #[derive(Deserialize)]
    struct Screenshot {
        url: String,
        #[serde(default, deserialize_with = "nullable")]
        title: String,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Links {
        #[serde(default)]
        website_url: Option<String>,
        #[serde(default)]
        wiki_url: Option<String>,
        #[serde(default)]
        issues_url: Option<String>,
        #[serde(default)]
        source_url: Option<String>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FileIndex {
        #[serde(default, deserialize_with = "nullable")]
        game_version: String,
        #[serde(default)]
        mod_loader: Option<u32>,
    }

    let info_request = curseforge_get(&format!("/mods/{}", mod_id)).await?;
    let desc_request = curseforge_get(&format!("/mods/{}/description", mod_id)).await?;
    let (info, description) = futures::join!(
        get_json::<ModResp>(info_request),
        get_json::<DescResp>(desc_request)
    );

    let project = info?.data;

    let description = description.map(|d| truncate(d.data)).unwrap_or_default();

    let mut links = Vec::new();
    if let Some(l) = project.links {
        push_link(&mut links, "CurseForge", l.website_url);
        push_link(&mut links, "原始碼", l.source_url);
        push_link(&mut links, "問題回報", l.issues_url);
        push_link(&mut links, "Wiki", l.wiki_url);
    }

    Ok(ContentDetail {
        id: project.id.to_string(),
        name: project.name,
        summary: project.summary,
        description,
        description_format: DescriptionFormat::Html,
        icon_url: project.logo.and_then(|l| l.url).filter(|u| !u.is_empty()),
        downloads: project.download_count,
        follows: project.thumbs_up_count,
        categories: project.categories.into_iter().map(|c| c.name).collect(),
        game_versions: dedup(
            project
                .latest_files_indexes
                .iter()
                .map(|f| f.game_version.clone()),
        ),
        loaders: dedup(
            project
                .latest_files_indexes
                .iter()
                .filter_map(|f| f.mod_loader)
                .filter_map(cf_loader_label)
                .map(str::to_string),
        ),

        license: None,
        updated: iso_to_unix(&project.date_modified),
        gallery: project
            .screenshots
            .into_iter()
            .map(|s| GalleryImage {
                url: s.url,
                title: s.title,
            })
            .collect(),
        links,
    })
}

fn truncate(text: String) -> String {
    let Some((end, _)) = text.char_indices().nth(MAX_DESCRIPTION) else {
        return text;
    };
    let cut = text[..end].rfind('\n').unwrap_or(end);
    format!("{}…", text[..cut].trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_description_is_cut_on_a_line_boundary() {
        let long = format!("{}\n{}", "中".repeat(100), "文".repeat(MAX_DESCRIPTION));
        let cut = truncate(long);
        assert_eq!(cut.chars().count(), 101);
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn short_description_is_left_alone() {
        assert_eq!(truncate("<p>中文</p>".to_string()), "<p>中文</p>");
    }

    #[test]
    fn file_names_stay_in_the_target_folder() {
        assert_eq!(safe_file_name("sodium.jar", ContentKind::Mod), "sodium.jar");
        assert_eq!(
            safe_file_name("../../evil.jar", ContentKind::Mod),
            "evil.jar"
        );
        assert_eq!(
            safe_file_name("a\\b/c.zip", ContentKind::ResourcePack),
            "c.zip"
        );
        assert_eq!(safe_file_name("..", ContentKind::Mod), "download.jar");
        assert_eq!(safe_file_name("   ", ContentKind::Shader), "download.zip");
    }

    #[test]
    fn curseforge_splits_versions_from_loaders() {
        let file = CfFile {
            id: 1,
            display_name: "Sodium 0.5".to_string(),
            file_name: "sodium.jar".to_string(),
            download_url: None,
            game_versions: vec![
                "Fabric".to_string(),
                "1.20.1".to_string(),
                "Quilt".to_string(),
            ],
            release_type: 1,
            file_date: String::new(),
            hashes: Vec::new(),
        };

        assert_eq!(file.mc_version(), "1.20.1");
        assert_eq!(file.loaders(), ["fabric", "quilt"]);
    }
}
