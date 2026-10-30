use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, Row};

use super::{count_mods, Candidate, ForeignInstance, Found};
use crate::commands::{path_string, LoaderKind};
use crate::store::instance_meta::{InstanceSettings, DEFAULT_ICON};

const DATABASE: &str = "app.db";

const FULL_QUERY: &str = "SELECT path, name, game_version, mod_loader, mod_loader_version, \
     icon_path, last_played, submitted_time_played + recent_time_played, override_java_path \
     FROM profiles";
const BASIC_QUERY: &str =
    "SELECT path, name, game_version, mod_loader, mod_loader_version FROM profiles";

pub(super) fn known() -> Vec<Candidate> {
    let Some(data) = dirs::data_dir() else {
        return Vec::new();
    };
    ["ModrinthApp", "com.modrinth.theseus"]
        .into_iter()
        .map(|folder| Candidate {
            kind: "modrinth",
            name: "Modrinth App",
            path: data.join(folder),
        })
        .collect()
}

pub(super) fn data_dir(path: &Path) -> Option<PathBuf> {
    if path.join(DATABASE).is_file() && path.join("profiles").is_dir() {
        return Some(path.to_path_buf());
    }
    let parent = path.parent()?;
    (path.file_name()? == "profiles" && parent.join(DATABASE).is_file())
        .then(|| parent.to_path_buf())
}

#[derive(Debug, Default)]
struct Profile {
    path: String,
    name: String,
    game_version: String,
    mod_loader: String,
    mod_loader_version: String,
    icon_path: Option<String>,
    last_played: i64,
    time_played: i64,
    java_path: Option<String>,
}

pub(super) async fn list(data: &Path) -> Vec<Found> {
    let db = data.join(DATABASE);
    let profiles = tokio::task::spawn_blocking(move || read_profiles(&db).unwrap_or_default())
        .await
        .unwrap_or_default();
    profiles
        .into_iter()
        .filter_map(|p| to_found(data, p))
        .collect()
}

pub(super) async fn read(dir: &Path) -> Option<Found> {
    let data = data_dir(dir.parent()?)?;
    let wanted = dir.file_name()?;
    list(&data)
        .await
        .into_iter()
        .find(|f| f.game_dir.as_deref().and_then(Path::file_name) == Some(wanted))
}

fn read_profiles(db: &Path) -> rusqlite::Result<Vec<Profile>> {
    let conn = Connection::open_with_flags(
        db,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    match query(&conn, FULL_QUERY, true) {
        Ok(profiles) => Ok(profiles),
        Err(_) => query(&conn, BASIC_QUERY, false),
    }
}

fn query(conn: &Connection, sql: &str, full: bool) -> rusqlite::Result<Vec<Profile>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map([], |row| Ok(profile(row, full)))?;
    Ok(rows.flatten().collect())
}

fn profile(row: &Row, full: bool) -> Profile {
    let text = |i: usize| row.get::<_, Option<String>>(i).ok().flatten().unwrap_or_default();
    let number = |i: usize| row.get::<_, Option<i64>>(i).ok().flatten().unwrap_or_default();
    let optional = |i: usize| {
        row.get::<_, Option<String>>(i)
            .ok()
            .flatten()
            .filter(|s| !s.trim().is_empty())
    };
    Profile {
        path: text(0),
        name: text(1),
        game_version: text(2),
        mod_loader: text(3),
        mod_loader_version: text(4),
        icon_path: if full { optional(5) } else { None },
        last_played: if full { number(6) } else { 0 },
        time_played: if full { number(7) } else { 0 },
        java_path: if full { optional(8) } else { None },
    }
}

fn to_found(data: &Path, profile: Profile) -> Option<Found> {
    if profile.path.is_empty() || profile.path.contains(['/', '\\']) || profile.path.contains("..") {
        return None;
    }
    let dir = data.join("profiles").join(&profile.path);
    if !dir.is_dir() {
        return None;
    }

    let mc = profile.game_version;
    let version = profile
        .mod_loader_version
        .strip_prefix(&format!("{}-", mc))
        .unwrap_or(&profile.mod_loader_version)
        .to_string();
    let (loader, loader_version, unsupported_loader) =
        match profile.mod_loader.to_ascii_lowercase().as_str() {
            "forge" => (LoaderKind::Forge, version, None),
            "neoforge" => (LoaderKind::NeoForge, version, None),
            "fabric" => (LoaderKind::Fabric, version, None),
            "vanilla" | "" => (LoaderKind::Vanilla, String::new(), None),
            other => (LoaderKind::Vanilla, String::new(), Some(other.to_string())),
        };
    let last_played = if profile.last_played > 100_000_000_000 {
        profile.last_played / 1000
    } else {
        profile.last_played
    };

    Some(Found {
        info: ForeignInstance {
            path: path_string(&dir),
            name: if profile.name.trim().is_empty() {
                profile.path.clone()
            } else {
                profile.name
            },
            notes: String::new(),
            mc_version: mc,
            loader,
            loader_version,
            mod_count: count_mods(&dir),
            last_played: last_played.max(0) as u64,
            play_time_seconds: profile.time_played.max(0) as u64,
            unsupported_loader,
        },
        game_dir: Some(dir),
        exclude: Vec::new(),
        icon: DEFAULT_ICON.to_string(),
        custom_icon: profile.icon_path.map(PathBuf::from).filter(|p| p.is_file()),
        settings: InstanceSettings::default(),
        java_path: profile.java_path.filter(|p| Path::new(p).is_file()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reads_profiles_from_database() {
        let data = std::env::temp_dir().join(format!("nolauncher-modrinth-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(data.join("profiles/Fabric Pack/mods")).unwrap();
        std::fs::write(data.join("profiles/Fabric Pack/mods/a.jar"), "").unwrap();
        {
            let conn = Connection::open(data.join(DATABASE)).unwrap();
            conn.execute_batch(
                "CREATE TABLE profiles (path TEXT, name TEXT, game_version TEXT, mod_loader TEXT, mod_loader_version TEXT);
                 INSERT INTO profiles VALUES ('Fabric Pack', 'Fabric Pack', '1.21.1', 'fabric', '0.16.5');
                 INSERT INTO profiles VALUES ('Gone', 'Gone', '1.21.1', 'vanilla', NULL);",
            )
            .unwrap();
        }

        assert_eq!(data_dir(&data), Some(data.clone()));
        assert_eq!(data_dir(&data.join("profiles")), Some(data.clone()));
        let found = list(&data).await;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].info.loader, LoaderKind::Fabric);
        assert_eq!(found[0].info.loader_version, "0.16.5");
        assert_eq!(found[0].info.mod_count, 1);
        assert!(read(&data.join("profiles/Fabric Pack")).await.is_some());

        let _ = std::fs::remove_dir_all(&data);
    }
}
