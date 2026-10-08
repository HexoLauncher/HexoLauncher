use std::path::{Path, PathBuf};

use base64::Engine;
use tauri::Emitter;

use super::instance::EVENT_INSTANCES_CHANGED;
use super::instance_dir;
use super::modpack_browse::http;
use crate::error::{CmdError, CmdResult};
use crate::store::instance_meta::{InstanceMeta, DEFAULT_ICON};

pub const ICON_FILE: &str = "icon.png";
pub const CUSTOM_ICON: &str = "custom";

const MAX_BYTES: usize = 4 * 1024 * 1024;

fn mime_of(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => Some("image/png"),
        [0xFF, 0xD8, 0xFF, ..] => Some("image/jpeg"),
        [b'G', b'I', b'F', b'8', ..] => Some("image/gif"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some("image/webp"),
        [b'B', b'M', ..] => Some("image/bmp"),
        _ => None,
    }
}

pub fn data_url(bytes: &[u8]) -> Option<String> {
    mime_of(bytes).map(|mime| {
        format!(
            "data:{};base64,{}",
            mime,
            base64::engine::general_purpose::STANDARD.encode(bytes)
        )
    })
}

fn validate(bytes: &[u8]) -> CmdResult<()> {
    if bytes.len() > MAX_BYTES {
        return Err(CmdError::launcher(format!(
            "圖示太大（{} KiB），請換一張小於 4 MiB 的",
            bytes.len() / 1024
        )));
    }
    if mime_of(bytes).is_none() {
        return Err(CmdError::launcher(
            "不支援的圖片格式（可用 PNG、JPG、GIF、WebP、BMP）",
        ));
    }
    Ok(())
}

pub fn read_icon(dir: &Path) -> Option<String> {
    std::fs::read(dir.join(ICON_FILE))
        .ok()
        .and_then(|bytes| data_url(&bytes))
}

async fn write_icon(dir: &Path, bytes: &[u8]) -> CmdResult<()> {
    validate(bytes)?;
    tokio::fs::create_dir_all(dir).await?;
    tokio::fs::write(dir.join(ICON_FILE), bytes).await?;
    Ok(())
}

pub async fn copy_icon(dir: &Path, source: &Path) -> CmdResult<()> {
    write_icon(dir, &tokio::fs::read(source).await?).await
}

pub async fn download_icon(dir: &Path, url: &str) -> CmdResult<()> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(CmdError::launcher(format!("不是網址: {}", url)));
    }
    let bytes = http()
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    write_icon(dir, &bytes).await
}

pub async fn apply_source(
    dir: &Path,
    meta: &mut InstanceMeta,
    path: Option<&str>,
    url: Option<&str>,
) {
    if meta.icon != CUSTOM_ICON {
        return;
    }
    let path = path.map(str::trim).filter(|p| !p.is_empty());
    let url = url.map(str::trim).filter(|u| !u.is_empty());

    let placed = match (path, url) {
        (Some(path), _) => copy_icon(dir, Path::new(path)).await.is_ok(),
        (None, Some(url)) => download_icon(dir, url).await.is_ok(),
        (None, None) => dir.join(ICON_FILE).is_file(),
    };
    if !placed {
        meta.icon = DEFAULT_ICON.to_string();
    }
}

#[tauri::command]
pub async fn read_image_file(path: String) -> CmdResult<String> {
    let bytes = tokio::fs::read(&path).await?;
    validate(&bytes)?;
    Ok(data_url(&bytes).unwrap_or_default())
}

#[tauri::command]
pub async fn set_instance_icon_image(
    app: tauri::AppHandle,
    base_dir: String,
    instance_name: String,
    path: String,
) -> CmdResult<String> {
    let dir: PathBuf = instance_dir(Path::new(&base_dir), &instance_name)?;
    if !dir.exists() {
        return Err(CmdError::new(
            "instanceNotFound",
            format!("找不到 instance: {}", instance_name),
        ));
    }
    copy_icon(&dir, Path::new(&path)).await?;
    let _ = app.emit(EVENT_INSTANCES_CHANGED, ());
    Ok(read_icon(&dir).unwrap_or_default())
}
