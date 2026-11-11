<script lang="ts">
	import { onDestroy, onMount, untrack } from 'svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import { Button, CheckRow, Dialog, Select, Spinner, TextInput, ToolButton } from '$lib/ui';
	import MapCanvas, { type ChunkRect, type Frame } from './MapCanvas.svelte';
	import * as api from '$lib/api';
	import type { ToolChunkStat } from '$lib/api';
	import { formatSize } from '$lib/format';
	import { app, report, reportError } from '$lib/state.svelte';
	import { RegionStore } from '$lib/tools/regions';
	import {
		centerDistance,
		drawChunkGrid,
		drawMarker,
		formatTicks,
		scaleText,
		themeColors,
		visibleRegions,
		type ThemeColors
	} from '$lib/tools/draw';
	import { tools, worldRef, type WorldOverlay } from '$lib/tools/state.svelte';
	import { m } from '$lib/paraglide/messages.js';

	/** 放大到這個程度以內才讀區域圖（log2 格/像素）。 */
	const LOAD_ZOOM = 5;
	/** 圖層需要的區塊統計，放大到這個程度以內才讀。 */
	const STATS_ZOOM = 3;
	const MAX_SELECTION = 65_536;

	let canvas = $state<MapCanvas>();
	let mode = $state<'pan' | 'select'>('pan');
	let store: RegionStore | null = null;
	let colors: ThemeColors = { bg: '#1d1518', ink: '#e8e8e8', line: '#342d30', brand: '#e5343f' };
	let markers: { sx: number; sy: number; text: string }[] = [];
	const selection = new SvelteSet<string>();

	let confirmRegenerate = $state(false);
	let pruneOpen = $state(false);
	let prunePreview = $state<{ removed: number; scanned: number } | null>(null);
	let backup = $state(true);
	let working = $state(false);
	let statsHidden = $state(false);

	const ref = $derived(worldRef(app.baseDir));
	const world = $derived(tools.world);
	const dimensions = $derived(world?.dimensions ?? []);
	const running = $derived(app.running.some((r) => r.instanceName === tools.instanceId));
	const editBlocked = $derived(!world || world.locked || running);
	const criteria = $derived<api.ToolPruneCriteria>({
		maxInhabitedTicks: tools.prune.useInhabited ? Math.max(0, Math.round(Number(tools.prune.minutes) * 1200)) : null,
		unloadedDays: tools.prune.useAge ? Math.max(0, Number(tools.prune.days)) : null
	});
	const hasCriteria = $derived(criteria.maxInhabitedTicks !== null || criteria.unloadedDays !== null);

	// 條件改了，舊的預覽數量就不準了。
	$effect(() => {
		void [criteria.maxInhabitedTicks, criteria.unloadedDays, tools.worldDimension];
		untrack(() => (prunePreview = null));
	});

	/** 和後端 `PruneFilter::matches` 相同的規則：勾選的條件都要符合。 */
	function matches(s: ToolChunkStat, now: number) {
		const { maxInhabitedTicks: ticks, unloadedDays: days } = criteria;
		if (ticks === null && days === null) return false;
		const cutoff = days === null ? null : now - days * 86_400;
		return (ticks === null || s.inhabitedTime < ticks) && (cutoff === null || (s.savedAt !== 0 && s.savedAt < cutoff));
	}

	// 世界裡沒有目前選的維度時，換成第一個有的。
	$effect(() => {
		if (dimensions.length && !dimensions.includes(tools.worldDimension)) tools.worldDimension = dimensions[0];
	});

	// 換世界或維度：重建區域來源、清掉選取。
	$effect(() => {
		const r = ref;
		const dim = tools.worldDimension;
		const key = r && world ? `${r.baseDir}|${r.instanceName}|${r.folder}|${dim}` : '';
		if (store?.key === key) return;
		store?.dispose();
		untrack(() => {
			selection.clear();
			prunePreview = null;
		});
		store = key && r ? new RegionStore(r, dim, () => {
			canvas?.redraw();
			canvas?.refreshHover();
		}) : null;
		canvas?.redraw();
	});

	$effect(() => {
		void [tools.worldOverlay, selection.size, pruneOpen, criteria, world];
		canvas?.redraw();
	});

	const dimLabel = (id: string) =>
		({
			'minecraft:overworld': m.tools_dim_overworld(),
			'minecraft:the_nether': m.tools_dim_nether(),
			'minecraft:the_end': m.tools_dim_end()
		})[id] ?? id;

	function draw(ctx: CanvasRenderingContext2D, f: Frame) {
		ctx.fillStyle = colors.bg;
		ctx.fillRect(0, 0, f.width, f.height);
		markers = [];
		if (!store) return;
		store.beginFrame();
		const { rx0, rz0, rx1, rz1 } = visibleRegions(f);
		const regionPx = 512 / f.b;
		const overlay: WorldOverlay = pruneOpen ? 'age' : tools.worldOverlay;
		const wantStats = overlay !== 'none' && f.zoom <= STATS_ZOOM;
		statsHidden = overlay !== 'none' && !wantStats;

		for (let rz = rz0; rz <= rz1; rz++) {
			for (let rx = rx0; rx <= rx1; rx++) {
				if (!store.has(rx, rz)) continue;
				const x = f.toX(rx * 512);
				const y = f.toY(rz * 512);
				const prio = centerDistance(f, rx * 512 + 256, rz * 512 + 256);
				const bmp = store.image(rx, rz, prio, f.zoom <= LOAD_ZOOM);
				if (bmp) ctx.drawImage(bmp, x, y, regionPx, regionPx);
				else {
					ctx.fillStyle = 'rgba(255, 255, 255, 0.06)';
					ctx.fillRect(x, y, regionPx, regionPx);
				}
				if (wantStats) {
					const stats = store.chunkStats(rx, rz, prio + 1, true);
					if (stats) drawStats(ctx, f, stats, overlay);
				}
				if (regionPx >= 24) {
					ctx.strokeStyle = 'rgba(255, 255, 255, 0.12)';
					ctx.lineWidth = 1;
					ctx.strokeRect(Math.round(x) + 0.5, Math.round(y) + 0.5, regionPx, regionPx);
				}
			}
		}
		store.endFrame();
		drawChunkGrid(ctx, f);
		drawSelection(ctx, f);

		if (world) {
			if (world.spawn && tools.worldDimension === 'minecraft:overworld') {
				const [sx, , sz] = world.spawn;
				drawMarker(ctx, f, f.toX(sx), f.toY(sz), '#7fffd4', 'diamond');
				markers.push({ sx: f.toX(sx), sy: f.toY(sz), text: `${m.tools_spawn()} · ${sx}, ${sz}` });
			}
			for (const p of world.players) {
				if (p.dimension !== tools.worldDimension) continue;
				drawMarker(ctx, f, f.toX(p.x), f.toY(p.z), '#ffffff', 'player');
				markers.push({
					sx: f.toX(p.x),
					sy: f.toY(p.z),
					text: `${m.tools_player()} · ${Math.floor(p.x)}, ${Math.floor(p.y)}, ${Math.floor(p.z)}`
				});
			}
		}
	}

	function drawStats(ctx: CanvasRenderingContext2D, f: Frame, stats: Map<string, ToolChunkStat>, overlay: WorldOverlay) {
		const size = 16 / f.b;
		const now = Date.now() / 1000;
		for (const s of stats.values()) {
			const x = f.toX(s.x * 16);
			const y = f.toY(s.z * 16);
			if (x + size < 0 || y + size < 0 || x > f.width || y > f.height) continue;
			if (pruneOpen) {
				if (matches(s, now)) {
					ctx.fillStyle = 'rgba(255, 60, 60, 0.5)';
					ctx.fillRect(x, y, size, size);
				}
				continue;
			}
			if (overlay === 'status') {
				if (s.status && s.status !== 'full') {
					ctx.fillStyle = 'rgba(255, 163, 71, 0.55)';
					ctx.fillRect(x, y, size, size);
				}
			} else if (overlay === 'age') {
				ctx.fillStyle = ageColor(s.savedAt, now);
				ctx.fillRect(x, y, size, size);
			} else {
				ctx.fillStyle = heat(s.inhabitedTime);
				ctx.fillRect(x, y, size, size);
			}
		}
	}

	/** 最後載入時間的顏色：剛載入過偏綠，一年以上偏紫；沒有時間戳記的變暗。 */
	function ageColor(savedAt: number, now: number) {
		if (!savedAt) return 'rgba(0, 0, 0, 0.35)';
		const days = Math.max(0, (now - savedAt) / 86_400);
		const t = Math.min(1, Math.log10(1 + days) / Math.log10(1 + 365));
		return `hsla(${130 + 150 * t}, 80%, 55%, 0.55)`;
	}

	/** 居住時間的顏色：0 透明，1 分鐘偏藍，10 小時以上紅。 */
	function heat(ticks: number) {
		if (ticks <= 0) return 'rgba(0, 0, 0, 0.35)';
		const t = Math.min(1, Math.log10(1 + ticks / 1200) / Math.log10(1 + 600));
		const hue = 220 - 220 * t;
		return `hsla(${hue}, 90%, 55%, 0.55)`;
	}

	function drawSelection(ctx: CanvasRenderingContext2D, f: Frame) {
		if (!selection.size) return;
		const size = 16 / f.b;
		ctx.fillStyle = 'rgba(229, 52, 63, 0.35)';
		for (const key of selection) {
			const [cx, cz] = key.split(',').map(Number);
			const x = f.toX(cx * 16);
			const y = f.toY(cz * 16);
			if (x + size < 0 || y + size < 0 || x > f.width || y > f.height) continue;
			ctx.fillRect(x, y, Math.max(size, 1), Math.max(size, 1));
		}
	}

	function describe(x: number, z: number, sx: number, sy: number) {
		if (!store) return [];
		const cx = x >> 4;
		const cz = z >> 4;
		const lines = [`X ${x} · Z ${z} · ${m.tools_chunk()} ${cx}, ${cz} · ${m.tools_region()} r.${cx >> 5}.${cz >> 5}`];
		const stats = store.stats.get(`${cx >> 5},${cz >> 5}`)?.get(`${cx},${cz}`);
		if (stats) {
			const parts = [
				`${m.tools_status()} ${stats.status ?? '?'}`,
				`${m.tools_inhabited()} ${formatTicks(stats.inhabitedTime)}`
			];
			if (stats.savedAt) parts.push(`${m.tools_last_loaded()} ${formatAge(stats.savedAt)}`);
			lines.push(parts.join(' · '));
		}
		let best = '';
		let bestDist = 100;
		for (const mk of markers) {
			const d = (mk.sx - sx) ** 2 + (mk.sy - sy) ** 2;
			if (d < bestDist) {
				bestDist = d;
				best = mk.text;
			}
		}
		return best ? [best, ...lines] : lines;
	}

	function selectRect(r: ChunkRect, additive: boolean) {
		const count = (r.cx1 - r.cx0 + 1) * (r.cz1 - r.cz0 + 1);
		if (count + (additive ? selection.size : 0) > MAX_SELECTION) {
			report(m.tools_selection_too_large({ max: MAX_SELECTION }));
			return;
		}
		if (!additive) selection.clear();
		if (count === 1 && additive && selection.has(`${r.cx0},${r.cz0}`)) {
			selection.delete(`${r.cx0},${r.cz0}`);
			return;
		}
		for (let cz = r.cz0; cz <= r.cz1; cz++) for (let cx = r.cx0; cx <= r.cx1; cx++) selection.add(`${cx},${cz}`);
	}

	function jumpTo(x: number, z: number, zoom = 0) {
		tools.worldView.x = x;
		tools.worldView.z = z;
		tools.worldView.zoom = zoom;
	}

	function formatAge(savedAt: number) {
		const hours = (Date.now() / 1000 - savedAt) / 3600;
		if (hours < 1) return m.tools_ago_just_now();
		if (hours < 48) return m.tools_ago_hours({ count: Math.floor(hours) });
		return m.tools_ago_days({ count: Math.floor(hours / 24) });
	}

	function affected(chunks: string[]) {
		return new Set(chunks.map((k) => k.split(',').map(Number)).map(([cx, cz]) => `${cx >> 5},${cz >> 5}`));
	}

	async function regenerateSelected() {
		if (!ref || !store) return;
		confirmRegenerate = false;
		working = true;
		const chunks = [...selection];
		try {
			const result = await api.toolsRegenerateChunks(
				ref,
				tools.worldDimension,
				chunks.map((k) => k.split(',').map(Number) as [number, number]),
				backup
			);
			store.invalidate(affected(chunks));
			selection.clear();
			report(
				result.backup
					? m.tools_status_regenerated_backup({ count: result.report.removed, path: result.backup })
					: m.tools_status_regenerated({ count: result.report.removed })
			);
		} catch (e) {
			reportError(m.tools_error_edit(), e);
		} finally {
			working = false;
		}
	}

	async function previewPrune() {
		if (!ref) return;
		working = true;
		try {
			const result = await api.toolsPruneChunks(ref, tools.worldDimension, criteria, true, false);
			prunePreview = { removed: result.report.removed, scanned: result.report.scanned };
		} catch (e) {
			reportError(m.tools_error_edit(), e);
		} finally {
			working = false;
		}
	}

	async function runPrune() {
		if (!ref || !store) return;
		working = true;
		try {
			const result = await api.toolsPruneChunks(ref, tools.worldDimension, criteria, false, backup);
			store.invalidate([...store.images.keys(), ...store.stats.keys()]);
			pruneOpen = false;
			prunePreview = null;
			report(
				m.tools_status_pruned({
					count: result.report.removed,
					size: formatSize(result.report.bytesFreed)
				}) + (result.backup ? ` · ${m.tools_backup_saved({ path: result.backup })}` : '')
			);
		} catch (e) {
			reportError(m.tools_error_edit(), e);
		} finally {
			working = false;
		}
	}

	onMount(() => {
		colors = themeColors();
		canvas?.redraw();
	});

	onDestroy(() => store?.dispose());
</script>

<div class="flex min-h-0 flex-1 flex-col gap-2">
	<div class="flex shrink-0 flex-wrap items-center gap-1.75">
		<Select class="w-32" bind:value={tools.worldDimension} disabled={!dimensions.length} aria-label={m.tools_dimension()}>
			{#each dimensions as d (d)}
				<option value={d}>{dimLabel(d)}</option>
			{/each}
		</Select>
		<Select class="w-32" bind:value={tools.worldOverlay} aria-label={m.tools_overlay()}>
			<option value="none">{m.tools_overlay_none()}</option>
			<option value="status">{m.tools_overlay_status()}</option>
			<option value="inhabited">{m.tools_overlay_inhabited()}</option>
			<option value="age">{m.tools_overlay_age()}</option>
		</Select>
		<div class="flex shrink-0 gap-0.5 rounded-md bg-btn p-0.5" role="group" aria-label={m.tools_map_mode()}>
			<ToolButton layout="bare" class="px-2.5 py-1" active={mode === 'pan'} onclick={() => (mode = 'pan')}>
				{m.tools_mode_pan()}
			</ToolButton>
			<ToolButton layout="bare" class="px-2.5 py-1" active={mode === 'select'} onclick={() => (mode = 'select')}>
				{m.tools_mode_select()}
			</ToolButton>
		</div>
		{#if world?.players.length}
			{@const p = world.players.find((p) => p.dimension === tools.worldDimension) ?? world.players[0]}
			<Button
				icon="user"
				onclick={() => {
					if (dimensions.includes(p.dimension)) tools.worldDimension = p.dimension;
					jumpTo(p.x, p.z);
				}}
			>
				{m.tools_player()}
			</Button>
		{/if}
		{#if world?.spawn}
			<Button
				icon="rerender"
				onclick={() => {
					tools.worldDimension = 'minecraft:overworld';
					jumpTo(world.spawn![0], world.spawn![2]);
				}}
			>
				{m.tools_spawn()}
			</Button>
		{/if}
		<span class="flex-1"></span>
		<Button icon="minus" onclick={() => canvas?.zoomBy(1)} aria-label={m.tools_zoom_out()} title={m.tools_zoom_out()}></Button>
		<span class="min-w-24 text-center whitespace-nowrap text-muted tabular-nums">{scaleText(tools.worldView.zoom)}</span>
		<Button icon="plus" onclick={() => canvas?.zoomBy(-1)} aria-label={m.tools_zoom_in()} title={m.tools_zoom_in()}></Button>
	</div>

	<div class="flex shrink-0 flex-wrap items-center gap-1.75">
		{#if editBlocked && world}
			<span class="text-warn">{m.tools_world_in_use()}</span>
		{:else}
			<span class="text-muted">
				{selection.size ? m.tools_selected_chunks({ count: selection.size }) : m.tools_select_hint()}
			</span>
		{/if}
		<span class="flex-1"></span>
		{#if working}<Spinner />{/if}
		<Button disabled={!selection.size} onclick={() => selection.clear()}>{m.tools_clear_selection()}</Button>
		<Button
			variant="destructive"
			icon="refresh"
			disabled={!selection.size || editBlocked || working}
			onclick={() => (confirmRegenerate = true)}
		>
			{m.tools_regenerate_chunks()}
		</Button>
		<Button
			icon="trash"
			disabled={!world || editBlocked || working}
			onclick={() => {
				pruneOpen = true;
				prunePreview = null;
				tools.worldView.zoom = Math.min(tools.worldView.zoom, STATS_ZOOM);
			}}
		>
			{m.tools_prune()}
		</Button>
	</div>

	<div class="relative flex min-h-0 flex-1 overflow-hidden rounded-lg border border-line">
		<MapCanvas bind:this={canvas} view={tools.worldView} {mode} {draw} {describe} onselectrect={selectRect}>
			{#snippet overlay()}
				{#if !ref}
					<div class="pointer-events-none absolute inset-0 grid place-items-center text-muted">{m.tools_pick_world()}</div>
				{:else if world && !dimensions.length}
					<div class="pointer-events-none absolute inset-0 grid place-items-center text-muted">{m.tools_world_empty()}</div>
				{/if}
				{#if statsHidden}
					<div class="pointer-events-none absolute top-2 left-2 rounded-md bg-scrim px-2.5 py-1 text-[12px] text-muted">
						{m.tools_zoom_for_overlay()}
					</div>
				{/if}
			{/snippet}
		</MapCanvas>

		{#if pruneOpen}
			<aside class="flex w-64 shrink-0 flex-col gap-2 overflow-y-auto border-l border-line bg-window px-3 py-2.5">
				<span class="font-semibold">{m.tools_prune()}</span>
				<p class="text-[12px] leading-[1.6] text-muted">{m.tools_prune_intro()}</p>
				<CheckRow><input type="checkbox" bind:checked={tools.prune.useInhabited} />{m.tools_prune_by_inhabited()}</CheckRow>
				<label class="-mt-1 flex items-center gap-2 pl-6 text-muted">
					<TextInput class="w-20" type="number" min="0" step="0.5" disabled={!tools.prune.useInhabited} bind:value={tools.prune.minutes} />
					{m.tools_unit_minutes()}
				</label>
				<CheckRow><input type="checkbox" bind:checked={tools.prune.useAge} />{m.tools_prune_by_age()}</CheckRow>
				<label class="-mt-1 flex items-center gap-2 pl-6 text-muted">
					<TextInput class="w-20" type="number" min="0" step="1" disabled={!tools.prune.useAge} bind:value={tools.prune.days} />
					{m.tools_unit_days()}
				</label>
				<Button disabled={working || !hasCriteria} onclick={previewPrune}>{m.tools_prune_preview()}</Button>
				{#if prunePreview}
					<span class="text-ink">{m.tools_prune_result({ removed: prunePreview.removed, scanned: prunePreview.scanned })}</span>
				{/if}
				<CheckRow><input type="checkbox" bind:checked={backup} />{m.tools_backup_first()}</CheckRow>
				<div class="mt-auto flex gap-1.75">
					<Button
						variant="destructive"
						disabled={working || editBlocked || !prunePreview || !prunePreview.removed}
						onclick={runPrune}
					>
						{m.tools_prune_run()}
					</Button>
					<Button
						onclick={() => {
							pruneOpen = false;
							prunePreview = null;
						}}
					>
						{m.common_cancel()}
					</Button>
				</div>
			</aside>
		{/if}
	</div>
</div>

{#if confirmRegenerate}
	<Dialog
		title={m.tools_regenerate_title({ count: selection.size })}
		width="460px"
		onclose={() => (confirmRegenerate = false)}
	>
		<div class="flex flex-col gap-3 px-5 py-5 leading-[1.75]">
			<p>{m.tools_regenerate_confirm()}</p>
			<p class="text-muted">{m.tools_regenerate_note()}</p>
			<CheckRow><input type="checkbox" bind:checked={backup} />{m.tools_backup_first()}</CheckRow>
		</div>
		{#snippet footer()}
			<Button variant="destructive" icon="refresh" onclick={regenerateSelected}>{m.tools_regenerate()}</Button>
			<Button onclick={() => (confirmRegenerate = false)}>{m.common_cancel()}</Button>
		{/snippet}
	</Dialog>
{/if}
