use super::*;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use hexomc_lib::{
    match_local, read_mod_info, scan_local_mods, sha256_file, LocalMatch, LocalMod, ModLoader,
};

use crate::commands::instance::load_config;
use crate::commands::validate_file_name;
use crate::state::AppState;

fn problem(message: impl Into<String>) -> CmdError {
    CmdError::new("modInstall", message)
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
    let resolved = browser()
        .await?
        .resolve_dependencies(
            platform,
            &project_id,
            &version_id,
            &config.version_id,
            &loader.mod_loader(),
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
        let file_name = &item.file.file_name;
        validate_file_name(file_name)?;
        if !file_name.to_ascii_lowercase().ends_with(".jar")
            || !names.insert(file_name.to_ascii_lowercase())
        {
            return Err(problem(format!(
                "Invalid or duplicate mod filename: {}",
                file_name
            )));
        }
        let sha1 = item
            .file
            .sha1
            .as_ref()
            .filter(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| problem(format!("{}: no valid download checksum.", item.name)))?;
        let path = stage.0.join(file_name);
        download_file(&DownloadTask::new(&item.file.url, &path).with_sha1(sha1)).await?;
    }
    let prepared = tokio::task::spawn_blocking(move || -> CmdResult<PreparedInstall> {
        let locals = scan_local_mods(&mc.join("mods"))?;
        let mut items = Vec::new();
        let mut digests = Vec::new();
        let mut mod_ids = HashSet::new();
        for (index, item) in resolved.into_iter().enumerate() {
            let path = stage.0.join(&item.file.file_name);
            let info = read_mod_info(&path)
                .filter(|m| m.loader != ModLoader::Unknown && !m.mod_id.is_empty())
                .ok_or_else(|| {
                    problem(format!(
                        "{}: unable to identify the downloaded mod.",
                        item.file.file_name
                    ))
                })?;
            if !mod_ids.insert(info.mod_id.clone()) {
                return Err(problem(format!("Duplicate mod ID: {}", info.mod_id)));
            }
            let hash = sha256_file(&path)?;
            let (installed, conflict) =
                match match_local(&item.file.file_name, &hash, &info.mod_id, &locals) {
                    LocalMatch::NotInstalled => (false, None),
                    LocalMatch::Installed => (true, None),
                    LocalMatch::Conflict(file) => (false, Some(file)),
                };
            items.push(PlanItem {
                name: info.name,
                version: info.version,
                file_name: item.file.file_name,
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
    if scan_local_mods(dir)? != plan.locals {
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
        if sha256_file(&plan.stage.0.join(&item.file_name))? != *hash {
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
            .find(|item| item.sha256 == plan.digests[0])
            .map(|item| item.file_name.clone())
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
            digests.push(sha256_file(&stage.0.join(name)).unwrap());
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
}
