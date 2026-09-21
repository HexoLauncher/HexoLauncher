use hexomc_lib::MicrosoftAuth;
use serde::{Deserialize, Serialize};

use super::{now_secs, read_json, store_file, write_json};
use crate::error::{CmdError, CmdResult};

const FILE: &str = "accounts.json";

const REFRESH_MARGIN: u64 = 120;

/// 打包時用 `HEXO_MS_CLIENT_ID` 帶進來的 Azure 應用程式 (client) ID。
/// hexomc-lib 0.1.5 起不再內建，登入流程要自己給一組。
const MS_CLIENT_ID: Option<&str> = option_env!("HEXO_MS_CLIENT_ID");

pub fn ms_auth() -> CmdResult<MicrosoftAuth> {
    let client_id = MS_CLIENT_ID.unwrap_or_default().trim();
    if client_id.is_empty() {
        return Err(CmdError::new(
            "authError",
            "這個組建沒有內建 Microsoft 應用程式 ID，無法使用 Microsoft 登入",
        ));
    }
    Ok(MicrosoftAuth::new(client_id)?)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountKind {
    Microsoft,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredAccount {
    pub id: String,
    pub kind: AccountKind,
    pub player_name: String,
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub xuid: String,
    #[serde(default)]
    pub expires_at: u64,
}

impl StoredAccount {
    fn expired(&self) -> bool {
        self.kind == AccountKind::Microsoft && self.expires_at <= now_secs() + REFRESH_MARGIN
    }

    fn view(&self, selected: Option<&str>) -> AccountView {
        AccountView {
            id: self.id.clone(),
            kind: self.kind,
            player_name: self.player_name.clone(),
            uuid: self.uuid.clone(),
            expires_at: self.expires_at,
            expired: self.expired(),
            selected: selected == Some(self.id.as_str()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub id: String,
    pub kind: AccountKind,
    pub player_name: String,
    pub uuid: String,
    pub expires_at: u64,
    pub expired: bool,
    pub selected: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountStore {
    pub accounts: Vec<StoredAccount>,
    pub selected: Option<String>,
}

impl AccountStore {
    pub async fn load() -> CmdResult<Self> {
        Ok(read_json(&store_file(FILE)?).await?.unwrap_or_default())
    }

    pub async fn save(&self) -> CmdResult<()> {
        write_json(&store_file(FILE)?, self).await
    }

    pub fn views(&self) -> Vec<AccountView> {
        let selected = self.selected.as_deref();
        self.accounts.iter().map(|a| a.view(selected)).collect()
    }

    fn upsert(&mut self, account: StoredAccount) {
        let id = account.id.clone();
        match self.accounts.iter_mut().find(|a| a.id == id) {
            Some(slot) => *slot = account,
            None => self.accounts.push(account),
        }
        self.selected = Some(id);
    }

    fn take_selected(&self, id: Option<&str>) -> Option<&StoredAccount> {
        let id = id.or(self.selected.as_deref())?;
        self.accounts.iter().find(|a| a.id == id)
    }
}

pub struct ResolvedAccount {
    pub player_name: String,
    pub access_token: Option<String>,
    pub uuid: Option<String>,
    pub xuid: Option<String>,
}

pub async fn resolve(account_id: Option<&str>) -> CmdResult<ResolvedAccount> {
    let mut store = AccountStore::load().await?;

    let account = store
        .take_selected(account_id)
        .ok_or_else(|| CmdError::new("noAccount", "沒有可用的帳號，請先在設定裡新增一個"))?
        .clone();

    if account.kind == AccountKind::Offline {
        return Ok(ResolvedAccount {
            player_name: account.player_name,
            access_token: None,
            uuid: None,
            xuid: None,
        });
    }

    let account = if account.expired() {
        if account.refresh_token.is_empty() {
            return Err(CmdError::new(
                "authError",
                format!("{} 的登入已過期，請重新登入", account.player_name),
            ));
        }
        let refreshed = ms_auth()?.refresh_token(&account.refresh_token).await?;
        let updated = from_auth(refreshed);
        let selected = store.selected.clone();
        store.upsert(updated.clone());
        store.selected = selected.or(Some(updated.id.clone()));
        store.save().await?;
        updated
    } else {
        account
    };

    Ok(ResolvedAccount {
        player_name: account.player_name,
        access_token: Some(account.access_token),
        uuid: Some(account.uuid),
        xuid: Some(account.xuid),
    })
}

pub fn from_auth(auth: hexomc_lib::AuthResult) -> StoredAccount {
    StoredAccount {
        id: format!("msa:{}", auth.uuid),
        kind: AccountKind::Microsoft,
        player_name: auth.player_name,
        uuid: auth.uuid,
        access_token: auth.access_token,
        refresh_token: auth.refresh_token,
        xuid: auth.xuid,
        expires_at: auth.expires_at,
    }
}

pub async fn store_account(account: StoredAccount) -> CmdResult<Vec<AccountView>> {
    let mut store = AccountStore::load().await?;
    store.upsert(account);
    store.save().await?;
    Ok(store.views())
}

#[tauri::command]
pub async fn list_accounts() -> CmdResult<Vec<AccountView>> {
    Ok(AccountStore::load().await?.views())
}

#[tauri::command]
pub async fn add_offline_account(player_name: String) -> CmdResult<Vec<AccountView>> {
    let name = player_name.trim().to_string();
    if name.is_empty() {
        return Err(CmdError::launcher("玩家名稱不可為空"));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || name.len() > 16 {
        return Err(CmdError::launcher(
            "離線帳號名稱只能用英數字與底線，且不超過 16 個字元",
        ));
    }

    store_account(StoredAccount {
        id: format!("offline:{}", name.to_lowercase()),
        kind: AccountKind::Offline,
        player_name: name,
        uuid: String::new(),
        access_token: String::new(),
        refresh_token: String::new(),
        xuid: String::new(),
        expires_at: 0,
    })
    .await
}

#[tauri::command]
pub async fn remove_account(account_id: String) -> CmdResult<Vec<AccountView>> {
    let mut store = AccountStore::load().await?;
    store.accounts.retain(|a| a.id != account_id);
    if store.selected.as_deref() == Some(account_id.as_str()) {
        store.selected = store.accounts.first().map(|a| a.id.clone());
    }
    store.save().await?;
    Ok(store.views())
}

#[tauri::command]
pub async fn select_account(account_id: String) -> CmdResult<Vec<AccountView>> {
    let mut store = AccountStore::load().await?;
    if !store.accounts.iter().any(|a| a.id == account_id) {
        return Err(CmdError::launcher(format!("找不到帳號: {}", account_id)));
    }
    store.selected = Some(account_id);
    store.save().await?;
    Ok(store.views())
}

#[tauri::command]
pub async fn refresh_account(account_id: String) -> CmdResult<Vec<AccountView>> {
    let store = AccountStore::load().await?;
    let account = store
        .accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| CmdError::launcher(format!("找不到帳號: {}", account_id)))?;

    if account.kind == AccountKind::Offline {
        return Err(CmdError::launcher("離線帳號不需要更新權杖"));
    }
    if account.refresh_token.is_empty() {
        return Err(CmdError::new(
            "authError",
            "這個帳號沒有可用的更新權杖，請重新登入",
        ));
    }

    let refreshed = ms_auth()?.refresh_token(&account.refresh_token).await?;
    store_account(from_auth(refreshed)).await
}
