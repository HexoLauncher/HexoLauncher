use hexomc_lib::HexoError;
use serde::Serialize;

use std::collections::BTreeMap;
use std::fmt::Display;

/// `kind` 供前端做分支處理
/// `code` 與 `params` 讓前端用目前的語言組出訊息，`message` 只是沒有對應翻譯時的備援。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CmdError {
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, String>,
}

impl CmdError {
    pub fn new(kind: &str, message: impl Into<String>) -> Self {
        Self {
            kind: kind.to_string(),
            message: message.into(),
            code: None,
            params: BTreeMap::new(),
        }
    }

    pub fn coded(kind: &str, code: &str, params: &[(&str, &dyn Display)]) -> Self {
        Self {
            kind: kind.to_string(),
            message: code.to_string(),
            code: Some(code.to_string()),
            params: params
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        }
    }

    pub fn launcher(message: impl Into<String>) -> Self {
        Self::new("launcher", message)
    }

    pub fn launcher_code(code: &str, params: &[(&str, &dyn Display)]) -> Self {
        Self::coded("launcher", code, params)
    }

    pub fn instance_not_found(name: &str) -> Self {
        Self::coded("instanceNotFound", "instance_not_found", &[("name", &name)])
    }
}

impl From<HexoError> for CmdError {
    fn from(err: HexoError) -> Self {
        let detail = err.to_string();
        match err {
            HexoError::Network(e) => Self::coded("network", "lib_network", &[("detail", &e)]),
            HexoError::Io(e) => Self::coded("io", "lib_io", &[("detail", &e)]),
            HexoError::Json(e) => Self::coded("json", "lib_json", &[("detail", &e)]),
            HexoError::Zip(e) => Self::coded("zip", "lib_zip", &[("detail", &e)]),
            HexoError::ChecksumMismatch { path } => Self::coded(
                "checksumMismatch",
                "lib_checksum_mismatch",
                &[("path", &path)],
            ),
            HexoError::VersionNotFound(v) => Self::coded(
                "versionNotFound",
                "lib_version_not_found",
                &[("version", &v)],
            ),
            HexoError::JavaNotFound { required } => {
                Self::coded("javaNotFound", "java_not_found", &[("version", &required)])
            }
            HexoError::AuthError(_) => Self::coded("authError", "lib_auth", &[("detail", &detail)]),
            HexoError::UnsupportedLoader(name) => Self::coded(
                "unsupportedLoader",
                "lib_unsupported_loader",
                &[("name", &name)],
            ),
            HexoError::InstanceNotFound(name) => Self::instance_not_found(&name),
            HexoError::DownloadFailed { url, .. } => {
                Self::coded("downloadFailed", "lib_download_failed", &[("url", &url)])
            }
            HexoError::ProcessorFailed(_) => Self::coded(
                "processorFailed",
                "lib_processor_failed",
                &[("detail", &detail)],
            ),
            HexoError::UnsupportedPlatform(_) => {
                Self::coded("unsupportedPlatform", "unsupported_platform", &[])
            }
            HexoError::InvalidId(id) => {
                Self::launcher_code("invalid_id", &[("id", &format!("{:?}", id))])
            }
            HexoError::InvalidMirrorUrl(url) => {
                Self::launcher_code("invalid_mirror_url", &[("url", &url)])
            }
            HexoError::MissingApiKey => {
                Self::coded("curseforgeKeyMissing", "curseforge_key_missing", &[])
            }
            HexoError::DownloadBlocked { website, .. } => Self::coded(
                "downloadBlocked",
                "download_blocked",
                &[("url", &website.unwrap_or_default())],
            ),
            HexoError::Dependency(_) => {
                Self::coded("modInstall", "lib_dependency", &[("detail", &detail)])
            }
            HexoError::Other(message) => {
                if let Some(error) = modpack_path_error(&message) {
                    return error;
                }
                Self::coded("other", "lib_other", &[("detail", &message)])
            }
        }
    }
}

fn modpack_path_error(message: &str) -> Option<CmdError> {
    if let Some(path) = message.strip_prefix("linked path in modpack destination: ") {
        return Some(CmdError::coded(
            "instanceLinked",
            "modpack_dest_linked",
            &[("path", &path)],
        ));
    }
    if let Some(path) = message.strip_prefix("unsafe path in modpack: ") {
        return Some(CmdError::coded(
            "unsafeModpack",
            "modpack_unsafe_path",
            &[("path", &path)],
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::CmdError;
    use hexomc_lib::HexoError;

    #[test]
    fn linked_destination_becomes_readable() {
        let err: CmdError =
            HexoError::Other("linked path in modpack destination: C:\\inst\\.minecraft".into())
                .into();
        assert_eq!(err.kind, "instanceLinked");
        assert_eq!(err.code.as_deref(), Some("modpack_dest_linked"));
        assert_eq!(err.params["path"], "C:\\inst\\.minecraft");
    }

    #[test]
    fn other_errors_are_unchanged() {
        let err: CmdError = HexoError::Other("something else".into()).into();
        assert_eq!(err.kind, "other");
        assert_eq!(err.params["detail"], "something else");
    }
}

impl From<std::io::Error> for CmdError {
    fn from(err: std::io::Error) -> Self {
        Self::coded("io", "lib_io", &[("detail", &err)])
    }
}

impl From<serde_json::Error> for CmdError {
    fn from(err: serde_json::Error) -> Self {
        Self::coded("json", "lib_json", &[("detail", &err)])
    }
}

impl From<reqwest::Error> for CmdError {
    fn from(err: reqwest::Error) -> Self {
        Self::coded("network", "lib_network", &[("detail", &err)])
    }
}

pub type CmdResult<T> = std::result::Result<T, CmdError>;
