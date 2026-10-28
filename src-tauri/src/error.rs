use hexomc_lib::HexoError;
use serde::Serialize;

/// `kind` 供前端做分支處理
/// `message` 顯示的訊息。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CmdError {
    pub kind: String,
    pub message: String,
}

impl CmdError {
    pub fn new(kind: &str, message: impl Into<String>) -> Self {
        Self {
            kind: kind.to_string(),
            message: message.into(),
        }
    }

    pub fn launcher(message: impl Into<String>) -> Self {
        Self::new("launcher", message)
    }
}

impl From<HexoError> for CmdError {
    fn from(err: HexoError) -> Self {
        let kind = match &err {
            HexoError::Network(_) => "network",
            HexoError::Io(_) => "io",
            HexoError::Json(_) => "json",
            HexoError::Zip(_) => "zip",
            HexoError::ChecksumMismatch { .. } => "checksumMismatch",
            HexoError::VersionNotFound(_) => "versionNotFound",
            HexoError::JavaNotFound { .. } => "javaNotFound",
            HexoError::AuthError(_) => "authError",
            HexoError::UnsupportedLoader(_) => "unsupportedLoader",
            HexoError::InstanceNotFound(_) => "instanceNotFound",
            HexoError::DownloadFailed { .. } => "downloadFailed",
            HexoError::ProcessorFailed(_) => "processorFailed",
            HexoError::UnsupportedPlatform(_) => {
                return Self::new(
                    "unsupportedPlatform",
                    "ATLauncher 與 FTB 只提供模組包，沒辦法單獨下載模組或材質",
                )
            }
            HexoError::InvalidId(id) => return Self::launcher(format!("ID 不合法: {:?}", id)),
            HexoError::MissingApiKey => {
                return Self::new(
                    "curseforgeKeyMissing",
                    "使用 CurseForge 需要 API 金鑰，請先到「設定」填入",
                )
            }
            HexoError::DownloadBlocked { website, .. } => {
                let mut message =
                    "作者不允許第三方下載這個檔案，請到 CurseForge 網站下載後再自行安裝".to_string();
                if let Some(url) = website {
                    message.push_str(&format!("：{}", url));
                }
                return Self::new("downloadBlocked", message);
            }
            HexoError::Dependency(_) => "modInstall",
            HexoError::Other(_) => "other",
        };
        Self::new(kind, err.to_string())
    }
}

impl From<std::io::Error> for CmdError {
    fn from(err: std::io::Error) -> Self {
        Self::new("io", err.to_string())
    }
}

impl From<serde_json::Error> for CmdError {
    fn from(err: serde_json::Error) -> Self {
        Self::new("json", err.to_string())
    }
}

impl From<reqwest::Error> for CmdError {
    fn from(err: reqwest::Error) -> Self {
        Self::new("network", err.to_string())
    }
}

pub type CmdResult<T> = std::result::Result<T, CmdError>;
