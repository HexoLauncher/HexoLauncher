use super::*;
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use hexomc_lib::{read_mod_info, ModLoader};
use sha2::{Digest, Sha256};

use crate::commands::instance::load_config;
use crate::commands::validate_file_name;
use crate::state::AppState;

#[derive(Clone, Deserialize)]
struct Dependency {
    project_id: Option<String>,
    version_id: Option<String>,
    dependency_type: String,
}

#[derive(Deserialize)]
struct MrInstallVersion {
    #[serde(flatten)]
    version: MrVersion,
    project_id: String,
    #[serde(default)]
    dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfDependency {
    mod_id: u64,
    relation_type: u32,
}

impl CfDependency {
    fn into_dependency(self) -> Option<Dependency> {
        let kind = match self.relation_type {
            2 => "optional",
            3 => "required",
            5 => "incompatible",
            _ => return None,
        };
        Some(Dependency {
            project_id: Some(self.mod_id.to_string()),
            version_id: None,
            dependency_type: kind.to_string(),
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfInstallFile {
    #[serde(flatten)]
    file: CfFile,
    mod_id: u64,
    #[serde(default)]
    dependencies: Vec<CfDependency>,
}

struct ResolvedMod {
    project: String,
    version: String,
    name: String,
    file: RemoteFile,
    dependencies: Vec<Dependency>,
}

fn problem(message: impl Into<String>) -> CmdError {
    CmdError::new("modInstall", message)
}

fn compatible(mc: &str, loader: LoaderKind, games: &[String], loaders: &[String]) -> CmdResult<()> {
    let name = loader_name(loader).ok_or_else(|| problem("A mod loader is required."))?;
    if !games.iter().any(|v| v == mc) || !loaders.iter().any(|v| v.eq_ignore_ascii_case(name)) {
        return Err(problem(format!(
            "No compatible file for Minecraft {mc} / {name}."
        )));
    }
    Ok(())
}

async fn resolve_version(
    platform: Platform,
    project: &str,
    version: Option<&str>,
    mc: &str,
    loader: LoaderKind,
) -> CmdResult<ResolvedMod> {
    let version = match version {
        Some(id) => id.to_string(),
        None => {
            let mut versions = list_content_versions(
                platform,
                ContentKind::Mod,
                project.to_string(),
                Some(mc.to_string()),
                Some(loader),
            )
            .await?;
            versions.sort_by(|a, b| {
                (b.kind == "release")
                    .cmp(&(a.kind == "release"))
                    .then(b.published.cmp(&a.published))
            });
            versions
                .into_iter()
                .find(|v| v.downloadable)
                .ok_or_else(|| problem(format!("{project}: no downloadable version for {mc}.")))?
                .id
        }
    };
    match platform {
        Platform::Modrinth => {
            let data: MrInstallVersion =
                get_json(http().get(format!("{MODRINTH_API}/version/{}", segment(&version)?)))
                    .await?;
            if !project.is_empty() && data.project_id != project {
                return Err(problem("The requested version belongs to another project."));
            }
            compatible(
                mc,
                loader,
                &data.version.game_versions,
                &data.version.loaders,
            )?;
            let file = data
                .version
                .primary_file()
                .ok_or_else(|| problem("This version has no file."))?;
            Ok(ResolvedMod {
                project: data.project_id,
                version: data.version.id.clone(),
                name: data.version.name.clone(),
                file: RemoteFile {
                    name: file.filename.clone(),
                    url: file.url.clone(),
                    sha1: file.hashes.get("sha1").cloned(),
                },
                dependencies: data.dependencies,
            })
        }
        Platform::CurseForge => {
            #[derive(Deserialize)]
            struct Response {
                data: CfInstallFile,
            }
            let data: Response = get_json(
                curseforge_get(&format!(
                    "/mods/{}/files/{}",
                    numeric_id(project)?,
                    numeric_id(&version)?
                ))
                .await?,
            )
            .await?;
            let data = data.data;
            if data.mod_id.to_string() != project {
                return Err(problem("The requested file belongs to another project."));
            }
            compatible(mc, loader, &data.file.game_versions, &data.file.loaders())?;
            let url = match data.file.download_url.clone() {
                Some(url) => url,
                None => {
                    #[derive(Deserialize)]
                    struct ProjectResponse {
                        data: Project,
                    }
                    #[derive(Deserialize)]
                    struct Project {
                        links: Links,
                    }
                    #[derive(Deserialize)]
                    #[serde(rename_all = "camelCase")]
                    struct Links {
                        website_url: String,
                    }
                    let project_info: ProjectResponse =
                        get_json(curseforge_get(&format!("/mods/{}", numeric_id(project)?)).await?)
                            .await?;
                    return Err(problem(format!(
                        "{project}: third-party download is blocked. Download it manually: {}",
                        project_info.data.links.website_url
                    )));
                }
            };
            Ok(ResolvedMod {
                project: project.to_string(),
                version,
                name: data.file.display_name.clone(),
                file: RemoteFile {
                    name: data.file.file_name.clone(),
                    url,
                    sha1: data.file.sha1(),
                },
                dependencies: data
                    .dependencies
                    .into_iter()
                    .filter_map(CfDependency::into_dependency)
                    .collect(),
            })
        }
        _ => Err(unsupported()),
    }
}

fn add_pin(pins: &mut BTreeMap<String, String>, project: &str, version: &str) -> CmdResult<()> {
    if pins.get(project).is_some_and(|id| id != version) {
        return Err(problem(format!(
            "{project}: dependencies require conflicting versions."
        )));
    }
    pins.insert(project.to_string(), version.to_string());
    Ok(())
}

async fn resolve_graph(
    platform: Platform,
    project: &str,
    version: &str,
    mc: &str,
    loader: LoaderKind,
    optional: bool,
) -> CmdResult<Vec<ResolvedMod>> {
    resolve_graph_with(project, version, optional, |project, version| async move {
        resolve_version(platform, &project, version.as_deref(), mc, loader).await
    })
    .await
}

async fn resolve_graph_with<F, Fut>(
    project: &str,
    version: &str,
    optional: bool,
    mut fetch: F,
) -> CmdResult<Vec<ResolvedMod>>
where
    F: FnMut(String, Option<String>) -> Fut,
    Fut: std::future::Future<Output = CmdResult<ResolvedMod>>,
{
    let mut pins = BTreeMap::from([(project.to_string(), version.to_string())]);
    let mut cache: HashMap<(String, Option<String>), ResolvedMod> = HashMap::new();
    for _ in 0..128 {
        let mut next_pins = BTreeMap::from([(project.to_string(), version.to_string())]);
        let mut queue = VecDeque::from([project.to_string()]);
        let mut visited = HashSet::new();
        let mut selected = Vec::new();
        while let Some(project) = queue.pop_front() {
            if !visited.insert(project.clone()) {
                continue;
            }
            if visited.len() > 256 {
                return Err(problem("The dependency graph exceeds 256 projects."));
            }
            let key = (
                project.clone(),
                next_pins
                    .get(&project)
                    .or_else(|| pins.get(&project))
                    .cloned(),
            );
            if !cache.contains_key(&key) {
                let resolved = fetch(project.clone(), key.1.clone())
                    .await
                    .map_err(|e| problem(format!("{project}: {}", e.message)))?;
                cache.insert(key.clone(), resolved);
            }
            let dependencies = cache[&key].dependencies.clone();
            for dep in dependencies.iter().filter(|d| {
                d.dependency_type == "required" || (optional && d.dependency_type == "optional")
            }) {
                let dep_project = match (&dep.project_id, &dep.version_id) {
                    (Some(id), _) => id.clone(),
                    (None, Some(id)) => fetch(String::new(), Some(id.clone())).await?.project,
                    _ => {
                        return Err(problem(format!(
                            "{project}: a dependency has no downloadable project or version."
                        )))
                    }
                };
                if let Some(id) = &dep.version_id {
                    add_pin(&mut next_pins, &dep_project, id)?;
                }
                queue.push_back(dep_project);
            }
            selected.push(key);
        }
        if next_pins != pins {
            pins = next_pins;
            continue;
        }
        for key in &selected {
            for dep in &cache[key].dependencies {
                if dep.dependency_type == "incompatible"
                    && selected.iter().any(|other| {
                        let node = &cache[other];
                        dep.project_id.as_ref().is_some_and(|p| p == &node.project)
                            && dep.version_id.as_ref().is_none_or(|v| v == &node.version)
                    })
                {
                    return Err(problem(format!(
                        "{}: the selected dependencies include an incompatible mod.",
                        cache[key].name
                    )));
                }
            }
        }
        return Ok(selected
            .into_iter()
            .map(|key| cache.remove(&key).unwrap())
            .collect());
    }
    Err(problem("Dependency versions did not converge."))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LocalMod {
    file: String,
    digest: String,
    mod_id: Option<String>,
}

fn digest(path: &Path) -> CmdResult<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let size = file.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        hash.update(&buffer[..size]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn local_mods(dir: &Path) -> CmdResult<Vec<LocalMod>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut mods = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let file = entry.file_name().to_string_lossy().to_string();
        let lower = file.to_ascii_lowercase();
        if !lower.ends_with(".jar") && !lower.ends_with(".jar.disabled") {
            continue;
        }
        mods.push(LocalMod {
            file,
            digest: digest(&entry.path())?,
            mod_id: read_mod_info(&entry.path())
                .filter(|m| m.loader != ModLoader::Unknown)
                .map(|m| m.mod_id),
        });
    }
    mods.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(mods)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    name: String,
    file_name: String,
    version: String,
    dependency: bool,
    installed: bool,
    conflict: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPlan {
    id: String,
    items: Vec<PlanItem>,
}

struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct PreparedInstall {
    stage: Staging,
    base_dir: String,
    instance: String,
    mc: String,
    loader: LoaderKind,
    items: Vec<PlanItem>,
    digests: Vec<String>,
    locals: Vec<LocalMod>,
    created: Instant,
}

static PLANS: OnceLock<Mutex<HashMap<String, PreparedInstall>>> = OnceLock::new();
static COMMIT: Mutex<()> = Mutex::new(());

fn plans() -> CmdResult<std::sync::MutexGuard<'static, HashMap<String, PreparedInstall>>> {
    PLANS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| problem("Installation plan lock is unavailable."))
}

fn classify(file: &str, hash: &str, mod_id: &str, locals: &[LocalMod]) -> (bool, Option<String>) {
    let matches: Vec<_> = locals
        .iter()
        .filter(|m| {
            m.digest == hash
                || m.file.eq_ignore_ascii_case(file)
                || m.mod_id.as_deref() == Some(mod_id)
        })
        .collect();
    if let Some(conflict) = matches
        .iter()
        .find(|m| m.digest != hash || m.file.to_ascii_lowercase().ends_with(".disabled"))
    {
        return (false, Some(conflict.file.clone()));
    }
    (!matches.is_empty(), None)
}

#[tauri::command]
pub async fn prepare_mod_install(
    base_dir: String,
    instance_name: String,
    platform: Platform,
    project_id: String,
    version_id: String,
    include_optional: bool,
) -> CmdResult<InstallPlan> {
    let config = load_config(Path::new(&base_dir), &instance_name).await?;
    let loader = LoaderKind::from(&config.loader_type);
    let resolved = resolve_graph(
        platform,
        &project_id,
        &version_id,
        &config.version_id,
        loader,
        include_optional,
    )
    .await?;
    let mc = minecraft_dir(Path::new(&base_dir), &instance_name)?;
    let id = uuid::Uuid::new_v4().to_string();
    let stage_path = mc.join(format!(".mod-install-{id}"));
    tokio::fs::create_dir(&stage_path).await?;
    let stage = Staging(stage_path);
    let mut names = HashSet::new();
    for item in &resolved {
        validate_file_name(&item.file.name)?;
        if !item.file.name.to_ascii_lowercase().ends_with(".jar")
            || !names.insert(item.file.name.to_ascii_lowercase())
        {
            return Err(problem(format!(
                "Invalid or duplicate mod filename: {}",
                item.file.name
            )));
        }
        let sha1 = item
            .file
            .sha1
            .as_ref()
            .filter(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| problem(format!("{}: no valid download checksum.", item.name)))?;
        let path = stage.0.join(&item.file.name);
        download_file(&DownloadTask::new(&item.file.url, &path).with_sha1(sha1)).await?;
    }
    let prepared = tokio::task::spawn_blocking(move || -> CmdResult<PreparedInstall> {
        let locals = local_mods(&mc.join("mods"))?;
        let mut items = Vec::new();
        let mut digests = Vec::new();
        let mut mod_ids = HashSet::new();
        for (index, item) in resolved.into_iter().enumerate() {
            let path = stage.0.join(&item.file.name);
            let info = read_mod_info(&path)
                .filter(|m| m.loader != ModLoader::Unknown && !m.mod_id.is_empty())
                .ok_or_else(|| {
                    problem(format!(
                        "{}: unable to identify the downloaded mod.",
                        item.file.name
                    ))
                })?;
            if !mod_ids.insert(info.mod_id.clone()) {
                return Err(problem(format!("Duplicate mod ID: {}", info.mod_id)));
            }
            let hash = digest(&path)?;
            let (installed, conflict) = classify(&item.file.name, &hash, &info.mod_id, &locals);
            items.push(PlanItem {
                name: info.name,
                version: info.version,
                file_name: item.file.name,
                dependency: index != 0,
                installed,
                conflict,
            });
            digests.push(hash);
        }
        Ok(PreparedInstall {
            stage,
            base_dir,
            instance: instance_name,
            mc: config.version_id,
            loader,
            items,
            digests,
            locals,
            created: Instant::now(),
        })
    })
    .await
    .map_err(|e| problem(e.to_string()))??;
    let result = InstallPlan {
        id: id.clone(),
        items: prepared.items.clone(),
    };
    let mut store = plans()?;
    store.retain(|_, plan| plan.created.elapsed() < Duration::from_secs(1800));
    if store.len() >= 8 {
        if let Some(oldest) = store
            .iter()
            .min_by_key(|(_, p)| p.created)
            .map(|(key, _)| key.clone())
        {
            store.remove(&oldest);
        }
    }
    store.insert(id, prepared);
    Ok(result)
}

#[tauri::command]
pub fn discard_mod_install(plan_id: String) -> CmdResult<()> {
    plans()?.remove(&plan_id);
    Ok(())
}

fn commit(plan: &PreparedInstall, dir: &Path) -> CmdResult<InstalledContent> {
    commit_with(plan, dir, |source, dest| std::fs::hard_link(source, dest))
}

fn commit_with(
    plan: &PreparedInstall,
    dir: &Path,
    mut link: impl FnMut(&Path, &Path) -> std::io::Result<()>,
) -> CmdResult<InstalledContent> {
    let _lock = COMMIT
        .lock()
        .map_err(|_| problem("Installation lock is unavailable."))?;
    if local_mods(dir)? != plan.locals {
        return Err(problem(
            "The installed mods changed. Prepare the installation again.",
        ));
    }
    if plan.items.iter().any(|item| item.conflict.is_some()) {
        return Err(problem(
            "Resolve the listed mod conflicts before installing.",
        ));
    }
    for (item, hash) in plan.items.iter().zip(&plan.digests) {
        if digest(&plan.stage.0.join(&item.file_name))? != *hash {
            return Err(problem(
                "A prepared file changed. Prepare the installation again.",
            ));
        }
    }
    std::fs::create_dir_all(dir)?;
    let mut created: Vec<PathBuf> = Vec::new();
    for item in plan.items.iter().filter(|item| !item.installed) {
        let dest = dir.join(&item.file_name);
        if let Err(error) = link(&plan.stage.0.join(&item.file_name), &dest) {
            let mut rollback_errors = Vec::new();
            for path in created {
                if let Err(e) = std::fs::remove_file(&path) {
                    rollback_errors.push(format!("{}: {e}", path_string(&path)));
                }
            }
            return Err(problem(format!(
                "Installation failed: {error}. {}",
                rollback_errors.join("; ")
            )));
        }
        created.push(dest);
    }
    let main = &plan.items[0];
    let file_name = if main.installed {
        plan.locals
            .iter()
            .find(|item| item.digest == plan.digests[0])
            .map(|item| item.file.clone())
            .unwrap_or_else(|| main.file_name.clone())
    } else {
        main.file_name.clone()
    };
    Ok(InstalledContent {
        path: path_string(&dir.join(&file_name)),
        file_name,
    })
}

#[tauri::command]
pub async fn commit_mod_install(
    state: tauri::State<'_, AppState>,
    plan_id: String,
) -> CmdResult<InstalledContent> {
    let plan = plans()?
        .remove(&plan_id)
        .ok_or_else(|| problem("Installation plan expired. Prepare it again."))?;
    if plan.created.elapsed() >= Duration::from_secs(1800) {
        return Err(problem("Installation plan expired. Prepare it again."));
    }
    if state.is_running(&plan.instance)? {
        return Err(problem("Close the game before installing mods."));
    }
    let config = load_config(Path::new(&plan.base_dir), &plan.instance).await?;
    if config.version_id != plan.mc || LoaderKind::from(&config.loader_type) != plan.loader {
        return Err(problem(
            "The instance version changed. Prepare the installation again.",
        ));
    }
    let dir = minecraft_dir(Path::new(&plan.base_dir), &plan.instance)?.join("mods");
    tokio::task::spawn_blocking(move || commit(&plan, &dir))
        .await
        .map_err(|e| problem(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dep(project: &str, version: Option<&str>, kind: &str) -> Dependency {
        Dependency {
            project_id: Some(project.to_string()),
            version_id: version.map(str::to_string),
            dependency_type: kind.to_string(),
        }
    }

    fn node(project: &str, version: &str, dependencies: Vec<Dependency>) -> ResolvedMod {
        ResolvedMod {
            project: project.to_string(),
            version: version.to_string(),
            name: project.to_string(),
            dependencies,
            file: RemoteFile {
                url: String::new(),
                name: format!("{project}.jar"),
                sha1: None,
            },
        }
    }

    #[tokio::test]
    async fn dependency_cycles_are_deduplicated_and_optional_mods_are_opt_in() {
        for optional in [false, true] {
            let result = resolve_graph_with("main", "1", optional, |project, _| {
                std::future::ready(Ok(match project.as_str() {
                    "main" => node(
                        "main",
                        "1",
                        vec![
                            dep("lib", None, "required"),
                            dep("extra", None, "optional"),
                            dep("embedded", None, "embedded"),
                        ],
                    ),
                    "lib" => node("lib", "2", vec![dep("main", Some("1"), "required")]),
                    "extra" => node("extra", "1", vec![dep("lib", None, "required")]),
                    _ => panic!("Unexpected dependency: {project}"),
                }))
            })
            .await
            .unwrap();
            assert_eq!(result.len(), if optional { 3 } else { 2 });
            assert_eq!(
                result.iter().filter(|item| item.project == "lib").count(),
                1
            );
        }
    }

    #[tokio::test]
    async fn a_later_exact_dependency_replaces_an_unpinned_choice() {
        let result = resolve_graph_with("main", "1", false, |project, version| {
            std::future::ready(Ok(match project.as_str() {
                "main" => node(
                    "main",
                    "1",
                    vec![dep("lib", None, "required"), dep("other", None, "required")],
                ),
                "lib" => node("lib", version.as_deref().unwrap_or("latest"), vec![]),
                "other" => node("other", "1", vec![dep("lib", Some("pinned"), "required")]),
                _ => panic!("Unexpected dependency"),
            }))
        })
        .await
        .unwrap();
        assert_eq!(
            result
                .iter()
                .find(|item| item.project == "lib")
                .unwrap()
                .version,
            "pinned"
        );
    }

    #[tokio::test]
    async fn conflicting_pins_and_incompatible_projects_are_rejected() {
        let conflict = resolve_graph_with("main", "1", false, |_, _| {
            std::future::ready(Ok(node(
                "main",
                "1",
                vec![
                    dep("lib", Some("1"), "required"),
                    dep("lib", Some("2"), "required"),
                ],
            )))
        })
        .await;
        assert!(conflict.is_err());
        let incompatible = resolve_graph_with("main", "1", false, |project, _| {
            std::future::ready(Ok(if project == "main" {
                node(
                    "main",
                    "1",
                    vec![
                        dep("lib", None, "required"),
                        dep("lib", None, "incompatible"),
                    ],
                )
            } else {
                node("lib", "1", vec![])
            }))
        })
        .await;
        assert!(incompatible.is_err());
    }

    #[test]
    fn installed_disabled_and_conflicting_mods_are_distinguished() {
        let mut locals = vec![LocalMod {
            file: "renamed.jar".into(),
            digest: "same".into(),
            mod_id: Some("lib".into()),
        }];
        assert_eq!(classify("lib.jar", "same", "lib", &locals), (true, None));
        assert_eq!(
            classify("lib.jar", "different", "lib", &locals),
            (false, Some("renamed.jar".into()))
        );
        locals[0].file = "renamed.jar.disabled".into();
        assert_eq!(
            classify("lib.jar", "same", "lib", &locals),
            (false, Some("renamed.jar.disabled".into()))
        );
        assert_eq!(classify("new.jar", "new", "new", &locals), (false, None));
    }

    #[test]
    fn both_game_version_and_loader_must_match() {
        let games = vec!["1.21.1".to_string()];
        let loaders = vec!["Fabric".to_string()];
        assert!(compatible("1.21.1", LoaderKind::Fabric, &games, &loaders).is_ok());
        assert!(compatible("1.21", LoaderKind::Fabric, &games, &loaders).is_err());
        assert!(compatible("1.21.1", LoaderKind::Forge, &games, &loaders).is_err());
        assert!(compatible("1.21.1", LoaderKind::Vanilla, &games, &loaders).is_err());
    }

    #[test]
    fn curseforge_dependencies_preserve_relation_types() {
        let data: CfInstallFile = serde_json::from_value(serde_json::json!({
            "id": 10,
            "modId": 100,
            "gameVersions": ["1.21.1", "Fabric"],
            "dependencies": [
                { "modId": 1, "relationType": 1 },
                { "modId": 2, "relationType": 2 },
                { "modId": 3, "relationType": 3 },
                { "modId": 5, "relationType": 5 }
            ]
        }))
        .unwrap();
        assert_eq!(data.mod_id, 100);
        assert!(compatible(
            "1.21.1",
            LoaderKind::Fabric,
            &data.file.game_versions,
            &data.file.loaders()
        )
        .is_ok());
        let deps: Vec<_> = data
            .dependencies
            .into_iter()
            .filter_map(CfDependency::into_dependency)
            .collect();
        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0].dependency_type, "optional");
        assert_eq!(deps[1].dependency_type, "required");
        assert_eq!(deps[1].project_id.as_deref(), Some("3"));
        assert_eq!(deps[2].dependency_type, "incompatible");
    }

    #[tokio::test]
    async fn exact_dependencies_do_not_require_a_latest_version() {
        let result = resolve_graph_with("main", "1", false, |project, version| {
            std::future::ready(match project.as_str() {
                "main" => Ok(node(
                    "main",
                    "1",
                    vec![dep("lib", Some("archived"), "required")],
                )),
                "lib" if version.as_deref() == Some("archived") => {
                    Ok(node("lib", "archived", vec![]))
                }
                _ => Err(problem("No latest version available")),
            })
        })
        .await
        .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[1].version, "archived");
    }

    fn prepared() -> (Staging, PreparedInstall, PathBuf) {
        let root = Staging(
            std::env::temp_dir().join(format!("hexo-install-test-{}", uuid::Uuid::new_v4())),
        );
        std::fs::create_dir(&root.0).unwrap();
        let stage = Staging(root.0.join("stage"));
        std::fs::create_dir(&stage.0).unwrap();
        let mut items = Vec::new();
        let mut digests = Vec::new();
        for name in ["main.jar", "lib.jar"] {
            std::fs::write(stage.0.join(name), name).unwrap();
            digests.push(digest(&stage.0.join(name)).unwrap());
            items.push(PlanItem {
                name: name.into(),
                file_name: name.into(),
                version: "1".into(),
                dependency: name != "main.jar",
                installed: false,
                conflict: None,
            });
        }
        let dir = root.0.join("mods");
        let plan = PreparedInstall {
            stage,
            base_dir: String::new(),
            instance: String::new(),
            mc: String::new(),
            loader: LoaderKind::Fabric,
            items,
            digests,
            locals: vec![],
            created: Instant::now(),
        };
        (root, plan, dir)
    }

    #[test]
    fn commit_installs_all_files_and_rejects_a_changed_mods_folder() {
        let (_root, plan, dir) = prepared();
        commit(&plan, &dir).unwrap();
        assert_eq!(std::fs::read(dir.join("main.jar")).unwrap(), b"main.jar");
        assert_eq!(std::fs::read(dir.join("lib.jar")).unwrap(), b"lib.jar");
        assert!(commit(&plan, &dir).is_err());
    }

    #[test]
    fn changed_staged_files_never_reach_the_mods_folder() {
        let (_root, plan, dir) = prepared();
        std::fs::write(plan.stage.0.join("lib.jar"), b"changed").unwrap();
        assert!(commit(&plan, &dir).is_err());
        assert!(!dir.exists());
    }

    #[test]
    fn failed_commit_rolls_back_only_new_files() {
        let (_root, plan, dir) = prepared();
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("keep.txt"), b"keep").unwrap();
        let result = commit_with(&plan, &dir, |source, dest| {
            if dest.ends_with("lib.jar") {
                std::fs::write(dest, b"external change")?;
            }
            std::fs::hard_link(source, dest)
        });
        assert!(result.is_err());
        assert!(!dir.join("main.jar").exists());
        assert_eq!(
            std::fs::read(dir.join("lib.jar")).unwrap(),
            b"external change"
        );
        assert_eq!(std::fs::read(dir.join("keep.txt")).unwrap(), b"keep");
    }

    #[tokio::test]
    #[ignore]
    async fn modrinth_live_dependency_smoke() {
        let root = resolve_version(
            Platform::Modrinth,
            "YL57xq9U",
            None,
            "1.21.1",
            LoaderKind::Fabric,
        )
        .await
        .unwrap();
        let graph = resolve_graph(
            Platform::Modrinth,
            &root.project,
            &root.version,
            "1.21.1",
            LoaderKind::Fabric,
            false,
        )
        .await
        .unwrap();
        assert!(graph.len() > 1);
        assert!(graph.iter().any(|item| item.project == "AANobbMI"));
        let stage = Staging(
            std::env::temp_dir().join(format!("hexo-download-test-{}", uuid::Uuid::new_v4())),
        );
        std::fs::create_dir(&stage.0).unwrap();
        for item in graph {
            let path = stage.0.join(&item.file.name);
            download_file(
                &DownloadTask::new(&item.file.url, &path)
                    .with_sha1(item.file.sha1.as_ref().unwrap()),
            )
            .await
            .unwrap();
            let info = read_mod_info(&path).unwrap();
            assert_ne!(info.loader, ModLoader::Unknown);
            assert!(!info.mod_id.is_empty());
        }
    }
}
