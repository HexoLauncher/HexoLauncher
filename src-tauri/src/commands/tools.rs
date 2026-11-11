use mcmap_util::seed::structure::{structure_set, vanilla_structure_sets, Placement};
use mcmap_util::seed::{parse_seed, GenVersion};
use mcmap_util::tech::{
    find_slime_afk_spots_tracked, find_structure_clusters_tracked, Cancelled, ClusterSearch, SearchArea,
    SlimeSearch, SlimeSpot, SpawnRing, StructureCluster, Task,
};
use mcmap_util::world::{ChunkStat, PruneFilter, PruneReport};
use mcmap_util::World;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use tauri::ipc::Response;
use tauri::{AppHandle, Emitter};

use super::assets::world_dir;
use crate::error::{CmdError, CmdResult};

/// 史萊姆搜尋最大半徑（格）；區塊表大小隨半徑平方成長。
const MAX_SLIME_RADIUS: i32 = 50_000;
/// 結構搜尋最大半徑（格），也就是世界邊界。
const MAX_CLUSTER_RADIUS: i32 = 30_000_000;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerPos {
    pub uuid: Option<String>,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub dimension: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldInfo {
    pub name: String,
    /// 種子字串；level.dat 沒有種子時為 null。
    pub seed: Option<String>,
    pub data_version: Option<i32>,
    /// 生成規則版本（`1.21.4`、`26` 等）；1.18 以前或讀不到時為 null。
    pub version: Option<&'static str>,
    /// 存檔裡有區域檔的維度 id。
    pub dimensions: Vec<String>,
    /// 遊戲正在使用這個世界（session.lock 被佔用）。
    pub locked: bool,
    pub players: Vec<PlayerPos>,
    pub spawn: Option<[i32; 3]>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditResult {
    pub report: PruneReport,
    /// 編輯前建立的備份檔路徑。
    pub backup: Option<String>,
}

fn seed_of(text: &str) -> CmdResult<i64> {
    parse_seed(text).ok_or_else(|| CmdError::launcher("請輸入種子"))
}

fn version_of(text: &str) -> CmdResult<GenVersion> {
    GenVersion::parse(text)
        .ok_or_else(|| CmdError::launcher(format!("不支援的版本: {text}（需要 1.18 以上）")))
}

/// 搜尋進度事件：`{ taskId, done, total }`，執行中每 150ms 一次，結束時再送一次。
pub const EVENT_TOOLS_PROGRESS: &str = "nolauncher://tools-progress";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    task_id: String,
    done: u64,
    total: u64,
}

/// 執行中的搜尋，給 `tools_cancel` 找。
fn running_tasks() -> &'static Mutex<HashMap<String, Arc<Task>>> {
    static TASKS: OnceLock<Mutex<HashMap<String, Arc<Task>>>> = OnceLock::new();
    TASKS.get_or_init(Default::default)
}

/// 進度回報：`(done, total)`。
type Reporter = Arc<dyn Fn(u64, u64) + Send + Sync>;

fn event_reporter(app: AppHandle, task_id: String) -> Reporter {
    Arc::new(move |done, total| {
        let _ = app.emit(EVENT_TOOLS_PROGRESS, ProgressEvent { task_id: task_id.clone(), done, total });
    })
}

/// 在背景執行緒跑一個可取消的搜尋，期間每 150ms 回報一次進度，結束時再報一次。
async fn tracked<T: Send + 'static>(
    task_id: String,
    report: Reporter,
    work: impl FnOnce(&Task) -> Result<T, Cancelled> + Send + 'static,
) -> CmdResult<T> {
    let task = Arc::new(Task::new());
    running_tasks()
        .lock()
        .map_err(|e| CmdError::launcher(e.to_string()))?
        .insert(task_id.clone(), task.clone());

    let finished = Arc::new(AtomicBool::new(false));
    let emit = {
        let task = task.clone();
        move || {
            let (done, total) = task.progress();
            report(done, total);
        }
    };
    let reporter = {
        let (finished, emit) = (finished.clone(), emit.clone());
        tauri::async_runtime::spawn(async move {
            while !finished.load(Ordering::Relaxed) {
                emit();
                tokio::time::sleep(Duration::from_millis(150)).await;
            }
        })
    };

    let worker = task.clone();
    let result = tokio::task::spawn_blocking(move || work(&worker)).await;
    finished.store(true, Ordering::Relaxed);
    let _ = reporter.await;
    emit();
    if let Ok(mut tasks) = running_tasks().lock() {
        tasks.remove(&task_id);
    }
    match result {
        Err(e) => Err(CmdError::launcher(e.to_string())),
        Ok(Err(Cancelled)) => Err(CmdError::new("cancelled", "已取消搜尋")),
        Ok(Ok(value)) => Ok(value),
    }
}

/// 取消執行中的搜尋；找不到（已結束）時什麼也不做。
#[tauri::command]
pub fn tools_cancel(task_id: String) {
    if let Ok(tasks) = running_tasks().lock() {
        if let Some(task) = tasks.get(&task_id) {
            task.cancel();
        }
    }
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> CmdResult<T> + Send + 'static) -> CmdResult<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| CmdError::launcher(e.to_string()))?
}

fn open_world(dir: &std::path::Path) -> CmdResult<World> {
    World::open(dir).map_err(|e| CmdError::launcher(format!("讀取世界失敗: {e}")))
}

/// 只接受世界裡實際存在的維度，避免維度 id 被拿來組出存檔外的路徑。
fn dimension_of(world: &World, id: &str) -> CmdResult<mcmap_util::world::DimensionStorage> {
    let id = if id.contains(':') { id.to_string() } else { format!("minecraft:{id}") };
    let known = ["minecraft:overworld", "minecraft:the_nether", "minecraft:the_end"];
    if known.contains(&id.as_str()) || world.dimensions().iter().any(|d| d.id == id) {
        Ok(world.dimension(&id))
    } else {
        Err(CmdError::launcher(format!("未知的維度: {id}")))
    }
}

fn lib_err(e: mcmap_util::Error) -> CmdError {
    CmdError::launcher(e.to_string())
}

/// 世界名稱、種子、維度、玩家位置等地圖需要的資訊。
#[tauri::command]
pub async fn tools_world_info(base_dir: String, instance_name: String, folder: String) -> CmdResult<WorldInfo> {
    let dir = world_dir(&base_dir, &instance_name, &folder)?;
    blocking(move || {
        let world = open_world(&dir)?;
        let summary = world.level.summary();
        Ok(WorldInfo {
            name: summary.level_name.clone(),
            seed: world.level.seed().map(|s| s.to_string()),
            data_version: summary.data_version,
            version: world.gen_version().map(GenVersion::name),
            dimensions: world
                .dimensions()
                .into_iter()
                .filter(|d| !d.regions().is_empty())
                .map(|d| d.id)
                .collect(),
            locked: world.is_locked(),
            players: world
                .players()
                .into_iter()
                .map(|p| PlayerPos {
                    uuid: p.uuid,
                    x: p.pos[0],
                    y: p.pos[1],
                    z: p.pos[2],
                    dimension: p.dimension,
                })
                .collect(),
            spawn: summary.spawn.map(|s| [s.x, s.y, s.z]),
        })
    })
    .await
}

/// 某維度所有區域檔的座標。
#[tauri::command]
pub async fn tools_world_regions(
    base_dir: String,
    instance_name: String,
    folder: String,
    dimension: String,
) -> CmdResult<Vec<(i32, i32)>> {
    let dir = world_dir(&base_dir, &instance_name, &folder)?;
    blocking(move || {
        let world = open_world(&dir)?;
        Ok(dimension_of(&world, &dimension)?.regions())
    })
    .await
}

/// 一個區域（512×512 格）的俯視圖，RGBA 原始位元組；沒有區域檔時回傳空內容。
#[tauri::command]
pub async fn tools_render_region(
    base_dir: String,
    instance_name: String,
    folder: String,
    dimension: String,
    rx: i32,
    rz: i32,
) -> CmdResult<Response> {
    let dir = world_dir(&base_dir, &instance_name, &folder)?;
    blocking(move || {
        let world = open_world(&dir)?;
        let dim = dimension_of(&world, &dimension)?;
        if !dim.regions().contains(&(rx, rz)) {
            return Ok(Response::new(Vec::new()));
        }
        Ok(Response::new(dim.render_region(rx, rz).map_err(lib_err)?.rgba))
    })
    .await
}

/// 一個區域內每個區塊的生成狀態、居住時間與存檔時間。
#[tauri::command]
pub async fn tools_region_chunks(
    base_dir: String,
    instance_name: String,
    folder: String,
    dimension: String,
    rx: i32,
    rz: i32,
) -> CmdResult<Vec<ChunkStat>> {
    let dir = world_dir(&base_dir, &instance_name, &folder)?;
    blocking(move || {
        let world = open_world(&dir)?;
        dimension_of(&world, &dimension)?.chunk_stats(rx, rz).map_err(lib_err)
    })
    .await
}

/// 編輯前的準備：確認遊戲沒有開著這個世界，需要時先備份到 `.minecraft/backups/`。
fn prepare_edit(world: &World, dir: &std::path::Path, backup: bool) -> CmdResult<Option<String>> {
    if world.is_locked() {
        return Err(CmdError::launcher("遊戲正在使用這個世界，請先關閉遊戲或回到標題畫面"));
    }
    if !backup {
        return Ok(None);
    }
    let backups = dir
        .parent()
        .and_then(|saves| saves.parent())
        .map(|mc| mc.join("backups"))
        .ok_or_else(|| CmdError::launcher("找不到備份資料夾"))?;
    std::fs::create_dir_all(&backups)?;
    let folder = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let dest = backups.join(format!("{}_{}_tools.zip", timestamp(), folder));
    world.backup(&dest).map_err(|e| CmdError::launcher(format!("備份失敗: {e}")))?;
    Ok(Some(dest.to_string_lossy().to_string()))
}

/// `YYYY-MM-DD_HH-MM-SS`（UTC），和遊戲備份檔名的格式相同。
fn timestamp() -> String {
    let secs = now_secs();
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Howard Hinnant 的 civil_from_days。
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}_{:02}-{:02}-{:02}",
        rem / 3600,
        rem / 60 % 60,
        rem % 60
    )
}

/// 重新生成指定區塊：刪除存檔裡的區塊、實體與 POI 資料，遊戲下次載入時會依種子重新生成。
#[tauri::command]
pub async fn tools_regenerate_chunks(
    base_dir: String,
    instance_name: String,
    folder: String,
    dimension: String,
    chunks: Vec<(i32, i32)>,
    backup: bool,
) -> CmdResult<EditResult> {
    if chunks.is_empty() {
        return Err(CmdError::launcher("沒有選取任何區塊"));
    }
    let dir = world_dir(&base_dir, &instance_name, &folder)?;
    blocking(move || {
        let world = open_world(&dir)?;
        let dim = dimension_of(&world, &dimension)?;
        let backup = prepare_edit(&world, &dir, backup)?;
        let report = world.delete_chunks(&dim.id, &chunks).map_err(lib_err)?;
        Ok(EditResult { report, backup })
    })
    .await
}

/// 依條件清理（重新生成）整個維度的區塊；`dry_run` 只回報數量。
///
/// `max_inhabited_ticks`：居住時間低於這個刻數；`unloaded_days`：超過這麼多天沒有載入
/// （區域檔記錄的最後存檔時間）。兩個都給時要同時符合。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn tools_prune_chunks(
    base_dir: String,
    instance_name: String,
    folder: String,
    dimension: String,
    max_inhabited_ticks: Option<i64>,
    unloaded_days: Option<f64>,
    dry_run: bool,
    backup: bool,
) -> CmdResult<EditResult> {
    if max_inhabited_ticks.is_none() && unloaded_days.is_none() {
        return Err(CmdError::launcher("請至少選擇一個清理條件"));
    }
    let filter = PruneFilter {
        max_inhabited_ticks,
        saved_before: unloaded_days.map(|days| saved_before(days, now_secs())),
    };
    let dir = world_dir(&base_dir, &instance_name, &folder)?;
    blocking(move || {
        let world = open_world(&dir)?;
        let dim = dimension_of(&world, &dimension)?;
        let backup = if dry_run { None } else { prepare_edit(&world, &dir, backup)? };
        let report = world.prune(&dim.id, &filter, dry_run).map_err(lib_err)?;
        Ok(EditResult { report, backup })
    })
    .await
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 「超過 `days` 天沒有載入」對應的存檔時間上限（Unix 秒）。
fn saved_before(days: f64, now: i64) -> u32 {
    (now - (days.max(0.0) * 86_400.0) as i64).clamp(0, u32::MAX as i64) as u32
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpreadSet {
    pub name: &'static str,
    pub dimension: &'static str,
    pub spacing: i32,
}

/// 可以拿來找多聯的結構集合（random spread 類型）。
#[tauri::command]
pub fn tools_structure_sets(version: String) -> CmdResult<Vec<SpreadSet>> {
    let version = version_of(&version)?;
    Ok(vanilla_structure_sets(version)
        .into_iter()
        .filter_map(|s| match s.placement {
            Placement::RandomSpread { spacing, .. } if spacing >= 8 => Some(SpreadSet {
                name: s.name,
                dimension: s.dimension().id(),
                spacing,
            }),
            _ => None,
        })
        .collect())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlimeQuery {
    pub seed: String,
    pub center_x: i32,
    pub center_z: i32,
    pub radius: i32,
    pub step: i32,
    pub limit: usize,
    pub min_separation: f64,
    pub ring_min: f64,
    pub ring_max: f64,
}

#[tauri::command]
pub async fn tools_slime_spots(app: AppHandle, query: SlimeQuery, task_id: String) -> CmdResult<Vec<SlimeSpot>> {
    let report = event_reporter(app, task_id.clone());
    slime_spots(query, task_id, report).await
}

async fn slime_spots(query: SlimeQuery, task_id: String, report: Reporter) -> CmdResult<Vec<SlimeSpot>> {
    let seed = seed_of(&query.seed)?;
    if !(1..=MAX_SLIME_RADIUS).contains(&query.radius) {
        return Err(CmdError::launcher(format!("搜尋半徑需在 1 到 {MAX_SLIME_RADIUS} 格之間")));
    }
    if !(query.ring_min >= 0.0 && query.ring_max > query.ring_min && query.ring_max <= 256.0) {
        return Err(CmdError::launcher("生怪範圍不合理"));
    }
    let opts = SlimeSearch {
        area: SearchArea::around(query.center_x, query.center_z, query.radius),
        ring: SpawnRing { min: query.ring_min, max: query.ring_max },
        step: query.step.clamp(1, 16),
        limit: query.limit.clamp(1, 100),
        min_separation: query.min_separation.max(0.0),
    };
    tracked(task_id, report, move |task| find_slime_afk_spots_tracked(seed, &opts, task)).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterQuery {
    pub seed: String,
    pub version: String,
    pub set: String,
    pub center_x: i32,
    pub center_z: i32,
    pub radius: i32,
    pub min_count: usize,
    pub ring_max: f64,
    /// 覆蓋率模式下每座結構最少要被涵蓋的比例（0–1）；null 表示整座結構都要在範圍內。
    pub min_coverage: Option<f64>,
    pub check_biomes: bool,
}

#[tauri::command]
pub async fn tools_structure_clusters(
    app: AppHandle,
    query: ClusterQuery,
    task_id: String,
) -> CmdResult<Vec<StructureCluster>> {
    let report = event_reporter(app, task_id.clone());
    structure_clusters(query, task_id, report).await
}

async fn structure_clusters(
    query: ClusterQuery,
    task_id: String,
    report: Reporter,
) -> CmdResult<Vec<StructureCluster>> {
    let seed = seed_of(&query.seed)?;
    let version = version_of(&query.version)?;
    let set = structure_set(version, &query.set)
        .ok_or_else(|| CmdError::launcher(format!("未知的結構: {}", query.set)))?;
    if !(1..=MAX_CLUSTER_RADIUS).contains(&query.radius) {
        return Err(CmdError::launcher(format!("搜尋半徑需在 1 到 {MAX_CLUSTER_RADIUS} 格之間")));
    }
    if !(16.0..=256.0).contains(&query.ring_max) {
        return Err(CmdError::launcher("掛機範圍需在 16 到 256 格之間"));
    }
    let mut opts = ClusterSearch {
        min_count: query.min_count.clamp(2, 4),
        ring: SpawnRing { min: 24.0, max: query.ring_max },
        ..ClusterSearch::quads(SearchArea::around(query.center_x, query.center_z, query.radius))
    };
    if let Some(min) = query.min_coverage {
        opts = opts.by_coverage(min);
    }
    tracked(task_id, report, move |task| {
        let biomes = query.check_biomes.then(|| {
            mcmap_util::biome::biome_source(
                seed,
                set.dimension(),
                version,
                mcmap_util::biome::WorldPreset::Normal,
            )
        });
        let mut found =
            find_structure_clusters_tracked(version, seed, &set, biomes.as_deref(), &opts, task)?;
        found.truncate(200);
        Ok(found)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run<T>(f: impl std::future::Future<Output = T>) -> T {
        tauri::async_runtime::block_on(f)
    }

    fn quiet() -> Reporter {
        Arc::new(|_, _| {})
    }

    fn search_slime(query: SlimeQuery) -> impl std::future::Future<Output = CmdResult<Vec<SlimeSpot>>> {
        slime_spots(query, "test".into(), quiet())
    }

    fn search_clusters(query: ClusterQuery) -> impl std::future::Future<Output = CmdResult<Vec<StructureCluster>>> {
        let id = format!("test-{}", query.set);
        structure_clusters(query, id, quiet())
    }

    #[test]
    fn reports_progress_and_cancels() {
        let reports = Arc::new(Mutex::new(Vec::<(u64, u64)>::new()));
        let sink: Reporter = {
            let reports = reports.clone();
            Arc::new(move |d, t| reports.lock().unwrap().push((d, t)))
        };
        let query = ClusterQuery {
            seed: "12345".into(),
            version: "26".into(),
            set: "swamp_huts".into(),
            center_x: 0,
            center_z: 0,
            radius: MAX_CLUSTER_RADIUS,
            min_count: 4,
            ring_max: 128.0,
            min_coverage: None,
            check_biomes: false,
        };
        let started = std::time::Instant::now();
        let result = run(async {
            let search = tauri::async_runtime::spawn(structure_clusters(query, "big".into(), sink));
            // Wait until the search is registered and has made progress, then cancel it.
            while reports.lock().unwrap().iter().all(|&(d, _)| d == 0) {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            tools_cancel("big".into());
            search.await.unwrap()
        });
        let err = result.unwrap_err();
        assert_eq!(err.kind, "cancelled");
        assert!(started.elapsed() < Duration::from_secs(30), "{:?}", started.elapsed());
        let reports = reports.lock().unwrap();
        assert!(reports.len() >= 2);
        let &(done, total) = reports.last().unwrap();
        assert!(total > 0 && done < total, "{done}/{total}");
        assert!(running_tasks().lock().unwrap().get("big").is_none());
    }

    #[test]
    fn rejects_bad_input() {
        let slime = |radius| SlimeQuery {
            seed: "12345".into(),
            center_x: 0,
            center_z: 0,
            radius,
            step: 4,
            limit: 3,
            min_separation: 256.0,
            ring_min: 24.0,
            ring_max: 128.0,
        };
        assert!(run(search_slime(slime(0))).is_err());
        assert!(run(search_slime(slime(MAX_SLIME_RADIUS + 1))).is_err());
        assert!(run(search_slime(SlimeQuery { seed: " ".into(), ..slime(100) })).is_err());
        assert!(tools_structure_sets("1.16.5".into()).is_err());
    }

    #[test]
    fn finds_slime_spots_and_clusters() {
        let spots = run(search_slime(SlimeQuery {
            seed: "12345".into(),
            center_x: 0,
            center_z: 0,
            radius: 1500,
            step: 8,
            limit: 3,
            min_separation: 256.0,
            ring_min: 24.0,
            ring_max: 128.0,
        }))
        .unwrap();
        assert_eq!(spots.len(), 3);

        let sets = tools_structure_sets("26".into()).unwrap();
        assert!(sets.iter().any(|s| s.name == "swamp_huts"));
        assert!(sets.iter().all(|s| s.name != "mineshafts" && s.name != "strongholds"));

        let query = |set: &str, min_coverage| ClusterQuery {
            seed: "2960637590762717488".into(),
            version: "26".into(),
            set: set.into(),
            center_x: 0,
            center_z: 0,
            radius: 1_000_000,
            min_count: 3,
            ring_max: 128.0,
            min_coverage,
            check_biomes: true,
        };
        // Same seed as the mcmap_util checks: 2 triple witch huts, 2 triple monuments by coverage.
        assert_eq!(run(search_clusters(query("swamp_huts", None))).unwrap().len(), 2);
        assert_eq!(run(search_clusters(query("ocean_monuments", Some(0.25)))).unwrap().len(), 2);
        assert!(run(search_clusters(query("nope", None))).is_err());
    }

    /// Reads a real world when the developer's launcher folder exists.
    #[test]
    fn reads_world_seed_when_available() {
        let base = dirs::home_dir()
            .unwrap_or_default()
            .join("Library/Application Support/Nolauncher");
        let saves = base.join("instance/Fabulously-Optimized/.minecraft/saves");
        let Some(folder) = std::fs::read_dir(&saves)
            .ok()
            .and_then(|mut d| d.find_map(|e| e.ok()))
            .map(|e| e.file_name().to_string_lossy().to_string())
        else {
            return;
        };
        let info = run(tools_world_info(
            base.to_string_lossy().into(),
            "Fabulously-Optimized".into(),
            folder,
        ))
        .unwrap();
        println!("{info:?}");
        assert!(info.seed.as_deref().is_some_and(|s| s.parse::<i64>().is_ok()));
        assert!(info.version.is_some());
        assert!(info.dimensions.iter().any(|d| d == "minecraft:overworld"));

        let (base, inst) = (base.to_string_lossy().to_string(), "Fabulously-Optimized".to_string());
        let folder = std::fs::read_dir(&saves).unwrap().find_map(|e| e.ok()).unwrap().file_name().to_string_lossy().to_string();
        let regions = run(tools_world_regions(base.clone(), inst.clone(), folder.clone(), "overworld".into())).unwrap();
        assert!(!regions.is_empty());
        let (rx, rz) = regions[0];
        let stats = run(tools_region_chunks(base.clone(), inst.clone(), folder.clone(), "minecraft:overworld".into(), rx, rz)).unwrap();
        assert!(!stats.is_empty());
        // Path tricks in the dimension id are rejected.
        assert!(run(tools_world_regions(base.clone(), inst.clone(), folder.clone(), "minecraft:../../x".into())).is_err());
        let prune = |ticks, days| tools_prune_chunks(base.clone(), inst.clone(), folder.clone(), "overworld".into(), ticks, days, true, true);
        let dry = run(prune(Some(0), None)).unwrap();
        assert_eq!(dry.report.removed, 0);
        assert!(dry.backup.is_none());
        // Every chunk was saved after 1970, and none in the future.
        let all = run(prune(None, Some(-1.0))).unwrap();
        assert_eq!(all.report.removed, all.report.scanned);
        assert_eq!(run(prune(None, Some(365.0 * 60.0))).unwrap().report.removed, 0);
        assert!(run(prune(None, None)).is_err());
    }

    #[test]
    fn unloaded_days_cutoff() {
        assert_eq!(saved_before(1.0, 1_000_000), 1_000_000 - 86_400);
        assert_eq!(saved_before(1e9, 1_000_000), 0);
        assert_eq!(saved_before(-5.0, 1_000_000), 1_000_000);
    }

    #[test]
    fn timestamp_looks_like_game_backups() {
        let t = timestamp();
        assert_eq!(t.len(), 19, "{t}");
        assert!(t.starts_with("20") && &t[10..11] == "_", "{t}");
    }
}
