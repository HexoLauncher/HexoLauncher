use hexomc_lib::{poll_device_code, refresh_token, request_device_code, AuthResult};
use serde::Serialize;

use crate::error::{CmdError, CmdResult};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeDto {
    pub session_id: String,
    pub user_code: String,
    pub verification_uri: String,
    pub message: String,

    pub expires_in: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDto {
    pub player_name: String,
    pub uuid: String,
    pub access_token: String,
    pub refresh_token: String,
    pub xuid: String,
    /// access_token 過期時間（unix 秒）
    pub expires_at: u64,
}

impl From<AuthResult> for AccountDto {
    fn from(auth: AuthResult) -> Self {
        Self {
            player_name: auth.player_name,
            uuid: auth.uuid,
            access_token: auth.access_token,
            refresh_token: auth.refresh_token,
            xuid: auth.xuid,
            expires_at: auth.expires_at,
        }
    }
}

#[tauri::command]
pub async fn ms_login_start(state: tauri::State<'_, AppState>) -> CmdResult<DeviceCodeDto> {
    let (info, raw) = request_device_code().await?;
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
) -> CmdResult<AccountDto> {
    let session = state
        .take_login_session(&session_id)?
        .ok_or_else(|| CmdError::new("authError", "登入 session 不存在或已使用，請重新登入"))?;

    let auth = poll_device_code(&session).await?;
    Ok(auth.into())
}

#[tauri::command]
pub fn ms_login_cancel(state: tauri::State<'_, AppState>, session_id: String) -> CmdResult<()> {
    state.take_login_session(&session_id)?;
    Ok(())
}

#[tauri::command]
pub async fn ms_login_refresh(refresh: String) -> CmdResult<AccountDto> {
    Ok(refresh_token(&refresh).await?.into())
}
