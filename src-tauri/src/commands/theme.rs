use std::path::PathBuf;

use crate::error::{CmdError, CmdResult};
use crate::store::{now_secs, store_dir};

const PREFIX: &str = "background-";

const ALLOWED: [&str; 6] = ["png", "jpg", "jpeg", "webp", "gif", "bmp"];

const MAX_BYTES: usize = 64 * 1024 * 1024;

fn extension_of(path: &std::path::Path) -> CmdResult<String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if !ALLOWED.contains(&ext.as_str()) {
        return Err(if ext.is_empty() {
            CmdError::launcher_code(
                "theme_bad_format_noext",
                &[("allowed", &ALLOWED.join(", "))],
            )
        } else {
            CmdError::launcher_code(
                "theme_bad_format",
                &[("ext", &ext), ("allowed", &ALLOWED.join(", "))],
            )
        });
    }
    Ok(ext)
}

fn stored_path(name: &str) -> CmdResult<PathBuf> {
    let ok = name.starts_with(PREFIX)
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..");
    if !ok {
        return Err(CmdError::launcher_code(
            "not_background_name",
            &[("name", &name)],
        ));
    }
    Ok(store_dir()?.join(name))
}

async fn sweep(keep: &str) -> CmdResult<()> {
    let dir = store_dir()?;
    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(PREFIX) && name != keep {
            let _ = tokio::fs::remove_file(entry.path()).await;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn set_background_image(source: String) -> CmdResult<String> {
    let src = PathBuf::from(&source);
    let ext = extension_of(&src)?;

    let data = tokio::fs::read(&src).await?;
    if data.len() > MAX_BYTES {
        return Err(CmdError::launcher(format!(
            "圖片太大（{} MiB），請換一張小於 64 MiB 的",
            data.len() / 1024 / 1024
        )));
    }

    let name = format!("{}{}.{}", PREFIX, now_secs(), ext);
    let dir = store_dir()?;
    tokio::fs::create_dir_all(&dir).await?;
    tokio::fs::write(dir.join(&name), &data).await?;
    sweep(&name).await?;

    Ok(name)
}

#[tauri::command]
pub async fn background_image_path(name: String) -> CmdResult<String> {
    let path = stored_path(&name)?;
    extension_of(&path)?;
    if !path.is_file() {
        return Err(CmdError::launcher_code(
            "background_not_found",
            &[("name", &name)],
        ));
    }
    Ok(crate::commands::path_string(&path))
}

#[tauri::command]
pub async fn clear_background_image() -> CmdResult<()> {
    sweep("").await
}
