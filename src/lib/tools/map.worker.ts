/// <reference lib="webworker" />
import init, { SeedMap } from './pkg/mcmap_wasm.js';
import wasmUrl from './pkg/mcmap_wasm_bg.wasm?url';
import type { WorkerJob, WorkerReply } from './seedmap';

const ready = init({ module_or_path: wasmUrl });
let current: { key: string; map: SeedMap } | null = null;

function mapFor(seed: string, version: string): SeedMap {
	const key = `${seed}|${version}`;
	if (current?.key !== key) {
		current?.map.free();
		current = { key, map: new SeedMap(seed, version, 'normal') };
	}
	return current.map;
}

const scope = self as unknown as DedicatedWorkerGlobalScope;

scope.onmessage = async (event: MessageEvent<WorkerJob>) => {
	const job = event.data;
	try {
		await ready;
		const map = mapFor(job.seed, job.version);
		let reply: WorkerReply;
		if (job.kind === 'tile') {
			const rgba = map.biomeTile(job.dimension, job.x0, job.z0, job.size, job.size, job.scale);
			reply = { id: job.id, ok: true, kind: 'tile', rgba };
			scope.postMessage(reply, [rgba.buffer]);
			return;
		}
		if (job.kind === 'structures') {
			const list = map.structures(job.set, job.x0, job.z0, job.x1, job.z1, true);
			reply = { id: job.id, ok: true, kind: 'structures', list };
		} else {
			reply = { id: job.id, ok: true, kind: 'strongholds', list: map.strongholds(job.limit) };
		}
		scope.postMessage(reply);
	} catch (e) {
		scope.postMessage({ id: job.id, ok: false, error: e instanceof Error ? e.message : String(e) });
	}
};
