import init, { SeedMap, biomeNames } from './pkg/mcmap_wasm.js';
import wasmUrl from './pkg/mcmap_wasm_bg.wasm?url';

export type Dimension = 'overworld' | 'the_nether' | 'the_end';

/** wasm `structures()` 回傳的一筆結構。 */
export interface FoundStructure {
	set: string;
	structure: string | null;
	chunkX: number;
	chunkZ: number;
	x: number;
	z: number;
}

export interface StrongholdPos {
	chunkX: number;
	chunkZ: number;
	x: number;
	z: number;
}

type JobBase = { id: number; seed: string; version: string };
export type WorkerJob =
	| (JobBase & {
			kind: 'tile';
			dimension: Dimension;
			x0: number;
			z0: number;
			size: number;
			scale: number;
	  })
	| (JobBase & { kind: 'structures'; set: string; x0: number; z0: number; x1: number; z1: number })
	| (JobBase & { kind: 'strongholds'; limit: number });

export type WorkerReply =
	| { id: number; ok: true; kind: 'tile'; rgba: Uint8Array }
	| { id: number; ok: true; kind: 'structures'; list: FoundStructure[] }
	| { id: number; ok: true; kind: 'strongholds'; list: StrongholdPos[] }
	| { id: number; ok: false; error: string };

type Payload<K extends WorkerJob['kind']> = Omit<Extract<WorkerJob, { kind: K }>, 'id'>;
type Result<K extends WorkerJob['kind']> = Extract<WorkerReply, { kind: K }>;

let ready: Promise<unknown> | undefined;

export function loadWasm() {
	return (ready ??= init({ module_or_path: wasmUrl }));
}

let mainMap: { key: string; map: SeedMap } | null = null;
let names: string[] | null = null;

/** 主執行緒上的 SeedMap，給便宜的即時查詢用（滑鼠所在的生物群系、史萊姆區塊）。 */
export async function mainSeedMap(seed: string, version: string): Promise<SeedMap> {
	await loadWasm();
	const key = `${seed}|${version}`;
	if (mainMap?.key !== key) {
		const map = new SeedMap(seed, version, 'normal');
		mainMap?.map.free();
		mainMap = { key, map };
	}
	return mainMap.map;
}

/** 生物群系編號對應的 id（`minecraft:plains` …）。 */
export function biomeName(index: number): string {
	names ??= biomeNames();
	return names[index] ?? '?';
}

interface Pending {
	key: string;
	job: Omit<WorkerJob, 'id'>;
	priority: number;
	resolve: (reply: WorkerReply) => void;
}

/**
 * 固定數量的 worker，工作依優先度排隊（數字小的先做），同一個 key 只會排一次。
 * `prune` 會丟掉已經不需要的排隊工作（例如捲出畫面的圖磚）。
 */
export class WorkerPool {
	private workers: { worker: Worker; busy: Pending | null }[] = [];
	private queue = new Map<string, Pending>();
	private nextId = 1;
	private inflight = new Map<number, Pending>();

	constructor(size: number) {
		for (let i = 0; i < size; i++) {
			const worker = new Worker(new URL('./map.worker.ts', import.meta.url), { type: 'module' });
			const slot = { worker, busy: null as Pending | null };
			worker.onmessage = (event: MessageEvent<WorkerReply>) => {
				const pending = this.inflight.get(event.data.id);
				this.inflight.delete(event.data.id);
				slot.busy = null;
				pending?.resolve(event.data);
				this.pump();
			};
			worker.onerror = (event) => {
				const pending = slot.busy;
				slot.busy = null;
				if (pending) {
					for (const [id, p] of this.inflight) if (p === pending) this.inflight.delete(id);
					pending.resolve({ id: 0, ok: false, error: event.message || 'worker error' });
				}
				this.pump();
			};
			this.workers.push(slot);
		}
	}

	/** 排入工作；同 key 已在排隊就只更新優先度並沿用同一個結果。 */
	run<K extends WorkerJob['kind']>(
		key: string,
		job: Payload<K>,
		priority: number
	): Promise<Result<K> | { ok: false; error: string }> {
		const queued = this.queue.get(key);
		if (queued) {
			queued.priority = Math.min(queued.priority, priority);
			return new Promise((resolve) => {
				const prev = queued.resolve;
				queued.resolve = (r) => {
					prev(r);
					resolve(r as Result<K>);
				};
			});
		}
		return new Promise((resolve) => {
			this.queue.set(key, { key, job, priority, resolve: (r) => resolve(r as Result<K>) });
			this.pump();
		});
	}

	isQueued(key: string) {
		return this.queue.has(key);
	}

	/** 丟掉 `keep` 以外、且還沒開始的工作；它們的 promise 會以 `cancelled` 結束。 */
	prune(keep: (key: string) => boolean) {
		for (const [key, p] of this.queue) {
			if (!keep(key)) {
				this.queue.delete(key);
				p.resolve({ id: 0, ok: false, error: 'cancelled' });
			}
		}
	}

	private pump() {
		for (const slot of this.workers) {
			if (slot.busy || this.queue.size === 0) continue;
			let best: Pending | null = null;
			for (const p of this.queue.values()) if (!best || p.priority < best.priority) best = p;
			if (!best) return;
			this.queue.delete(best.key);
			const id = this.nextId++;
			slot.busy = best;
			this.inflight.set(id, best);
			slot.worker.postMessage({ ...best.job, id });
		}
	}

	destroy() {
		this.prune(() => false);
		for (const { worker } of this.workers) worker.terminate();
		this.workers = [];
	}
}

/**
 * 地圖畫布用的快取。刻意不是響應式狀態：內容只給 canvas 重畫時讀，
 * 放進去時由呼叫端自己排一次重畫。
 */
export class MapCache {
	/** 生物群系圖磚，依最近使用排序（LRU）。 */
	readonly tiles = new Map<string, ImageBitmap>();
	/** 已排隊或計算中的工作 key。 */
	readonly loading = new Set<string>();
	/** 每個結構分塊算出的結構清單。 */
	readonly structures = new Map<string, FoundStructure[]>();
	/** 這一幀畫面需要的工作 key，用來丟掉捲出畫面的排隊工作。 */
	readonly wanted = new Set<string>();

	constructor(private readonly maxTiles: number) {}

	getTile(key: string) {
		const bmp = this.tiles.get(key);
		if (bmp) {
			this.tiles.delete(key);
			this.tiles.set(key, bmp);
		}
		return bmp;
	}

	putTile(key: string, bmp: ImageBitmap) {
		this.tiles.set(key, bmp);
		while (this.tiles.size > this.maxTiles) {
			const oldest = this.tiles.keys().next().value!;
			this.tiles.get(oldest)?.close();
			this.tiles.delete(oldest);
		}
	}

	clear() {
		for (const bmp of this.tiles.values()) bmp.close();
		this.tiles.clear();
		this.structures.clear();
	}
}
