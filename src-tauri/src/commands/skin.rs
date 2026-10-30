use std::path::PathBuf;
use std::time::Duration;

use base64::Engine;
use serde::Deserialize;

use super::icon::data_url;
use super::modpack_browse::{get_json, http};
use crate::error::{CmdError, CmdResult};
use crate::store::store_dir;

const PROFILE_API: &str = "https://sessionserver.mojang.com/session/minecraft/profile";
const FRESH_FOR: Duration = Duration::from_secs(60 * 60);

#[derive(Deserialize)]
struct Profile {
    #[serde(default)]
    properties: Vec<Property>,
}

#[derive(Deserialize)]
struct Property {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct Textures {
    textures: TextureSet,
}

#[derive(Deserialize)]
struct TextureSet {
    #[serde(rename = "SKIN")]
    skin: Option<Texture>,
}

#[derive(Deserialize)]
struct Texture {
    url: String,
}

pub(super) fn cache_path(uuid: &str) -> CmdResult<PathBuf> {
    Ok(store_dir()?.join("skins").join(format!("{}.png", uuid)))
}

fn is_fresh(path: &PathBuf) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age < FRESH_FOR)
}

async fn fetch_skin(uuid: &str) -> CmdResult<Option<Vec<u8>>> {
    let profile: Profile = get_json(http().get(format!("{}/{}", PROFILE_API, uuid))).await?;
    let Some(encoded) = profile
        .properties
        .into_iter()
        .find(|p| p.name == "textures")
        .map(|p| p.value)
    else {
        return Ok(None);
    };

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|e| CmdError::launcher(format!("皮膚資料格式錯誤: {}", e)))?;
    let textures: Textures = serde_json::from_slice(&decoded)?;
    let Some(skin) = textures.textures.skin else {
        return Ok(None);
    };

    let url = skin.url.replacen("http://", "https://", 1);
    let bytes = http()
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    Ok(Some(bytes.to_vec()))
}

#[tauri::command]
pub async fn player_skin(uuid: String) -> CmdResult<Option<String>> {
    let uuid = uuid.replace('-', "").to_ascii_lowercase();
    if uuid.len() != 32 || !uuid.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(None);
    }

    let path = cache_path(&uuid)?;
    let cached = || std::fs::read(&path).ok().and_then(|b| data_url(&b));

    if is_fresh(&path) {
        if let Some(url) = cached() {
            return Ok(Some(url));
        }
    }

    match fetch_skin(&uuid).await {
        Ok(Some(bytes)) => {
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::write(&path, &bytes).await?;
            Ok(data_url(&bytes))
        }
        Ok(None) => {
            let _ = tokio::fs::remove_file(&path).await;
            Ok(None)
        }
        Err(_) => Ok(cached()),
    }
}
