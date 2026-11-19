use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::icon::data_url;
use super::modpack_browse::http;
use super::skin::cache_path;
use crate::error::{CmdError, CmdResult};
use crate::store::accounts::resolve;
use crate::store::{now_secs, read_json, store_dir, store_file, write_json};

const FILE: &str = "wardrobe.json";
const PROFILE_API: &str = "https://api.minecraftservices.com/minecraft/profile";
const MAX_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkinVariant {
    #[default]
    Classic,
    Slim,
}

impl SkinVariant {
    fn api(self) -> &'static str {
        match self {
            SkinVariant::Classic => "classic",
            SkinVariant::Slim => "slim",
        }
    }

    fn from_api(value: &str) -> Self {
        if value.eq_ignore_ascii_case("slim") {
            SkinVariant::Slim
        } else {
            SkinVariant::Classic
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredSkin {
    id: String,
    name: String,
    variant: SkinVariant,
    sha256: String,
    added_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct WardrobeStore {
    skins: Vec<StoredSkin>,
}

impl WardrobeStore {
    async fn load() -> CmdResult<Self> {
        Ok(read_json(&store_file(FILE)?).await?.unwrap_or_default())
    }

    async fn save(&self) -> CmdResult<()> {
        write_json(&store_file(FILE)?, self).await
    }

    fn find(&self, id: &str) -> CmdResult<&StoredSkin> {
        self.skins
            .iter()
            .find(|s| s.id == id)
            .ok_or_else(|| CmdError::launcher_code("skin_not_found", &[("id", &id)]))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WardrobeSkin {
    pub id: String,
    pub name: String,
    pub variant: SkinVariant,
    pub legacy: bool,
    pub image: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSkin {
    pub variant: SkinVariant,
    pub legacy: bool,
    pub image: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cape {
    pub id: String,
    pub alias: String,
    pub active: bool,
    pub image: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinProfile {
    pub skin: Option<ActiveSkin>,
    pub capes: Vec<Cape>,
}

#[derive(Debug, Deserialize)]
struct ProfileResponse {
    #[serde(default)]
    skins: Vec<ProfileTexture>,
    #[serde(default)]
    capes: Vec<ProfileTexture>,
}

#[derive(Debug, Deserialize)]
struct ProfileTexture {
    id: String,
    state: String,
    url: String,
    #[serde(default)]
    variant: Option<String>,
    #[serde(default)]
    alias: Option<String>,
}

fn wardrobe_dir() -> CmdResult<PathBuf> {
    Ok(store_dir()?.join("wardrobe"))
}

fn skin_file(id: &str) -> CmdResult<PathBuf> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return Err(CmdError::launcher_code("invalid_id", &[("id", &id)]));
    }
    Ok(wardrobe_dir()?.join(format!("{}.png", id)))
}

fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((width, height))
}

fn check_skin(bytes: &[u8]) -> CmdResult<bool> {
    if bytes.len() > MAX_BYTES {
        return Err(CmdError::launcher_code(
            "skin_too_large",
            &[("size", &(bytes.len() / 1024))],
        ));
    }
    match png_size(bytes) {
        Some((64, 64)) => Ok(false),
        Some((64, 32)) => Ok(true),
        Some((w, h)) => Err(CmdError::launcher_code(
            "skin_dimensions",
            &[("width", &w), ("height", &h)],
        )),
        None => Err(CmdError::launcher_code("skin_must_be_png", &[])),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn image_url(bytes: &[u8]) -> String {
    data_url(bytes).unwrap_or_default()
}

async fn views(store: &WardrobeStore) -> Vec<WardrobeSkin> {
    let mut out = Vec::with_capacity(store.skins.len());
    for skin in &store.skins {
        let Ok(path) = skin_file(&skin.id) else {
            continue;
        };
        let Ok(bytes) = tokio::fs::read(&path).await else {
            continue;
        };
        out.push(WardrobeSkin {
            id: skin.id.clone(),
            name: skin.name.clone(),
            variant: skin.variant,
            legacy: png_size(&bytes) == Some((64, 32)),
            image: image_url(&bytes),
            sha256: skin.sha256.clone(),
        });
    }
    out
}

async fn insert(
    store: &mut WardrobeStore,
    bytes: &[u8],
    name: &str,
    variant: SkinVariant,
) -> CmdResult<()> {
    check_skin(bytes)?;
    let sha256 = sha256_hex(bytes);
    if let Some(existing) = store.skins.iter().find(|s| s.sha256 == sha256) {
        return Err(CmdError::launcher_code(
            "skin_duplicate",
            &[("name", &existing.name)],
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    tokio::fs::create_dir_all(wardrobe_dir()?).await?;
    tokio::fs::write(skin_file(&id)?, bytes).await?;
    let name = name.trim();
    store.skins.push(StoredSkin {
        id,
        name: if name.is_empty() {
            "未命名".into()
        } else {
            name.into()
        },
        variant,
        sha256,
        added_at: now_secs(),
    });
    Ok(())
}

async fn account_token(account_id: &str) -> CmdResult<(String, String)> {
    let account = resolve(Some(account_id)).await?;
    match (account.access_token, account.uuid) {
        (Some(token), Some(uuid)) => Ok((token, uuid)),
        _ => Err(CmdError::launcher_code("offline_no_skin", &[])),
    }
}

async fn services(request: reqwest::RequestBuilder) -> CmdResult<ProfileResponse> {
    let response = request.send().await?;
    let status = response.status();
    if status.is_success() {
        return Ok(response.json().await?);
    }
    let body = response.text().await.unwrap_or_default();
    let kind = if status == reqwest::StatusCode::UNAUTHORIZED {
        "authError"
    } else {
        "launcher"
    };
    let body = body.trim();
    Err(match status.as_u16() {
        401 => CmdError::coded(kind, "skin_service_401", &[]),
        429 => CmdError::coded(kind, "skin_service_429", &[]),
        code if body.is_empty() => CmdError::coded(kind, "skin_service_http", &[("status", &code)]),
        code => CmdError::coded(
            kind,
            "skin_service_http_body",
            &[("status", &code), ("body", &body)],
        ),
    })
}

async fn download(url: &str) -> CmdResult<Vec<u8>> {
    let url = url.replacen("http://", "https://", 1);
    Ok(http()
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec())
}

async fn to_profile(response: ProfileResponse) -> CmdResult<SkinProfile> {
    let skin = match response.skins.iter().find(|s| s.state == "ACTIVE") {
        Some(active) => {
            let bytes = download(&active.url).await?;
            Some(ActiveSkin {
                variant: SkinVariant::from_api(active.variant.as_deref().unwrap_or_default()),
                legacy: png_size(&bytes) == Some((64, 32)),
                sha256: sha256_hex(&bytes),
                image: image_url(&bytes),
            })
        }
        None => None,
    };

    let mut capes = Vec::with_capacity(response.capes.len());
    for cape in &response.capes {
        let image = download(&cape.url)
            .await
            .map(|b| image_url(&b))
            .unwrap_or_default();
        capes.push(Cape {
            id: cape.id.clone(),
            alias: cape.alias.clone().unwrap_or_default(),
            active: cape.state == "ACTIVE",
            image,
        });
    }
    Ok(SkinProfile { skin, capes })
}

async fn fetch_profile(token: &str) -> CmdResult<ProfileResponse> {
    services(http().get(PROFILE_API).bearer_auth(token)).await
}

#[tauri::command]
pub async fn wardrobe_list() -> CmdResult<Vec<WardrobeSkin>> {
    Ok(views(&WardrobeStore::load().await?).await)
}

#[tauri::command]
pub async fn wardrobe_add(
    path: String,
    name: String,
    variant: SkinVariant,
) -> CmdResult<Vec<WardrobeSkin>> {
    let bytes = tokio::fs::read(&path).await?;
    let mut store = WardrobeStore::load().await?;
    insert(&mut store, &bytes, &name, variant).await?;
    store.save().await?;
    Ok(views(&store).await)
}

#[tauri::command]
pub async fn wardrobe_update(
    id: String,
    name: String,
    variant: SkinVariant,
) -> CmdResult<Vec<WardrobeSkin>> {
    let mut store = WardrobeStore::load().await?;
    store.find(&id)?;
    if let Some(skin) = store.skins.iter_mut().find(|s| s.id == id) {
        let name = name.trim();
        if !name.is_empty() {
            skin.name = name.into();
        }
        skin.variant = variant;
    }
    store.save().await?;
    Ok(views(&store).await)
}

#[tauri::command]
pub async fn wardrobe_remove(id: String) -> CmdResult<Vec<WardrobeSkin>> {
    let mut store = WardrobeStore::load().await?;
    store.find(&id)?;
    store.skins.retain(|s| s.id != id);
    store.save().await?;
    let _ = tokio::fs::remove_file(skin_file(&id)?).await;
    Ok(views(&store).await)
}

#[tauri::command]
pub async fn skin_profile(account_id: String) -> CmdResult<SkinProfile> {
    let (token, _) = account_token(&account_id).await?;
    to_profile(fetch_profile(&token).await?).await
}

#[tauri::command]
pub async fn wardrobe_save_current(
    account_id: String,
    name: String,
) -> CmdResult<Vec<WardrobeSkin>> {
    let (token, _) = account_token(&account_id).await?;
    let profile = fetch_profile(&token).await?;
    let active = profile
        .skins
        .iter()
        .find(|s| s.state == "ACTIVE")
        .ok_or_else(|| CmdError::launcher_code("account_no_skin", &[]))?;
    let bytes = download(&active.url).await?;
    let variant = SkinVariant::from_api(active.variant.as_deref().unwrap_or_default());

    let mut store = WardrobeStore::load().await?;
    insert(&mut store, &bytes, &name, variant).await?;
    store.save().await?;
    Ok(views(&store).await)
}

#[tauri::command]
pub async fn wardrobe_apply(account_id: String, id: String) -> CmdResult<SkinProfile> {
    let store = WardrobeStore::load().await?;
    let skin = store.find(&id)?.clone();
    let bytes = tokio::fs::read(skin_file(&id)?).await?;
    check_skin(&bytes)?;

    let (token, uuid) = account_token(&account_id).await?;
    let file = reqwest::multipart::Part::bytes(bytes.clone())
        .file_name("skin.png")
        .mime_str("image/png")?;
    let form = reqwest::multipart::Form::new()
        .text("variant", skin.variant.api())
        .part("file", file);
    let response = services(
        http()
            .post(format!("{}/skins", PROFILE_API))
            .bearer_auth(&token)
            .multipart(form),
    )
    .await?;

    let cache = cache_path(&uuid.replace('-', "").to_ascii_lowercase())?;
    if let Some(parent) = cache.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let _ = tokio::fs::write(&cache, &bytes).await;

    to_profile(response).await
}

#[tauri::command]
pub async fn set_cape(account_id: String, cape_id: Option<String>) -> CmdResult<SkinProfile> {
    let (token, _) = account_token(&account_id).await?;
    let url = format!("{}/capes/active", PROFILE_API);
    let response = match cape_id {
        Some(cape_id) => {
            services(
                http()
                    .put(url)
                    .bearer_auth(&token)
                    .json(&serde_json::json!({ "capeId": cape_id })),
            )
            .await?
        }
        None => services(http().delete(url).bearer_auth(&token)).await?,
    };
    to_profile(response).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes
    }

    #[test]
    fn accepts_only_skin_sizes() {
        assert_eq!(check_skin(&png(64, 64)).unwrap(), false);
        assert_eq!(check_skin(&png(64, 32)).unwrap(), true);
        assert!(check_skin(&png(128, 128)).is_err());
        assert!(check_skin(b"GIF89a").is_err());
    }

    #[test]
    fn skin_ids_cannot_escape() {
        assert!(skin_file("0b6f6a1e-2c2f-4a57-9b7e-3f7c1d2a9e10").is_ok());
        for bad in ["", "../accounts", "a/b", "x.json"] {
            assert!(skin_file(bad).is_err(), "{bad}");
        }
    }
}
