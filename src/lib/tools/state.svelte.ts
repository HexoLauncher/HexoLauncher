import type { Dimension } from './seedmap';
import * as api from '$lib/api';
import type { ToolSlimeSpot, ToolStructureCluster, ToolWorldInfo, ToolWorldRef } from '$lib/api';
import { m } from '$lib/paraglide/messages.js';

export type SeedMode = 'world' | 'manual';

export interface MapFocus {
	x: number;
	z: number;
	label: string;

	ring?: { min: number; max: number };

	points?: { x: number; z: number; label: string }[];

	chunks?: [number, number][];
}

export const VERSIONS = ['26', '1.21.4', '1.21', '1.20', '1.19', '1.18'] as const;

/** 地圖視角：中心（方塊座標）與 log2(每像素幾格)。 */
export interface MapView {
	x: number;
	z: number;
	zoom: number;
}

export type WorldOverlay = 'none' | 'status' | 'inhabited' | 'age';

/** 依條件清理的設定。 */
export interface PruneSettings {
	useInhabited: boolean;
	minutes: number;
	useAge: boolean;
	days: number;
}

export interface SlimeForm {
	centerX: number;
	centerZ: number;
	radius: number;
	step: string;
	limit: number;
	separation: number;
	heightDiff: number;
}

export interface ClusterForm {
	setName: string;
	minCount: string;
	mode: 'enclose' | 'coverage';
	minCoverage: number;
	afkRadius: number;
	radius: number;
	centerX: number;
	centerZ: number;
	checkBiomes: boolean;
}

const STORE_KEY = 'tools.state';
const RESULTS_KEY = 'tools.results';

interface Saved {
	mode: SeedMode;
	instanceId: string;
	worldFolder: string;
	manualSeed: string;
	manualVersion: string;
	dimension: Dimension;
	slime: boolean;
	terrain: boolean;
	sets: string[];
	strongholds: boolean;
	seedView: MapView;
	worldView: MapView;
	worldDimension: string;
	worldOverlay: WorldOverlay;
	prune: PruneSettings;
}

/** 搜尋表單與結果，存在另一個 key，避免每次拖動地圖都重寫一大包資料。 */
interface SavedResults {
	slimeForm: SlimeForm;
	slimeResults: ToolSlimeSpot[];
	slimeElapsed: string;
	clusterForm: ClusterForm;
	clusterResults: ToolStructureCluster[];
	clusterElapsed: string;
	clusterResultMode: 'enclose' | 'coverage';
	clusterResultSet: string;
	clusterResultRing: number;
}

function load<T>(key: string): Partial<T> {
	try {
		return JSON.parse(localStorage.getItem(key) ?? '{}') as Partial<T>;
	} catch {
		return {};
	}
}

function save(key: string, value: unknown) {
	try {
		localStorage.setItem(key, JSON.stringify(value));
	} catch {
		// 無法寫入就算了，只影響下次開啟時的預設值。
	}
}

const saved = load<Saved>(STORE_KEY);
const results = load<SavedResults>(RESULTS_KEY);

export const tools = $state({
	mode: (saved.mode ?? 'world') as SeedMode,
	instanceId: saved.instanceId ?? '',
	worldFolder: saved.worldFolder ?? '',
	manualSeed: saved.manualSeed ?? '',
	manualVersion: saved.manualVersion ?? '26',

	/** 選取的世界讀到的資訊；手動模式或還沒讀到時為 null。 */
	world: null as ToolWorldInfo | null,
	worldError: '',
	/** 目前生效的種子（字串）與生成版本；空字串表示還沒有。 */
	seed: '',
	version: '26',
	/** 讀取世界時的說明或錯誤。 */
	sourceNote: '',
	sourceError: '',

	dimension: (saved.dimension ?? 'overworld') as Dimension,
	slime: saved.slime ?? true,
	/** 種子地圖上，已生成的區塊改畫存檔裡的實際地形。 */
	terrain: saved.terrain ?? true,
	sets: saved.sets ?? ['villages', 'swamp_huts', 'ocean_monuments', 'pillager_outposts'],
	strongholds: saved.strongholds ?? true,

	seedView: saved.seedView ?? { x: 0, z: 0, zoom: 2 },
	focus: null as MapFocus | null,
	/** 待套用的「縮放到能看到半徑幾格」要求；地圖套用後清成 0。 */
	fitRadius: 0,

	worldView: saved.worldView ?? { x: 0, z: 0, zoom: 1 },
	worldDimension: saved.worldDimension ?? 'minecraft:overworld',
	worldOverlay: (saved.worldOverlay ?? 'none') as WorldOverlay,
	prune: {
		useInhabited: true,
		minutes: 1,
		useAge: false,
		days: 30,
		...saved.prune
	} as PruneSettings,

	slimeForm: {
		centerX: 0,
		centerZ: 0,
		radius: 3000,
		step: '4',
		limit: 10,
		separation: 256,
		heightDiff: 0,
		...results.slimeForm
	} as SlimeForm,
	slimeResults: results.slimeResults ?? ([] as ToolSlimeSpot[]),
	slimeElapsed: results.slimeElapsed ?? '',
	slimeRunning: false,
	/** 目前在地圖上查看的結果（不存檔）。 */
	slimeSelected: -1,

	clusterForm: {
		setName: 'swamp_huts',
		minCount: '3',
		mode: 'enclose',
		minCoverage: 25,
		afkRadius: 128,
		radius: 100000,
		centerX: 0,
		centerZ: 0,
		checkBiomes: true,
		...results.clusterForm
	} as ClusterForm,
	clusterResults: results.clusterResults ?? ([] as ToolStructureCluster[]),
	clusterElapsed: results.clusterElapsed ?? '',
	clusterResultMode: results.clusterResultMode ?? ('enclose' as 'enclose' | 'coverage'),
	clusterResultSet: results.clusterResultSet ?? '',
	clusterResultRing: results.clusterResultRing ?? 128,
	clusterRunning: false,
	clusterSelected: -1
});

/** 目前選取世界的參照；手動模式或沒選世界時為 null。 */
export function worldRef(baseDir: string): ToolWorldRef | null {
	if (!tools.instanceId || !tools.worldFolder || !baseDir) return null;
	return { baseDir, instanceName: tools.instanceId, folder: tools.worldFolder };
}

export function persist() {
	const data: Saved = {
		mode: tools.mode,
		instanceId: tools.instanceId,
		worldFolder: tools.worldFolder,
		manualSeed: tools.manualSeed,
		manualVersion: tools.manualVersion,
		dimension: tools.dimension,
		slime: tools.slime,
		terrain: tools.terrain,
		sets: tools.sets,
		strongholds: tools.strongholds,
		seedView: tools.seedView,
		worldView: tools.worldView,
		worldDimension: tools.worldDimension,
		worldOverlay: tools.worldOverlay,
		prune: tools.prune
	};
	save(STORE_KEY, data);
}

export function persistResults() {
	const data: SavedResults = {
		slimeForm: tools.slimeForm,
		slimeResults: tools.slimeResults,
		slimeElapsed: tools.slimeElapsed,
		clusterForm: tools.clusterForm,
		clusterResults: tools.clusterResults,
		clusterElapsed: tools.clusterElapsed,
		clusterResultMode: tools.clusterResultMode,
		clusterResultSet: tools.clusterResultSet,
		clusterResultRing: tools.clusterResultRing
	};
	save(RESULTS_KEY, data);
}

export interface SetInfo {
	label: () => string;
	color: string;
	dimension: Dimension;
	/** 最大的 log2(格/像素)；更遠時不畫，避免一次算太多。 */
	maxZoom: number;
}

export const STRUCTURE_SETS: Record<string, SetInfo> = {
	villages: { label: m.tools_set_villages, color: '#e8b04a', dimension: 'overworld', maxZoom: 6 },
	desert_pyramids: {
		label: m.tools_set_desert_pyramids,
		color: '#f2d27a',
		dimension: 'overworld',
		maxZoom: 6
	},
	jungle_temples: {
		label: m.tools_set_jungle_temples,
		color: '#4caf50',
		dimension: 'overworld',
		maxZoom: 6
	},
	igloos: { label: m.tools_set_igloos, color: '#bfe6ff', dimension: 'overworld', maxZoom: 6 },
	swamp_huts: {
		label: m.tools_set_swamp_huts,
		color: '#a466d6',
		dimension: 'overworld',
		maxZoom: 6
	},
	pillager_outposts: {
		label: m.tools_set_pillager_outposts,
		color: '#9e9e9e',
		dimension: 'overworld',
		maxZoom: 6
	},
	ocean_monuments: {
		label: m.tools_set_ocean_monuments,
		color: '#2ec4b6',
		dimension: 'overworld',
		maxZoom: 6
	},
	woodland_mansions: {
		label: m.tools_set_woodland_mansions,
		color: '#8d5a3b',
		dimension: 'overworld',
		maxZoom: 8
	},
	ancient_cities: {
		label: m.tools_set_ancient_cities,
		color: '#3a6ea5',
		dimension: 'overworld',
		maxZoom: 6
	},
	trial_chambers: {
		label: m.tools_set_trial_chambers,
		color: '#d9822b',
		dimension: 'overworld',
		maxZoom: 6
	},
	trail_ruins: {
		label: m.tools_set_trail_ruins,
		color: '#c96f53',
		dimension: 'overworld',
		maxZoom: 6
	},
	ruined_portals: {
		label: m.tools_set_ruined_portals,
		color: '#7b3fe4',
		dimension: 'overworld',
		maxZoom: 5
	},
	shipwrecks: {
		label: m.tools_set_shipwrecks,
		color: '#b08a5a',
		dimension: 'overworld',
		maxZoom: 5
	},
	ocean_ruins: {
		label: m.tools_set_ocean_ruins,
		color: '#5fa8d3',
		dimension: 'overworld',
		maxZoom: 5
	},
	abandoned_camp: {
		label: m.tools_set_abandoned_camp,
		color: '#c2a878',
		dimension: 'overworld',
		maxZoom: 6
	},
	buried_treasures: {
		label: m.tools_set_buried_treasures,
		color: '#ffd700',
		dimension: 'overworld',
		maxZoom: 2
	},
	mineshafts: {
		label: m.tools_set_mineshafts,
		color: '#a1887f',
		dimension: 'overworld',
		maxZoom: 2
	},
	nether_complexes: {
		label: m.tools_set_nether_complexes,
		color: '#ff5252',
		dimension: 'the_nether',
		maxZoom: 6
	},
	nether_fossils: {
		label: m.tools_set_nether_fossils,
		color: '#e0d6c8',
		dimension: 'the_nether',
		maxZoom: 3
	},
	end_cities: { label: m.tools_set_end_cities, color: '#d6a0ff', dimension: 'the_end', maxZoom: 7 }
};

export function setLabel(name: string) {
	return STRUCTURE_SETS[name]?.label() ?? name;
}

/** 讓地圖置中在一點，並縮放到能看到半徑 `radius` 格的範圍。 */
export function showOnMap(focus: MapFocus, radius: number) {
	tools.focus = focus;
	tools.seedView.x = focus.x;
	tools.seedView.z = focus.z;
	// 縮放交給地圖元件依實際大小決定（見 SeedMapView）。
	tools.fitRadius = radius;
}

// ---- 搜尋耗時預估 ----
// 工作量：史萊姆 = (半徑 / 10 000)² × (4 / 間距)²；多聯 = (半徑 / 1 000 000)² ×（覆蓋率模式 1.2）。
// 每單位工作量的秒數先用 release 建置的實測值，之後依這台電腦實際跑出來的時間校正。

const RATES_KEY = 'tools.rates';
const DEFAULT_RATES = { slime: 0.95, cluster: 0.25 };
/** 太快的搜尋主要是固定開銷，不拿來校正。 */
const MIN_CALIBRATION_SECONDS = 0.3;

export type FinderKind = keyof typeof DEFAULT_RATES;

export const rates = $state({ ...DEFAULT_RATES, ...load<typeof DEFAULT_RATES>(RATES_KEY) });

export function slimeWork(radius: number, step: number) {
	return (radius / 10_000) ** 2 * (4 / Math.max(1, step)) ** 2;
}

export function clusterWork(radius: number, coverage: boolean) {
	return (radius / 1_000_000) ** 2 * (coverage ? 1.2 : 1);
}

export function estimateSeconds(kind: FinderKind, work: number) {
	return work * rates[kind];
}

/** 用實際耗時校正（指數移動平均，新的一次佔一半）。 */
export function recordRun(kind: FinderKind, work: number, seconds: number) {
	if (!(work > 0) || seconds < MIN_CALIBRATION_SECONDS) return;
	rates[kind] = rates[kind] * 0.5 + (seconds / work) * 0.5;
	save(RATES_KEY, rates);
}

// ---- 執行中的搜尋 ----

/** 一個執行中的搜尋：進度由 Tauri 端的事件更新。 */
export interface RunningSearch {
	id: string;
	done: number;
	total: number;
	/** `performance.now()` 的起始時間。 */
	started: number;
	cancelling: boolean;
}

export const searches = $state({
	slime: null as RunningSearch | null,
	cluster: null as RunningSearch | null
});

let progressListener: Promise<unknown> | null = null;

/** 開始追蹤一個搜尋，回傳要傳給 Tauri 的 task id。 */
export function startSearch(kind: FinderKind): string {
	progressListener ??= api.onToolsProgress((p) => {
		for (const key of ['slime', 'cluster'] as const) {
			const s = searches[key];
			if (s?.id === p.taskId) {
				s.done = p.done;
				s.total = p.total;
			}
		}
	});
	const id = `${kind}-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
	searches[kind] = { id, done: 0, total: 0, started: performance.now(), cancelling: false };
	return id;
}

export function finishSearch(kind: FinderKind) {
	searches[kind] = null;
}

export async function cancelSearch(kind: FinderKind) {
	const s = searches[kind];
	if (!s || s.cancelling) return;
	s.cancelling = true;
	try {
		await api.toolsCancel(s.id);
	} catch {
		// 搜尋可能剛好結束了
	}
}
