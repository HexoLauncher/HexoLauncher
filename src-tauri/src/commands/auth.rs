use serde::Serialize;

use crate::error::{CmdError, CmdResult};
use crate::state::AppState;
use crate::store::accounts::{from_auth, ms_auth, store_account, AccountView};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeDto {
    pub session_id: String,
    pub user_code: String,
    pub verification_uri: String,
    pub message: String,
    pub expires_in: u32,
}

#[tauri::command]
pub async fn ms_login_start(state: tauri::State<'_, AppState>) -> CmdResult<DeviceCodeDto> {
    let (info, raw) = ms_auth()?.request_device_code().await?;
    let session_id = uuid::Uuid::new_v4().to_string();
    let expires_in = raw.expires_in;
    state.put_login_session(&session_id, raw)?;

    Ok(DeviceCodeDto {
        session_id,
        user_code: info.user_code,
        verification_uri: info.verification_uri,
        message: info.message,
        expires_in,
    })
}

#[tauri::command]
pub async fn ms_login_poll(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> CmdResult<Vec<AccountView>> {
    let session = state
        .take_login_session(&session_id)?
        .ok_or_else(|| CmdError::new("authError", "登入 session 不存在或已使用，請重新登入"))?;

    let auth = ms_auth()?.poll_device_code(&session).await?;
    store_account(from_auth(auth)).await
}

#[tauri::command]
pub fn ms_login_cancel(state: tauri::State<'_, AppState>, session_id: String) -> CmdResult<()> {
    state.take_login_session(&session_id)?;
    Ok(())
}
