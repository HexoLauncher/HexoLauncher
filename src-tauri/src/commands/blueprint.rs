use std::collections::HashMap;
use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::modpack_browse::http;
use super::{minecraft_dir, path_string};
use crate::error::{CmdError, CmdResult};
use crate::store::accounts::{self, AccountKind, AccountStore};
use crate::store::{now_secs, read_json, store_dir, store_file, write_json};

const AUTH_FILE: &str = "blueprint_auth.json";
const MOJANG_JOIN: &str = "https://sessionserver.mojang.com/session/minecraft/join";

const SERVER: Option<&str> = match option_env!("NOLAUNCHER_BLUEPRINT_SERVER") {
    Some(url) if !url.is_empty() => Some(url),
    _ if cfg!(debug_assertions) => Some("http://localhost:8080"),
    _ => None,
};

pub fn cache_dir() -> CmdResult<PathBuf> {
    Ok(store_dir()?.join("blueprint-cache"))
}

fn server_url() -> CmdResult<String> {
    SERVER
        .map(|url| url.trim_end_matches('/').to_string())
        .ok_or_else(|| CmdError::new("blueprintUnavailable", "這個版本沒有設定藍圖庫伺服器"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct User {
    pub uuid: String,
    pub username: String,
    #[serde(alias = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Owner {
    pub uuid: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRange {
    pub min: Option<String>,
    pub max: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct Cover {
    pub url: Option<String>,
    pub source: Option<String>,
    pub image_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct Render {
    pub status: String,
    pub url: Option<String>,
    pub error: Option<String>,
    pub mc_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub id: String,
    pub url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct Blueprint {
    pub id: String,
    pub owner: Owner,
    pub name: String,
    pub description: String,
    pub format: String,
    pub tags: Vec<String>,
    pub mc_versions: VersionRange,
    pub data_version: Option<i64>,
    pub size: Option<Vec<i64>>,
    pub total_blocks: Option<i64>,
    pub file: FileInfo,
    pub cover: Cover,
    pub render: Render,
    pub images: Vec<Image>,
    pub download_count: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct BlueprintList {
    pub items: Vec<Blueprint>,
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
}

#[derive(Debug, Deserialize)]
struct Challenge {
    server_id: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    token: String,
    user: User,
}

#[derive(Debug, Deserialize)]
struct ServerError {
    error: String,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredSession {
    token: String,
    expires_at: u64,
    user: User,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct AuthStore {
    sessions: HashMap<String, StoredSession>,
    terms: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct TermsResponse {
    version: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintTerms {
    pub version: String,
    pub content: String,
    pub accepted: bool,
}

impl AuthStore {
    async fn load() -> CmdResult<Self> {
        Ok(read_json(&store_file(AUTH_FILE)?)
            .await?
            .unwrap_or_default())
    }

    async fn save(&self) -> CmdResult<()> {
        write_json(&store_file(AUTH_FILE)?, self).await
    }
}

fn session_key(server: &str, uuid: &str) -> String {
    format!("{}|{}", server, plain_uuid(uuid))
}

fn plain_uuid(uuid: &str) -> String {
    uuid.replace('-', "").to_ascii_lowercase()
}

fn jwt_exp(token: &str) -> u64 {
    let Some(payload) = token.split('.').nth(1) else {
        return 0;
    };
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|claims| claims.get("exp").and_then(|v| v.as_u64()))
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionAccount {
    pub id: String,
    pub player_name: String,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintSession {
    pub server: String,
    /// 啟動器目前選的帳號；離線帳號可以瀏覽和下載，但不能登入藍圖庫。
    pub account: Option<SessionAccount>,
    /// 已登入藍圖庫時的使用者。
    pub user: Option<User>,
}

async fn current_session() -> CmdResult<(BlueprintSession, Option<String>)> {
    let server = server_url()?;
    let store = AccountStore::load().await?;
    let selected = store
        .selected
        .as_deref()
        .and_then(|id| store.accounts.iter().find(|a| a.id == id));

    let Some(account) = selected else {
        return Ok((
            BlueprintSession {
                server,
                account: None,
                user: None,
            },
            None,
        ));
    };

    let online = account.kind == AccountKind::Microsoft;
    let mut user = None;
    let mut token = None;
    if online {
        let auth = AuthStore::load().await?;
        if let Some(saved) = auth.sessions.get(&session_key(&server, &account.uuid)) {
            if saved.expires_at == 0 || saved.expires_at > now_secs() + 60 {
                user = Some(saved.user.clone());
                token = Some(saved.token.clone());
            }
        }
    }

    Ok((
        BlueprintSession {
            server,
            account: Some(SessionAccount {
                id: account.id.clone(),
                player_name: account.player_name.clone(),
                online,
            }),
            user,
        },
        token,
    ))
}

async fn forget_session(server: &str, uuid: &str) -> CmdResult<()> {
    let mut auth = AuthStore::load().await?;
    if auth.sessions.remove(&session_key(server, uuid)).is_some() {
        auth.save().await?;
    }
    Ok(())
}

fn not_signed_in() -> CmdError {
    CmdError::new("blueprintAuth", "還沒有登入藍圖庫")
}

/// 帶 JWT 的請求要用的東西：伺服器網址、token、以及 401 時要清掉的 UUID。
struct Authed {
    server: String,
    token: String,
    uuid: String,
}

async fn authed() -> CmdResult<Authed> {
    let (session, token) = current_session().await?;
    let (Some(token), Some(user)) = (token, session.user) else {
        return Err(not_signed_in());
    };
    Ok(Authed {
        server: session.server,
        token,
        uuid: user.uuid,
    })
}

async fn error_from(response: reqwest::Response) -> CmdError {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let message = match serde_json::from_str::<ServerError>(&body) {
        Ok(err) => format!("{}（{}）", err.message, err.error),
        Err(_) if !body.trim().is_empty() => format!("HTTP {}: {}", status.as_u16(), body.trim()),
        Err(_) => format!("HTTP {}", status.as_u16()),
    };
    let terms_required = serde_json::from_str::<ServerError>(&body)
        .is_ok_and(|err| err.error == "terms_required");
    let kind = if terms_required {
        "blueprintTerms"
    } else if status == reqwest::StatusCode::UNAUTHORIZED {
        "blueprintAuth"
    } else {
        "blueprint"
    };
    CmdError::new(kind, message)
}

async fn send(request: reqwest::RequestBuilder) -> CmdResult<reqwest::Response> {
    let response = request.send().await?;
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(error_from(response).await)
    }
}

async fn send_json<T: serde::de::DeserializeOwned>(
    request: reqwest::RequestBuilder,
) -> CmdResult<T> {
    Ok(send(request).await?.json().await?)
}

async fn send_authed(
    auth: &Authed,
    request: reqwest::RequestBuilder,
) -> CmdResult<reqwest::Response> {
    let result = send(request.bearer_auth(&auth.token)).await;
    if let Err(err) = &result {
        if err.kind == "blueprintAuth" {
            let _ = forget_session(&auth.server, &auth.uuid).await;
        }
    }
    result
}

async fn authed_json<T: serde::de::DeserializeOwned>(
    auth: &Authed,
    request: reqwest::RequestBuilder,
) -> CmdResult<T> {
    Ok(send_authed(auth, request).await?.json().await?)
}

fn check_id(id: &str) -> CmdResult<&str> {
    let ok = !id.is_empty() && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    if !ok {
        return Err(CmdError::launcher(format!("id 不合法: {}", id)));
    }
    Ok(id)
}

fn blueprint_url(server: &str, id: &str) -> CmdResult<String> {
    Ok(format!("{}/blueprints/{}", server, check_id(id)?))
}

#[tauri::command]
pub async fn blueprint_session() -> CmdResult<BlueprintSession> {
    Ok(current_session().await?.0)
}

#[tauri::command]
pub async fn blueprint_terms() -> CmdResult<Option<BlueprintTerms>> {
    let server = server_url()?;
    let response = http().get(format!("{}/terms", server)).send().await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(error_from(response).await);
    }
    let terms: TermsResponse = response.json().await?;
    let auth = AuthStore::load().await?;
    Ok(Some(BlueprintTerms {
        accepted: auth.terms.get(&server) == Some(&terms.version),
        version: terms.version,
        content: terms.content,
    }))
}

#[tauri::command]
pub async fn blueprint_login(terms_version: Option<String>) -> CmdResult<BlueprintSession> {
    let server = server_url()?;
    let terms_version = match terms_version {
        Some(version) => Some(version),
        None => AuthStore::load().await?.terms.get(&server).cloned(),
    };
    let account = accounts::resolve(None).await?;
    let (Some(access_token), Some(uuid)) = (account.access_token, account.uuid) else {
        return Err(CmdError::new(
            "blueprintAuth",
            "離線帳號不能登入藍圖庫，請切換到 Microsoft 帳號",
        ));
    };

    let challenge: Challenge = send_json(
        http()
            .post(format!("{}/auth/challenge", server))
            .header(reqwest::header::CONTENT_LENGTH, 0),
    )
    .await?;

    let joined = http()
        .post(MOJANG_JOIN)
        .json(&serde_json::json!({
            "accessToken": access_token,
            "selectedProfile": plain_uuid(&uuid),
            "serverId": challenge.server_id,
        }))
        .send()
        .await?;
    if !joined.status().is_success() {
        return Err(CmdError::new(
            "blueprintAuth",
            format!(
                "Mojang 拒絕了這次驗證（HTTP {}），請到設定重新整理或重新登入帳號",
                joined.status().as_u16()
            ),
        ));
    }

    let login: LoginResponse = send_json(http().post(format!("{}/auth/login", server)).json(
        &serde_json::json!({
            "username": account.player_name,
            "server_id": challenge.server_id,
            "terms_version": terms_version,
        }),
    ))
    .await?;

    let mut auth = AuthStore::load().await?;
    if let Some(version) = terms_version {
        auth.terms.insert(server.clone(), version);
    }
    auth.sessions.insert(
        session_key(&server, &uuid),
        StoredSession {
            expires_at: jwt_exp(&login.token),
            token: login.token,
            user: login.user,
        },
    );
    auth.save().await?;

    blueprint_session().await
}

#[tauri::command]
pub async fn blueprint_logout() -> CmdResult<BlueprintSession> {
    let (session, _) = current_session().await?;
    if let Some(user) = &session.user {
        forget_session(&session.server, &user.uuid).await?;
    }
    blueprint_session().await
}

#[tauri::command]
pub async fn blueprint_tags() -> CmdResult<Vec<Tag>> {
    let server = server_url()?;
    send_json(http().get(format!("{}/tags", server))).await
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BlueprintQuery {
    pub q: String,
    pub owner: Option<String>,
    pub tags: Vec<String>,
    pub version: Option<String>,
    pub page: u32,
    pub per_page: u32,
}

impl Default for BlueprintQuery {
    fn default() -> Self {
        Self {
            q: String::new(),
            owner: None,
            tags: Vec::new(),
            version: None,
            page: 1,
            per_page: 24,
        }
    }
}

#[tauri::command]
pub async fn blueprint_search(query: BlueprintQuery) -> CmdResult<BlueprintList> {
    let server = server_url()?;
    let mut params: Vec<(&str, String)> = vec![
        ("page", query.page.max(1).to_string()),
        ("per_page", query.per_page.clamp(1, 100).to_string()),
    ];
    let q = query.q.trim();
    if !q.is_empty() {
        params.push(("q", q.to_string()));
    }
    if let Some(owner) = query
        .owner
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        params.push(("owner", owner.to_string()));
    }
    if !query.tags.is_empty() {
        params.push(("tags", query.tags.join(",")));
    }
    if let Some(version) = query
        .version
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        params.push(("version", version.to_string()));
    }

    send_json(http().get(format!("{}/blueprints", server)).query(&params)).await
}

#[tauri::command]
pub async fn blueprint_get(id: String) -> CmdResult<Blueprint> {
    let server = server_url()?;
    send_json(http().get(blueprint_url(&server, &id)?)).await
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn extension_of(blueprint: &Blueprint) -> &'static str {
    if blueprint.format == "schem" {
        "schem"
    } else {
        "litematic"
    }
}

async fn fetch_file(blueprint: &Blueprint) -> CmdResult<Vec<u8>> {
    let sha = blueprint.file.sha256.to_ascii_lowercase();
    let cached = cache_dir()?.join(format!("{}.{}", sha, extension_of(blueprint)));
    if let Ok(bytes) = tokio::fs::read(&cached).await {
        if sha256_hex(&bytes) == sha {
            return Ok(bytes);
        }
    }

    let server = server_url()?;
    let url = format!("{}/download", blueprint_url(&server, &blueprint.id)?);
    let bytes = send(http().get(url)).await?.bytes().await?.to_vec();
    if sha256_hex(&bytes) != sha {
        return Err(CmdError::new(
            "checksumMismatch",
            format!("{} 下載後的 sha256 不符，請再試一次", blueprint.file.name),
        ));
    }

    if let Some(parent) = cached.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&cached, &bytes).await?;
    Ok(bytes)
}

pub fn safe_file_name(name: &str, fallback: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let cleaned = cleaned.trim().trim_start_matches('.').to_string();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

async fn free_path(dir: &Path, file_name: &str) -> PathBuf {
    let candidate = dir.join(file_name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = match file_name.rsplit_once('.') {
        Some((stem, ext)) => (stem.to_string(), format!(".{}", ext)),
        None => (file_name.to_string(), String::new()),
    };
    (2..)
        .map(|n| dir.join(format!("{} ({}){}", stem, n, ext)))
        .find(|p| !p.exists())
        .unwrap_or(candidate)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledBlueprint {
    pub file_name: String,
    pub path: String,
    pub existed: bool,
}

#[tauri::command]
pub async fn blueprint_install(
    base_dir: String,
    instance_name: String,
    id: String,
) -> CmdResult<InstalledBlueprint> {
    let blueprint = blueprint_get(id).await?;
    let bytes = fetch_file(&blueprint).await?;

    let dir = minecraft_dir(Path::new(&base_dir), &instance_name)?.join("schematics");
    tokio::fs::create_dir_all(&dir).await?;

    let fallback = format!("{}.{}", blueprint.id, extension_of(&blueprint));
    let file_name = safe_file_name(&blueprint.file.name, &fallback);

    let same = dir.join(&file_name);
    if let Ok(existing) = tokio::fs::read(&same).await {
        if sha256_hex(&existing) == blueprint.file.sha256.to_ascii_lowercase() {
            return Ok(InstalledBlueprint {
                file_name,
                path: path_string(&same),
                existed: true,
            });
        }
    }

    let target = free_path(&dir, &file_name).await;
    tokio::fs::write(&target, &bytes).await?;
    Ok(InstalledBlueprint {
        file_name: target
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or(file_name),
        path: path_string(&target),
        existed: false,
    })
}

#[tauri::command]
pub async fn blueprint_save_as(id: String, target_path: String) -> CmdResult<String> {
    let blueprint = blueprint_get(id).await?;
    let bytes = fetch_file(&blueprint).await?;
    tokio::fs::write(&target_path, &bytes).await?;
    Ok(target_path)
}

#[tauri::command]
pub async fn blueprint_preview(id: String, format: String) -> CmdResult<String> {
    if format != "litematic" {
        return Err(CmdError::launcher("目前只能預覽 .litematic"));
    }
    let server = server_url()?;
    let url = format!("{}/preview", blueprint_url(&server, &id)?);
    let bytes = send(http().get(url)).await?.bytes().await?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadRequest {
    pub path: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub mc_version_min: String,
    #[serde(default)]
    pub mc_version_max: String,
}

fn file_part(path: &Path, bytes: Vec<u8>, mime: &str) -> CmdResult<reqwest::multipart::Part> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    Ok(reqwest::multipart::Part::bytes(bytes)
        .file_name(name)
        .mime_str(mime)?)
}

#[tauri::command]
pub async fn blueprint_upload(request: UploadRequest) -> CmdResult<Blueprint> {
    let auth = authed().await?;
    let path = PathBuf::from(&request.path);
    let lower = request.path.to_ascii_lowercase();
    if !lower.ends_with(".litematic") && !lower.ends_with(".schem") {
        return Err(CmdError::launcher("只能上傳 .litematic 或 .schem"));
    }
    let bytes = tokio::fs::read(&path).await?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", file_part(&path, bytes, "application/octet-stream")?);
    for (key, value) in [
        ("name", &request.name),
        ("description", &request.description),
        ("mc_version_min", &request.mc_version_min),
        ("mc_version_max", &request.mc_version_max),
    ] {
        let value = value.trim();
        if !value.is_empty() {
            form = form.text(key, value.to_string());
        }
    }
    for tag in &request.tags {
        form = form.text("tags", tag.clone());
    }

    authed_json(
        &auth,
        http()
            .post(format!("{}/blueprints", auth.server))
            .multipart(form),
    )
    .await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "snake_case", deserialize = "camelCase"))]
pub struct BlueprintPatch {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub mc_version_min: String,
    pub mc_version_max: String,
}

#[tauri::command]
pub async fn blueprint_update(id: String, patch: BlueprintPatch) -> CmdResult<Blueprint> {
    let auth = authed().await?;
    authed_json(
        &auth,
        http().patch(blueprint_url(&auth.server, &id)?).json(&patch),
    )
    .await
}

#[tauri::command]
pub async fn blueprint_delete(id: String) -> CmdResult<()> {
    let auth = authed().await?;
    send_authed(&auth, http().delete(blueprint_url(&auth.server, &id)?)).await?;
    Ok(())
}

#[tauri::command]
pub async fn blueprint_rerender(id: String) -> CmdResult<Blueprint> {
    let auth = authed().await?;
    let url = format!("{}/rerender", blueprint_url(&auth.server, &id)?);
    authed_json(
        &auth,
        http().post(url).header(reqwest::header::CONTENT_LENGTH, 0),
    )
    .await
}

fn image_mime(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

#[tauri::command]
pub async fn blueprint_add_images(id: String, paths: Vec<String>) -> CmdResult<Blueprint> {
    let auth = authed().await?;
    if paths.is_empty() {
        return blueprint_get(id).await;
    }

    let mut form = reqwest::multipart::Form::new();
    for raw in &paths {
        let path = PathBuf::from(raw);
        let mime = image_mime(&path)
            .ok_or_else(|| CmdError::launcher(format!("只接受 PNG、JPEG、WebP: {}", raw)))?;
        let bytes = tokio::fs::read(&path).await?;
        form = form.part("image", file_part(&path, bytes, mime)?);
    }

    let url = format!("{}/images", blueprint_url(&auth.server, &id)?);
    authed_json(&auth, http().post(url).multipart(form)).await
}

#[tauri::command]
pub async fn blueprint_delete_image(id: String, image_id: String) -> CmdResult<Blueprint> {
    let auth = authed().await?;
    let url = format!(
        "{}/images/{}",
        blueprint_url(&auth.server, &id)?,
        check_id(&image_id)?
    );
    authed_json(&auth, http().delete(url)).await
}

#[tauri::command]
pub async fn blueprint_set_cover(id: String, image_id: Option<String>) -> CmdResult<Blueprint> {
    let auth = authed().await?;
    let url = format!("{}/cover", blueprint_url(&auth.server, &id)?);
    authed_json(
        &auth,
        http()
            .put(url)
            .json(&serde_json::json!({ "image_id": image_id })),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_rejects_path_tricks() {
        assert!(check_id("0b6f6a1e-2c2f-4a57-9b7e-3f7c1d2a9e10").is_ok());
        for bad in ["", "../users/me", "abc/def", "x?y=1", "%2e%2e"] {
            assert!(check_id(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn file_names_stay_in_folder() {
        assert_eq!(safe_file_name("農場.litematic", "x"), "農場.litematic");
        assert_eq!(
            safe_file_name("../../evil.litematic", "x"),
            "_.._evil.litematic"
        );
        assert_eq!(safe_file_name("a\\b:c.schem", "x"), "a_b_c.schem");
        assert_eq!(safe_file_name("  ", "fallback"), "fallback");
        assert_eq!(safe_file_name(".hidden", "x"), "hidden");
    }

    #[test]
    fn reads_jwt_exp() {
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"sub":"abc","exp":1893456000}"#);
        assert_eq!(jwt_exp(&format!("h.{payload}.s")), 1893456000);
        assert_eq!(jwt_exp("not-a-jwt"), 0);
    }

    #[test]
    fn session_key_ignores_uuid_format() {
        assert_eq!(
            session_key("http://x", "0B6F6A1E-2C2F-4A57-9B7E-3F7C1D2A9E10"),
            session_key("http://x", "0b6f6a1e2c2f4a579b7e3f7c1d2a9e10")
        );
    }

    #[test]
    fn auth_store_round_trips() {
        let user: User = serde_json::from_value(serde_json::json!({
            "uuid": "u", "username": "Steve", "created_at": "2026-01-01T00:00:00Z"
        }))
        .unwrap();
        let mut store = AuthStore::default();
        store.sessions.insert(
            "k".into(),
            StoredSession {
                token: "t".into(),
                expires_at: 1,
                user,
            },
        );
        let json = serde_json::to_string(&store).unwrap();
        let back: AuthStore = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sessions["k"].user.created_at, "2026-01-01T00:00:00Z");
    }

    #[test]
    fn parses_blueprint_from_server() {
        let json = r#"{
            "id": "0b6f6a1e-2c2f-4a57-9b7e-3f7c1d2a9e10",
            "owner": { "uuid": "u", "username": "Steve" },
            "name": "甘蔗機", "description": "", "format": "litematic",
            "tags": ["farm"], "mc_versions": { "min": "1.20", "max": null },
            "data_version": 3465, "size": [3, 4, 5], "total_blocks": 42,
            "file": { "name": "a.litematic", "size": 10, "sha256": "ab" },
            "cover": { "url": null, "source": null, "image_id": null },
            "render": { "status": "pending", "url": null, "error": null, "mc_version": null },
            "images": [], "download_count": 7,
            "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"
        }"#;
        let bp: Blueprint = serde_json::from_str(json).unwrap();
        let out = serde_json::to_value(&bp).unwrap();
        assert_eq!(out["mcVersions"]["min"], "1.20");
        assert_eq!(out["downloadCount"], 7);
        assert_eq!(out["render"]["mcVersion"], serde_json::Value::Null);
        assert_eq!(out["cover"]["imageId"], serde_json::Value::Null);
    }

    #[test]
    fn patch_goes_out_as_snake_case() {
        let patch: BlueprintPatch = serde_json::from_value(serde_json::json!({
            "name": "n", "description": "", "tags": [], "mcVersionMin": "1.20", "mcVersionMax": ""
        }))
        .unwrap();
        let out = serde_json::to_value(&patch).unwrap();
        assert_eq!(out["mc_version_min"], "1.20");
        assert_eq!(out["mc_version_max"], "");
    }
}
