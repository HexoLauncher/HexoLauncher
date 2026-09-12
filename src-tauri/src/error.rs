use hexomc_lib::HexoError;
use serde::Serialize;

/// 傳給前端的錯誤。`kind` 供前端做分支處理（例如 javaNotFound 時跳出下載 Java 的提示），
/// `message` 是可直接顯示的訊息。
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

    /// 啟動器自身的錯誤（不是 hexomc-lib 丟出來的）。
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

pub type CmdResult<T> = std::result::Result<T, CmdError>;
