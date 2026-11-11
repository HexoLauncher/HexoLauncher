import * as api from '$lib/api';
import type { ToolChunkStat, ToolWorldRef } from '$lib/api';

const MAX_IMAGES = 160;
const CONCURRENCY = 3;

type Job = { key: string; priority: number; run: () => Promise<void> };

export class RegionStore {
	/** 有區域檔的區域，`"rx,rz"`。還沒讀到時是 null。 */
	regions: Set<string> | null = null;
	readonly images = new Map<string, ImageBitmap | null>();
	readonly stats = new Map<string, Map<string, ToolChunkStat>>();
	private readonly busy = new Set<string>();
	private queue = new Map<string, Job>();
	/** 這一幀畫面用到的工作 key。 */
	private readonly frameKeys = new Set<string>();
	private running = 0;
	private disposed = false;

	constructor(
		readonly world: ToolWorldRef,
		readonly dimension: string,
		private readonly onchange: () => void
	) {
		this.reloadList();
	}

	get key() {
		return `${this.world.baseDir}|${this.world.instanceName}|${this.world.folder}|${this.dimension}`;
	}

	async reloadList() {
		try {
			const list = await api.toolsWorldRegions(this.world, this.dimension);
			if (this.disposed) return;
			this.regions = new Set(list.map(([x, z]) => `${x},${z}`));
		} catch {
			if (!this.disposed) this.regions = new Set();
		}
		this.onchange();
	}

	has(rx: number, rz: number) {
		return this.regions?.has(`${rx},${rz}`) ?? false;
	}

	/** 已載入的區域圖；還沒載入就排入佇列（數字小的先做）並回傳 undefined。 */
	image(rx: number, rz: number, priority: number, load: boolean): ImageBitmap | null | undefined {
		const key = `${rx},${rz}`;
		this.frameKeys.add(`img:${key}`);
		const cached = this.images.get(key);
		if (cached !== undefined) {
			this.images.delete(key);
			this.images.set(key, cached);
			return cached;
		}
		if (load && this.has(rx, rz))
			this.enqueue(`img:${key}`, priority, () => this.loadImage(rx, rz));
		return undefined;
	}

	/** 已載入的區塊統計（`"cx,cz"` → 統計）；還沒載入就排入佇列。 */
	chunkStats(rx: number, rz: number, priority: number, load: boolean) {
		const key = `${rx},${rz}`;
		this.frameKeys.add(`st:${key}`);
		const cached = this.stats.get(key);
		if (!cached && load && this.has(rx, rz))
			this.enqueue(`st:${key}`, priority, () => this.loadStats(rx, rz));
		return cached;
	}

	/** 開始畫一幀：之後 `image` / `chunkStats` 查過的區域都算這一幀需要的。 */
	beginFrame() {
		this.frameKeys.clear();
	}

	/** 畫完一幀：丟掉這一幀沒用到的排隊工作（已捲出畫面）。 */
	endFrame() {
		for (const key of this.queue.keys()) if (!this.frameKeys.has(key)) this.queue.delete(key);
	}

	/** 編輯過的區域重新讀取。 */
	invalidate(regions: Iterable<string>) {
		for (const key of regions) {
			this.images.get(key)?.close();
			this.images.delete(key);
			this.stats.delete(key);
		}
		this.reloadList();
	}

	dispose() {
		this.disposed = true;
		this.queue.clear();
		for (const bmp of this.images.values()) bmp?.close();
		this.images.clear();
	}

	private enqueue(key: string, priority: number, run: () => Promise<void>) {
		if (this.busy.has(key)) return;
		const queued = this.queue.get(key);
		if (queued) queued.priority = Math.min(queued.priority, priority);
		else this.queue.set(key, { key, priority, run });
		this.pump();
	}

	private pump() {
		while (this.running < CONCURRENCY && this.queue.size) {
			let best: Job | null = null;
			for (const job of this.queue.values()) if (!best || job.priority < best.priority) best = job;
			if (!best) return;
			const job = best;
			this.queue.delete(job.key);
			this.busy.add(job.key);
			this.running++;
			job
				.run()
				.catch(() => {})
				.finally(() => {
					this.busy.delete(job.key);
					this.running--;
					if (!this.disposed) {
						this.onchange();
						this.pump();
					}
				});
		}
	}

	private async loadImage(rx: number, rz: number) {
		const buffer = await api.toolsRenderRegion(this.world, this.dimension, rx, rz);
		if (this.disposed) return;
		let bmp: ImageBitmap | null = null;
		if (buffer.byteLength === 512 * 512 * 4) {
			bmp = await createImageBitmap(new ImageData(new Uint8ClampedArray(buffer), 512, 512));
		}
		this.images.set(`${rx},${rz}`, bmp);
		while (this.images.size > MAX_IMAGES) {
			const oldest = this.images.keys().next().value!;
			this.images.get(oldest)?.close();
			this.images.delete(oldest);
		}
	}

	private async loadStats(rx: number, rz: number) {
		const list = await api.toolsRegionChunks(this.world, this.dimension, rx, rz);
		if (this.disposed) return;
		this.stats.set(`${rx},${rz}`, new Map(list.map((s) => [`${s.x},${s.z}`, s])));
	}
}
