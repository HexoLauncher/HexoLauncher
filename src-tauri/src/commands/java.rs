use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::stream::{self, StreamExt};
use hexomc_lib::{download_java_with_progress, JavaInfo, ProgressFn};
use serde::Serialize;
use tauri::Emitter;

use super::{path_string, EVENT_JAVA_PROGRESS};
use crate::error::{CmdError, CmdResult};
use crate::store::launcher::LauncherConfig;

const JAVA_EXE: &str = if cfg!(windows) { "java.exe" } else { "java" };

const PROBE_CONCURRENCY: usize = 8;

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

const SCAN_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaDto {
    pub path: String,
    pub version: u32,
}

impl From<JavaInfo> for JavaDto {
    fn from(info: JavaInfo) -> Self {
        Self {
            path: path_string(&info.path),
            version: info.version,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaCandidate {
    pub path: String,

    pub version: u32,

    pub full_version: String,
    pub vendor: String,

    pub arch: String,
}

impl JavaCandidate {
    fn is_64bit(&self) -> bool {
        matches!(self.arch.as_str(), "amd64" | "x86_64" | "aarch64" | "arm64")
    }

    fn info(&self) -> JavaInfo {
        JavaInfo {
            path: PathBuf::from(&self.path),
            version: self.version,
        }
    }
}

#[tauri::command]
pub async fn detect_java(required_version: Option<u32>) -> CmdResult<Option<JavaDto>> {
    Ok(resolve_java(required_version.unwrap_or(21), None)
        .await?
        .map(JavaDto::from))
}

#[tauri::command]
pub async fn detect_java_candidates() -> Vec<JavaCandidate> {
    scan_java(true).await.as_ref().clone()
}

#[tauri::command]
pub async fn test_java(path: String) -> CmdResult<JavaDto> {
    let path = PathBuf::from(&path);
    if !path.is_file() {
        return Err(CmdError::launcher(format!(
            "檔案不存在: {}",
            path.display()
        )));
    }
    probe(path)
        .await
        .map(|c| JavaDto::from(c.info()))
        .ok_or_else(|| CmdError::launcher("這個檔案不是可用的 java 執行檔"))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaProgress {
    pub version: u32,
    pub done: usize,
    pub total: usize,
    pub message: String,
}

#[tauri::command]
pub async fn install_java(
    app: tauri::AppHandle,
    base_dir: String,
    version: u32,
) -> CmdResult<JavaDto> {
    let path =
        download_java_with_progress(version, Path::new(&base_dir), java_progress(app, version))
            .await?;

    invalidate_java_scan();

    Ok(probe(path.clone())
        .await
        .map(|c| JavaDto::from(c.info()))
        .unwrap_or(JavaDto {
            path: path_string(&path),
            version,
        }))
}

fn java_progress(app: tauri::AppHandle, version: u32) -> ProgressFn {
    let last: Mutex<(Option<Instant>, String)> = Mutex::new((None, String::new()));
    Arc::new(move |done, total, message| {
        let Ok(mut last) = last.lock() else {
            return;
        };
        let finished = total > 0 && done >= total;
        let recent = last
            .0
            .is_some_and(|at| at.elapsed() < Duration::from_millis(100));
        if last.1 == message && recent && !finished {
            return;
        }
        *last = (Some(Instant::now()), message.to_string());
        let _ = app.emit(
            EVENT_JAVA_PROGRESS,
            JavaProgress {
                version,
                done,
                total,
                message: message.to_string(),
            },
        );
    })
}

#[tauri::command]
pub async fn ensure_java(
    app: tauri::AppHandle,
    base_dir: String,
    required_version: u32,
    allow_download: Option<bool>,
) -> CmdResult<JavaDto> {
    if let Some(info) = resolve_java(required_version, None).await? {
        return Ok(info.into());
    }
    if !allow_download.unwrap_or(false) {
        return Err(java_not_found(required_version));
    }
    install_java(app, base_dir, required_version).await
}

pub async fn resolve_java(
    required_version: u32,
    explicit_path: Option<&str>,
) -> CmdResult<Option<JavaInfo>> {
    if let Some(raw) = explicit_path.filter(|p| !p.is_empty()) {
        let path = PathBuf::from(raw);
        if !path.exists() {
            return Err(CmdError::launcher(format!("指定的 Java 不存在: {}", raw)));
        }

        let info = probe(path.clone())
            .await
            .map(|c| c.info())
            .unwrap_or(JavaInfo {
                path,
                version: required_version,
            });
        if info.version < required_version {
            return Err(CmdError::new(
                "javaTooOld",
                format!(
                    "指定的 Java {} 版本太舊，這個實例至少需要 Java {}：{}",
                    info.version, required_version, raw
                ),
            ));
        }
        return Ok(Some(info));
    }

    let candidates = scan_java(false).await;
    Ok(pick(&candidates, required_version).map(JavaCandidate::info))
}

pub async fn resolve_instance_java(
    required_version: u32,
    instance_path: &str,
    default_path: &str,
) -> CmdResult<Option<JavaInfo>> {
    let instance_path = instance_path.trim();
    if !instance_path.is_empty() {
        return resolve_java(required_version, Some(instance_path)).await;
    }

    let default_path = default_path.trim();
    if !default_path.is_empty() {
        match resolve_java(required_version, Some(default_path)).await {
            Err(e) if e.kind == "javaTooOld" => {}
            other => return other,
        }
    }

    resolve_java(required_version, None).await
}

pub fn java_not_found(required_version: u32) -> CmdError {
    CmdError::new("javaNotFound", format!("找不到 Java {}", required_version))
}

pub async fn pick_java_for_tools() -> Option<JavaInfo> {
    let candidates = scan_java(false).await;
    [21, 17, 8]
        .into_iter()
        .find_map(|version| candidates.iter().find(|c| c.version == version))
        .map(JavaCandidate::info)
}

fn pick(candidates: &[JavaCandidate], required: u32) -> Option<&JavaCandidate> {
    candidates
        .iter()
        .find(|c| c.version == required)
        .or_else(|| {
            candidates
                .iter()
                .filter(|c| c.version > required)
                .min_by(|a, b| {
                    a.version
                        .cmp(&b.version)
                        .then(b.is_64bit().cmp(&a.is_64bit()))
                })
        })
}

static SCAN_CACHE: Mutex<Option<(Instant, Arc<Vec<JavaCandidate>>)>> = Mutex::new(None);

pub fn invalidate_java_scan() {
    if let Ok(mut cache) = SCAN_CACHE.lock() {
        *cache = None;
    }
}

async fn scan_java(refresh: bool) -> Arc<Vec<JavaCandidate>> {
    if !refresh {
        let cached = SCAN_CACHE.lock().ok().and_then(|cache| {
            cache
                .as_ref()
                .filter(|(at, _)| at.elapsed() < SCAN_CACHE_TTL)
                .map(|(_, list)| list.clone())
        });
        if let Some(list) = cached {
            return list;
        }
    }

    let base_dir = LauncherConfig::load()
        .await
        .ok()
        .and_then(|config| config.base_dir().ok());
    let paths = tokio::task::spawn_blocking(move || candidate_paths(base_dir.as_deref()))
        .await
        .unwrap_or_default();

    let mut found: Vec<JavaCandidate> = stream::iter(paths)
        .map(probe)
        .buffer_unordered(PROBE_CONCURRENCY)
        .filter_map(|candidate| async move { candidate })
        .collect()
        .await;

    found.sort_by(|a, b| {
        b.version
            .cmp(&a.version)
            .then(b.is_64bit().cmp(&a.is_64bit()))
            .then(a.path.cmp(&b.path))
    });

    let mut seen = HashSet::new();
    found.retain(|c| seen.insert(path_key(&c.path)));

    let found = Arc::new(found);
    if let Ok(mut cache) = SCAN_CACHE.lock() {
        *cache = Some((Instant::now(), found.clone()));
    }
    found
}

fn path_key(path: &str) -> String {
    if cfg!(windows) {
        path.replace('/', "\\").to_lowercase()
    } else {
        path.to_string()
    }
}

fn candidate_paths(base_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut exes: Vec<PathBuf> = Vec::new();

    let mut roots: Vec<(PathBuf, u32)> = Vec::new();

    if let Some(home) = std::env::var_os("JAVA_HOME") {
        exes.push(PathBuf::from(home).join("bin").join(JAVA_EXE));
    }
    if let Some(path) = std::env::var_os("PATH") {
        exes.extend(std::env::split_paths(&path).map(|dir| dir.join(JAVA_EXE)));
    }

    if let Some(base) = base_dir {
        roots.push((base.join("java"), 3));
    }

    let home = dirs::home_dir();
    // 開發工具下載 JDK 的位置：IntelliJ IDEA、Gradle toolchain、SDKMAN!
    for (relative, depth) in [
        (".jdks", 1),
        (".gradle/jdks", 1),
        (".sdkman/candidates/java", 1),
        // CurseForge App 附的 runtime
        ("curseforge/minecraft/Install/runtime", 4),
    ] {
        if let Some(home) = &home {
            roots.push((home.join(relative), depth));
        }
    }

    let minecraft_dir = if cfg!(target_os = "linux") {
        home.as_ref().map(|h| h.join(".minecraft"))
    } else if cfg!(target_os = "macos") {
        dirs::data_dir().map(|d| d.join("minecraft"))
    } else {
        dirs::data_dir().map(|d| d.join(".minecraft"))
    };
    if let Some(dir) = minecraft_dir {
        roots.push((dir.join("runtime"), 4));
    }
    if let Some(data) = dirs::data_dir() {
        roots.push((data.join("PrismLauncher").join("java"), 3));
    }

    #[cfg(windows)]
    {
        const VENDOR_DIRS: &[&str] = &[
            "Java",
            "Eclipse Adoptium",
            "Eclipse Foundation",
            "AdoptOpenJDK",
            "Zulu",
            "Azul",
            "Microsoft",
            "BellSoft",
            "Amazon Corretto",
            "Semeru",
            "IBM",
            "OpenJDK",
            "GraalVM",
        ];
        let program_dirs = ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"]
            .into_iter()
            .filter_map(std::env::var_os)
            .map(PathBuf::from)
            .chain(dirs::data_local_dir().map(|d| d.join("Programs")));
        for dir in program_dirs {
            for vendor in VENDOR_DIRS {
                roots.push((dir.join(vendor), 2));
            }
        }
        if let Some(local) = dirs::data_local_dir() {
            roots.push((
                local
                    .join("Packages")
                    .join("Microsoft.4297127D64EC6_8wekyb3d8bbwe")
                    .join("LocalCache")
                    .join("Local")
                    .join("runtime"),
                4,
            ));
        }
        if let Some(home) = &home {
            roots.push((home.join("scoop").join("apps"), 2));
        }
        for location in installed_java_locations() {
            roots.push((location, 2));
        }
    }

    #[cfg(target_os = "macos")]
    {
        exes.extend(
            macos_java_homes()
                .into_iter()
                .map(|home| home.join("bin").join(JAVA_EXE)),
        );
        if let Some(home) = &home {
            // IntelliJ IDEA 在 macOS 上下載 JDK 的位置
            roots.push((home.join("Library/Java/JavaVirtualMachines"), 1));
        }
        // Homebrew 的 openjdk 是 keg-only：opt/openjdk@21/libexec/openjdk.jdk/Contents/Home
        roots.push(("/opt/homebrew/opt".into(), 3));
        roots.push(("/usr/local/opt".into(), 3));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // 發行版套件管理員安裝的 JDK
        for dir in ["/usr/lib/jvm", "/usr/lib64/jvm"] {
            roots.push((dir.into(), 1));
        }
    }

    for (root, depth) in roots {
        find_java_executables(&root, depth, &mut exes);
    }

    let mut seen = HashSet::new();
    exes.retain(|path| path.is_file() && seen.insert(path_key(&path_string(path))));
    exes
}

fn find_java_executables(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    for home in [dir.to_path_buf(), dir.join("Contents").join("Home")] {
        let exe = home.join("bin").join(JAVA_EXE);
        if exe.is_file() {
            out.push(exe);
            return;
        }
    }
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_java_executables(&path, depth - 1, out);
        }
    }
}

#[cfg(windows)]
fn installed_java_locations() -> Vec<PathBuf> {
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };
    use winreg::RegKey;

    const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    const KEYWORDS: &[&str] = &[
        "java", "jdk", "jre", "openjdk", "temurin", "zulu", "liberica", "corretto", "graalvm",
    ];

    let mut locations = Vec::new();
    for (hive, view) in [
        (HKEY_LOCAL_MACHINE, KEY_WOW64_64KEY),
        (HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY),
        (HKEY_CURRENT_USER, 0),
    ] {
        let Ok(uninstall) = RegKey::predef(hive).open_subkey_with_flags(UNINSTALL, KEY_READ | view)
        else {
            continue;
        };
        for name in uninstall.enum_keys().flatten() {
            let Ok(entry) = uninstall.open_subkey_with_flags(&name, KEY_READ | view) else {
                continue;
            };
            let label = ["DisplayName", "Publisher"]
                .into_iter()
                .filter_map(|field| entry.get_value::<String, _>(field).ok())
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            if !KEYWORDS.iter().any(|keyword| label.contains(keyword)) {
                continue;
            }
            if let Ok(location) = entry.get_value::<String, _>("InstallLocation") {
                let location = location.trim();
                if !location.is_empty() {
                    locations.push(PathBuf::from(location));
                }
            }
        }
    }
    locations
}

#[cfg(target_os = "macos")]
fn macos_java_homes() -> Vec<PathBuf> {
    let Ok(output) = std::process::Command::new("/usr/libexec/java_home")
        .arg("-V")
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter_map(|line| {
            line.find(" /")
                .map(|at| PathBuf::from(line[at + 1..].trim()))
        })
        .collect()
}

async fn probe(path: PathBuf) -> Option<JavaCandidate> {
    let mut command = tokio::process::Command::new(&path);
    command
        .args(["-XshowSettings:properties", "-version"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = tokio::time::timeout(PROBE_TIMEOUT, command.output())
        .await
        .ok()?
        .ok()?;
    let text = String::from_utf8_lossy(&output.stderr);
    let props = parse_properties(&text);

    let full_version = props
        .get("java.version")
        .cloned()
        .or_else(|| version_from_banner(&text))?;
    let version = props
        .get("java.specification.version")
        .and_then(|v| major_version(v))
        .or_else(|| major_version(&full_version))?;

    let real_path = props
        .get("java.home")
        .map(|home| Path::new(home).join("bin").join(JAVA_EXE))
        .filter(|exe| exe.is_file());

    Some(JavaCandidate {
        path: path_string(&real_path.unwrap_or(path)),
        version,
        full_version,
        vendor: props.get("java.vendor").cloned().unwrap_or_default(),
        arch: props.get("os.arch").cloned().unwrap_or_default(),
    })
}

fn parse_properties(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let (key, value) = line.trim().split_once(" = ")?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

fn version_from_banner(text: &str) -> Option<String> {
    let line = text.lines().find(|line| line.contains(" version \""))?;
    let start = line.find('"')? + 1;
    let end = start + line[start..].find('"')?;
    Some(line[start..end].to_string())
}

fn major_version(version: &str) -> Option<u32> {
    let mut parts = version.split(['.', '_', '-', '+']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn major_versions() {
        assert_eq!(major_version("1.8.0_392"), Some(8));
        assert_eq!(major_version("1.8"), Some(8));
        assert_eq!(major_version("17"), Some(17));
        assert_eq!(major_version("21.0.4"), Some(21));
        assert_eq!(major_version("25-ea"), Some(25));
        assert_eq!(major_version("abc"), None);
    }

    #[test]
    fn reads_settings_output() {
        let text = "Property settings:\n    java.home = C:\\Java\\jdk-21\n    java.library.path = C:\\a\n        C:\\b\n    java.specification.version = 21\n    java.vendor = Eclipse Adoptium\n    java.version = 21.0.4\n    os.arch = amd64\n\nopenjdk version \"21.0.4\" 2024-07-16\n";
        let props = parse_properties(text);
        assert_eq!(props["java.specification.version"], "21");
        assert_eq!(props["java.vendor"], "Eclipse Adoptium");
        assert_eq!(props["os.arch"], "amd64");
        assert_eq!(props["java.library.path"], "C:\\a");
        assert_eq!(version_from_banner(text).as_deref(), Some("21.0.4"));
    }

    #[tokio::test]
    #[ignore]
    async fn scan_this_machine() {
        let started = Instant::now();
        let paths = candidate_paths(None);
        println!("{} 個候選執行檔", paths.len());
        let found = scan_java(true).await;
        for java in found.iter() {
            println!(
                "Java {:<3} {:<10} {:<24} {:<8} {}",
                java.version, java.full_version, java.vendor, java.arch, java.path
            );
        }
        println!("找到 {} 個，花了 {:?}", found.len(), started.elapsed());
    }

    fn candidate(version: u32, arch: &str, path: &str) -> JavaCandidate {
        JavaCandidate {
            path: path.to_string(),
            version,
            full_version: version.to_string(),
            vendor: String::new(),
            arch: arch.to_string(),
        }
    }

    #[test]
    fn picks_exact_then_nearest_newer() {
        let list = vec![
            candidate(25, "amd64", "a"),
            candidate(21, "amd64", "b"),
            candidate(17, "amd64", "c"),
            candidate(17, "x86", "d"),
        ];
        assert_eq!(pick(&list, 21).map(|c| c.path.as_str()), Some("b"));
        assert_eq!(pick(&list, 17).map(|c| c.path.as_str()), Some("c"));
        assert_eq!(pick(&list, 18).map(|c| c.path.as_str()), Some("b"));
        assert_eq!(pick(&list, 8).map(|c| c.path.as_str()), Some("c"));
        assert!(pick(&list, 26).is_none());
    }
}
