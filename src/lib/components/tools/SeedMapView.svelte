<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { Button, CheckRow, Select, TextInput } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import MapCanvas, { type Frame } from './MapCanvas.svelte';
	import { app } from '$lib/state.svelte';
	import {
		MapCache,
		WorkerPool,
		biomeLabel,
		mainSeedMap,
		type Dimension,
		type StrongholdPos
	} from '$lib/tools/seedmap';
	import { RegionStore } from '$lib/tools/regions';
	import {
		centerDistance,
		drawChunkGrid,
		drawIconMarker,
		drawMarker,
		scaleText,
		themeColors,
		visibleRegions,
		type ThemeColors
	} from '$lib/tools/draw';
	import { structureSprite, structureSpriteUrl } from '$lib/tools/structureIcons';
	import { STRUCTURE_SETS, setLabel, tools, worldRef } from '$lib/tools/state.svelte';
	import type { SeedMap } from '$lib/tools/pkg/mcmap_wasm.js';
	import { m } from '$lib/paraglide/messages.js';

	/** 圖磚邊長（像素）。 */
	const TILE = 256;
	/** 結構查詢的分塊大小（格）；必須是 16 的倍數，分塊邊界才會對齊區塊。 */
	const STRUCT_TILE = 4096;
	const MAX_TILES = 320;
	/** 實際地形：放大到這個程度以內才讀區域檔（log2 格/像素）。 */
	const TERRAIN_LOAD_ZOOM = 5;

	/** 內嵌在搜尋頁時不顯示上方工具列，只在地圖角落放縮放按鈕。 */
	let { compact = false, onexpand }: { compact?: boolean; onexpand?: () => void } = $props();

	let canvas = $state<MapCanvas>();
	let layersOpen = $state(false);
	const mapButton =
		'grid size-7 place-items-center rounded-md bg-scrim text-ink transition-colors duration-150 hover:bg-btn-hi';
	let gotoText = $state('');
	let pendingCount = $state(0);
	let main = $state<SeedMap | null>(null);
	let available = $state<string[]>([]);
	let hiddenSets = $state(false);

	const pool = new WorkerPool(Math.max(2, Math.min(4, (navigator.hardwareConcurrency || 4) - 1)));
	const cache = new MapCache(MAX_TILES);
	const { loading, structures } = cache;
	let strongholds: { key: string; list: StrongholdPos[] } | null = null;
	interface Selection {
		title: string;
		subtitle: string;
		icon: string;
		x: number;
		z: number;
		biome: string;
	}
	interface Marker {
		sx: number;
		sy: number;
		text: string;
		pick?: Omit<Selection, 'biome'>;
	}
	let markers: Marker[] = [];
	let selected = $state<Selection | null>(null);
	let panelPos = $state<{ sx: number; sy: number; w: number; h: number } | null>(null);
	let copied = $state('');
	let colors: ThemeColors = { bg: '#1d1518', ink: '#e8e8e8', line: '#342d30', brand: '#e5343f' };
	let terrain: RegionStore | null = null;

	const seedKey = $derived(`${tools.seed}|${tools.version}`);
	const dims: { id: Dimension; label: () => string }[] = [
		{ id: 'overworld', label: m.tools_dim_overworld },
		{ id: 'the_nether', label: m.tools_dim_nether },
		{ id: 'the_end', label: m.tools_dim_end }
	];
	/** 實際地形只在「實例世界」模式、且該維度有存檔時才有。 */
	const terrainSource = $derived.by(() => {
		const ref = worldRef(app.baseDir);
		const dim = `minecraft:${tools.dimension}`;
		if (tools.mode !== 'world' || !ref || !tools.world?.dimensions.includes(dim)) return null;
		return { ref, dim };
	});

	// 換種子：清掉快取，重新取得這個版本有哪些結構。
	$effect(() => {
		const { seed, version } = tools;
		cache.clear();
		strongholds = null;
		pool.prune(() => false);
		main = null;
		if (!seed) return;
		mainSeedMap(seed, version)
			.then((map) => {
				if (tools.seed !== seed || tools.version !== version) return;
				main = map;
				const sets = map.structureSets() as { name: string }[];
				available = sets.map((s) => s.name).filter((n) => n in STRUCTURE_SETS);
			})
			.catch(() => (main = null));
	});

	// 換世界或維度：重建實際地形的來源。
	$effect(() => {
		const source = terrainSource;
		const key = source ? `${source.ref.baseDir}|${source.ref.instanceName}|${source.ref.folder}|${source.dim}` : '';
		if (terrain?.key === key) return;
		terrain?.dispose();
		terrain = source ? new RegionStore(source.ref, source.dim, () => canvas?.redraw()) : null;
	});

	$effect(() => {
		void [tools.dimension, tools.slime, tools.terrain, tools.strongholds, tools.focus, tools.sets.length];
		void [available, main, seedKey, terrainSource];
		canvas?.redraw();
	});

	// 搜尋結果要求「看得到整個範圍」：畫布有尺寸後依實際大小縮放。
	$effect(() => {
		const radius = tools.fitRadius;
		if (!radius || !canvas) return;
		if (canvas.fit(radius)) {
			tools.fitRadius = 0;
			return;
		}
		const id = requestAnimationFrame(() => {
			if (canvas?.fit(radius)) tools.fitRadius = 0;
		});
		return () => cancelAnimationFrame(id);
	});

	function tileKey(level: number, tx: number, tz: number) {
		return `tile|${seedKey}|${tools.dimension}|${level}|${tx}|${tz}`;
	}

	function requestTile(key: string, level: number, tx: number, tz: number, priority: number) {
		if (loading.has(key)) return;
		loading.add(key);
		pendingCount = loading.size;
		const scale = 2 ** level;
		const span = TILE * scale;
		pool
			.run<'tile'>(
				key,
				{
					kind: 'tile',
					seed: tools.seed,
					version: tools.version,
					dimension: tools.dimension,
					x0: tx * span,
					z0: tz * span,
					size: TILE,
					scale
				},
				priority
			)
			.then(async (reply) => {
				if (!reply.ok || !key.startsWith(`tile|${seedKey}|`)) return;
				const pixels = new Uint8ClampedArray(
					reply.rgba.buffer as ArrayBuffer,
					reply.rgba.byteOffset,
					reply.rgba.byteLength
				);
				cache.putTile(key, await createImageBitmap(new ImageData(pixels, TILE, TILE)));
				canvas?.redraw();
			})
			.finally(() => {
				loading.delete(key);
				pendingCount = loading.size;
			});
	}

	function requestStructures(key: string, set: string, sx: number, sz: number, priority: number) {
		if (loading.has(key)) return;
		loading.add(key);
		pool
			.run<'structures'>(
				key,
				{
					kind: 'structures',
					seed: tools.seed,
					version: tools.version,
					set,
					x0: sx * STRUCT_TILE,
					z0: sz * STRUCT_TILE,
					x1: sx * STRUCT_TILE + STRUCT_TILE - 1,
					z1: sz * STRUCT_TILE + STRUCT_TILE - 1
				},
				priority
			)
			.then((reply) => {
				if (reply.ok) structures.set(key, reply.list);
				else if (reply.error !== 'cancelled') structures.set(key, []);
				canvas?.redraw();
			})
			.finally(() => loading.delete(key));
	}

	function requestStrongholds() {
		const key = `sh|${seedKey}`;
		if (strongholds?.key === key || loading.has(key)) return;
		loading.add(key);
		pool
			.run<'strongholds'>(key, { kind: 'strongholds', seed: tools.seed, version: tools.version, limit: 128 }, -1)
			.then((reply) => {
				if (reply.ok && key === `sh|${seedKey}`) strongholds = { key, list: reply.list };
				canvas?.redraw();
			})
			.finally(() => loading.delete(key));
	}

	function draw(ctx: CanvasRenderingContext2D, f: Frame) {
		ctx.fillStyle = colors.bg;
		ctx.fillRect(0, 0, f.width, f.height);
		markers = [];
		if (!tools.seed) return;
		const wanted = cache.wanted;
		wanted.clear();

		drawBiomes(ctx, f, wanted);
		drawTerrain(ctx, f);
		drawSlime(ctx, f);
		drawChunkGrid(ctx, f);
		drawStructures(ctx, f, wanted);
		drawFocus(ctx, f);
		drawSelection(ctx, f);

		pool.prune((key) => wanted.has(key) || key.startsWith('sh|'));
	}

	/** 生物群系圖磚：用不小於目前縮放的整數層，近的先算；還沒算好就用粗一層的頂著。 */
	function drawBiomes(ctx: CanvasRenderingContext2D, f: Frame, wanted: Set<string>) {
		const level = Math.max(0, Math.ceil(f.zoom - 1e-9));
		const span = TILE * 2 ** level;
		const cells: [number, number, number][] = [];
		for (let tz = Math.floor(f.top / span); tz <= Math.floor((f.top + f.height * f.b) / span); tz++) {
			for (let tx = Math.floor(f.left / span); tx <= Math.floor((f.left + f.width * f.b) / span); tx++) {
				cells.push([tx, tz, centerDistance(f, (tx + 0.5) * span, (tz + 0.5) * span)]);
			}
		}
		cells.sort((a, c) => a[2] - c[2]);
		for (const [tx, tz, dist] of cells) {
			const key = tileKey(level, tx, tz);
			const dx = f.toX(tx * span);
			const dy = f.toY(tz * span);
			const size = span / f.b;
			const bmp = cache.getTile(key);
			if (bmp) {
				ctx.drawImage(bmp, dx, dy, size, size);
				continue;
			}
			for (let up = 1; up <= 4; up++) {
				const pSpan = span * 2 ** up;
				const ptx = Math.floor((tx * span) / pSpan);
				const ptz = Math.floor((tz * span) / pSpan);
				const parent = cache.tiles.get(tileKey(level + up, ptx, ptz));
				if (parent) {
					const ps = TILE / 2 ** up;
					const px = ((tx * span - ptx * pSpan) / pSpan) * TILE;
					const pz = ((tz * span - ptz * pSpan) / pSpan) * TILE;
					ctx.drawImage(parent, px, pz, ps, ps, dx, dy, size, size);
					break;
				}
			}
			wanted.add(key);
			requestTile(key, level, tx, tz, dist);
		}
	}

	/** 已生成的區塊改畫存檔裡的實際地形（未生成的地方是透明的，會露出生物群系）。 */
	function drawTerrain(ctx: CanvasRenderingContext2D, f: Frame) {
		if (!terrain || !tools.terrain) return;
		const { rx0, rz0, rx1, rz1 } = visibleRegions(f);
		const load = f.zoom <= TERRAIN_LOAD_ZOOM;
		terrain.beginFrame();
		for (let rz = rz0; rz <= rz1; rz++) {
			for (let rx = rx0; rx <= rx1; rx++) {
				if (!terrain.has(rx, rz)) continue;
				const bmp = terrain.image(rx, rz, centerDistance(f, rx * 512 + 256, rz * 512 + 256), load);
				if (bmp) ctx.drawImage(bmp, f.toX(rx * 512), f.toY(rz * 512), 512 / f.b, 512 / f.b);
			}
		}
		terrain.endFrame();
	}

	function drawSlime(ctx: CanvasRenderingContext2D, f: Frame) {
		const chunkPx = 16 / f.b;
		if (!main || !tools.slime || tools.dimension !== 'overworld' || chunkPx < 6) return;
		const cx0 = Math.floor(f.left / 16);
		const cz0 = Math.floor(f.top / 16);
		const cw = Math.ceil(f.width / chunkPx) + 2;
		const ch = Math.ceil(f.height / chunkPx) + 2;
		const bits = main.slimeChunks(cx0, cz0, cw, ch);
		ctx.fillStyle = 'rgba(120, 255, 140, 0.45)';
		ctx.strokeStyle = 'rgba(10, 70, 25, 0.9)';
		ctx.lineWidth = chunkPx >= 16 ? 2 : 1;
		for (let j = 0; j < ch; j++) {
			for (let i = 0; i < cw; i++) {
				if (!bits[j * cw + i]) continue;
				const x = f.toX((cx0 + i) * 16);
				const y = f.toY((cz0 + j) * 16);
				ctx.fillRect(x, y, chunkPx, chunkPx);
				ctx.strokeRect(x + 1, y + 1, chunkPx - 2, chunkPx - 2);
			}
		}
	}

	function drawStructures(ctx: CanvasRenderingContext2D, f: Frame, wanted: Set<string>) {
		let hidden = false;
		for (const set of tools.sets) {
			const info = STRUCTURE_SETS[set];
			if (!info || info.dimension !== tools.dimension || !available.includes(set)) continue;
			if (f.zoom > info.maxZoom) {
				hidden = true;
				continue;
			}
			for (let sz = Math.floor(f.top / STRUCT_TILE); sz <= Math.floor((f.top + f.height * f.b) / STRUCT_TILE); sz++) {
				for (let sx = Math.floor(f.left / STRUCT_TILE); sx <= Math.floor((f.left + f.width * f.b) / STRUCT_TILE); sx++) {
					const key = `st|${seedKey}|${set}|${sx}|${sz}`;
					const list = structures.get(key);
					if (!list) {
						wanted.add(key);
						const d = centerDistance(f, (sx + 0.5) * STRUCT_TILE, (sz + 0.5) * STRUCT_TILE);
						requestStructures(key, set, sx, sz, d * 0.5 - 1);
						continue;
					}
					for (const s of list) {
						drawIconMarker(ctx, f, f.toX(s.x), f.toY(s.z), info.color, structureSprite(set, info.color, () => canvas?.redraw()));
						markers.push({
							sx: f.toX(s.x),
							sy: f.toY(s.z),
							text: `${info.label()} · ${s.structure ?? ''} · ${s.x}, ${s.z}`,
							pick: {
								title: info.label(),
								subtitle: prettyStructure(s.structure),
								icon: structureSpriteUrl(set, info.color),
								x: s.x,
								z: s.z
							}
						});
					}
				}
			}
		}
		if (tools.strongholds && tools.dimension === 'overworld') {
			requestStrongholds();
			for (const s of strongholds?.list ?? []) {
				drawIconMarker(ctx, f, f.toX(s.x), f.toY(s.z), '#7fffd4', structureSprite('strongholds', '#7fffd4', () => canvas?.redraw()));
				markers.push({
					sx: f.toX(s.x),
					sy: f.toY(s.z),
					text: `${m.tools_strongholds()} · ${s.x}, ${s.z}`,
					pick: {
						title: m.tools_strongholds(),
						subtitle: '',
						icon: structureSpriteUrl('strongholds', '#7fffd4'),
						x: s.x,
						z: s.z
					}
				});
			}
		}
		if (tools.mode === 'world' && tools.world) {
			for (const p of tools.world.players) {
				if (p.dimension !== `minecraft:${tools.dimension}`) continue;
				drawMarker(ctx, f, f.toX(p.x), f.toY(p.z), '#ffffff', 'player');
				markers.push({ sx: f.toX(p.x), sy: f.toY(p.z), text: `${m.tools_player()} · ${Math.floor(p.x)}, ${Math.floor(p.z)}` });
			}
		}
		hiddenSets = hidden;
	}

	function drawSelection(ctx: CanvasRenderingContext2D, f: Frame) {
		if (!selected) {
			panelPos = null;
			return;
		}
		const sx = f.toX(selected.x);
		const sy = f.toY(selected.z);
		ctx.strokeStyle = colors.brand;
		ctx.lineWidth = 2;
		ctx.beginPath();
		ctx.arc(sx, sy, 17, 0, Math.PI * 2);
		ctx.stroke();
		const p = panelPos;
		if (!p || p.sx !== sx || p.sy !== sy || p.w !== f.width || p.h !== f.height) {
			panelPos = { sx, sy, w: f.width, h: f.height };
		}
	}

	function biomeAt(x: number, z: number) {
		if (!main) return '';
		try {
			return biomeLabel(main.biomeIds(tools.dimension, x, z, 1, 1, 1)[0]);
		} catch {
			return '';
		}
	}

	function pick(x: number, z: number, sx: number, sy: number) {
		copied = '';
		let best: Marker | null = null;
		let bestDist = 15 ** 2;
		for (const mk of markers) {
			if (!mk.pick) continue;
			const d = (mk.sx - sx) ** 2 + (mk.sy - sy) ** 2;
			if (d < bestDist) {
				bestDist = d;
				best = mk;
			}
		}
		selected = best?.pick ? { ...best.pick, biome: biomeAt(best.pick.x, best.pick.z) } : null;
		canvas?.redraw();
	}

	function closePanel() {
		selected = null;
		canvas?.redraw();
	}

	async function copyText(text: string, label: string) {
		try {
			await navigator.clipboard.writeText(text);
			copied = label;
		} catch {
			copied = '';
		}
	}

	function prettyStructure(id: string | null) {
		return (id ?? '').replace('minecraft:', '').replaceAll('_', ' ');
	}

	function drawFocus(ctx: CanvasRenderingContext2D, f: Frame) {
		const focus = tools.focus;
		if (!focus) return;
		const fx = f.toX(focus.x);
		const fy = f.toY(focus.z);
		const chunkPx = 16 / f.b;
		ctx.strokeStyle = 'rgba(92, 230, 110, 0.95)';
		ctx.lineWidth = 1.5;
		for (const [cx, cz] of focus.chunks ?? []) ctx.strokeRect(f.toX(cx * 16), f.toY(cz * 16), chunkPx, chunkPx);
		if (focus.ring) {
			ctx.lineWidth = 2;
			ctx.strokeStyle = colors.brand;
			ctx.setLineDash([6, 4]);
			for (const r of [focus.ring.max, focus.ring.min]) {
				ctx.beginPath();
				ctx.arc(fx, fy, r / f.b, 0, Math.PI * 2);
				ctx.stroke();
			}
			ctx.setLineDash([]);
		}
		for (const p of focus.points ?? []) {
			drawMarker(ctx, f, f.toX(p.x), f.toY(p.z), colors.brand, 'ring');
			markers.push({ sx: f.toX(p.x), sy: f.toY(p.z), text: p.label });
		}
		ctx.strokeStyle = colors.ink;
		ctx.lineWidth = 2;
		ctx.beginPath();
		ctx.moveTo(fx - 9, fy);
		ctx.lineTo(fx + 9, fy);
		ctx.moveTo(fx, fy - 9);
		ctx.lineTo(fx, fy + 9);
		ctx.stroke();
		markers.push({ sx: fx, sy: fy, text: `${focus.label} · ${Math.round(focus.x)}, ${Math.round(focus.z)}` });
	}

	function describe(x: number, z: number, sx: number, sy: number) {
		if (!tools.seed) return [];
		const biome = biomeAt(x, z);
		let best = '';
		let bestDist = 100;
		for (const mk of markers) {
			const d = (mk.sx - sx) ** 2 + (mk.sy - sy) ** 2;
			if (d < bestDist) {
				bestDist = d;
				best = mk.text;
			}
		}
		const coords = `X ${x} · Z ${z} · ${m.tools_chunk()} ${x >> 4}, ${z >> 4}${biome ? ` · ${biome}` : ''}`;
		return best ? [best, coords] : [coords];
	}

	function goto() {
		const nums = gotoText.match(/-?\d+(\.\d+)?/g)?.map(Number) ?? [];
		if (nums.length < 2) return;
		// 「x z」或「x y z」。
		const [x, z] = nums.length >= 3 ? [nums[0], nums[2]] : [nums[0], nums[1]];
		tools.seedView.x = x;
		tools.seedView.z = z;
		tools.focus = { x, z, label: m.tools_goto_target() };
	}

	function toggleSet(name: string, on: boolean) {
		tools.sets = on ? [...tools.sets, name] : tools.sets.filter((s) => s !== name);
	}

	$effect(() => {
		void [seedKey, tools.dimension];
		selected = null;
	});

	const otherDimension = $derived.by(() => {
		if (!selected) return null;
		if (tools.dimension === 'overworld')
			return { label: m.tools_info_in_nether(), x: Math.floor(selected.x / 8), z: Math.floor(selected.z / 8) };
		if (tools.dimension === 'the_nether')
			return { label: m.tools_info_in_overworld(), x: selected.x * 8, z: selected.z * 8 };
		return null;
	});

	onMount(() => {
		colors = themeColors();
		canvas?.redraw();
	});

	onDestroy(() => {
		cache.clear();
		pool.destroy();
		terrain?.dispose();
	});
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && selected && closePanel()} />

<div class="flex min-h-0 flex-1 flex-col gap-2">
	{#if !compact}
	<div class="flex shrink-0 flex-wrap items-center gap-1.75">
		<Select class="w-32" bind:value={tools.dimension} aria-label={m.tools_dimension()}>
			{#each dims as d (d.id)}
				<option value={d.id}>{d.label()}</option>
			{/each}
		</Select>
		<form
			class="flex min-w-0 items-center gap-1.75"
			onsubmit={(e) => {
				e.preventDefault();
				goto();
			}}
		>
			<TextInput class="w-44" bind:value={gotoText} placeholder={m.tools_goto_placeholder()} aria-label={m.tools_goto()} />
			<Button type="submit" icon="arrow">{m.tools_goto()}</Button>
		</form>
		<Button
			icon="rerender"
			onclick={() => {
				tools.seedView.x = 0;
				tools.seedView.z = 0;
				tools.focus = null;
			}}
		>
			{m.tools_origin()}
		</Button>
		<span class="flex-1"></span>
		{#if pendingCount > 0}
			<span class="text-faint">{m.tools_tiles_pending({ count: pendingCount })}</span>
		{/if}
		<Button icon="minus" onclick={() => canvas?.zoomBy(1)} aria-label={m.tools_zoom_out()} title={m.tools_zoom_out()}></Button>
		<span class="min-w-24 text-center whitespace-nowrap text-muted tabular-nums">{scaleText(tools.seedView.zoom)}</span>
		<Button icon="plus" onclick={() => canvas?.zoomBy(-1)} aria-label={m.tools_zoom_in()} title={m.tools_zoom_in()}></Button>
		<Button icon="layers" onclick={() => (layersOpen = !layersOpen)} aria-expanded={layersOpen}>{m.tools_layers()}</Button>
	</div>
	{/if}

	<div class="relative flex min-h-0 flex-1 overflow-hidden rounded-lg border border-line">
		<MapCanvas bind:this={canvas} view={tools.seedView} {draw} {describe} onblockclick={pick}>
			{#snippet overlay()}
				{#if selected && panelPos}
					{@const flipX = panelPos.sx > panelPos.w - 300}
					{@const flipY = panelPos.sy > panelPos.h - 280}
					<div
						class="absolute z-10 w-64 rounded-lg border border-line bg-raised p-3 text-[12.5px] shadow-md"
						style:left="{panelPos.sx + (flipX ? -24 : 24)}px"
						style:top="{panelPos.sy + (flipY ? 12 : -12)}px"
						style:transform="translate({flipX ? '-100%' : '0'}, {flipY ? '-100%' : '0'})"
						role="dialog"
						aria-label={selected.title}
					>
						<div class="mb-2 flex items-start gap-2">
							{#if selected.icon}
								<img src={selected.icon} alt="" class="size-7 shrink-0" />
							{/if}
							<div class="min-w-0 flex-1">
								<div class="truncate font-semibold">{selected.title}</div>
								{#if selected.subtitle}
									<div class="truncate text-faint">{selected.subtitle}</div>
								{/if}
							</div>
							<button class="text-faint hover:text-ink" onclick={closePanel} aria-label={m.common_close()}>
								<Icon name="close" size={14} />
							</button>
						</div>
						<dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 tabular-nums">
							<dt class="text-faint">{m.tools_info_coords()}</dt>
							<dd>X {selected.x} · Z {selected.z}</dd>
							{#if otherDimension}
								<dt class="text-faint">{otherDimension.label}</dt>
								<dd>X {otherDimension.x} · Z {otherDimension.z}</dd>
							{/if}
							{#if selected.biome}
								<dt class="text-faint">{m.tools_info_biome()}</dt>
								<dd class="truncate">{selected.biome}</dd>
							{/if}
						</dl>
						<div class="mt-2.5 flex flex-wrap items-center gap-1.5">
							<Button icon="copy" onclick={() => copyText(`${selected!.x} ${selected!.z}`, 'coords')}>
								{m.tools_info_copy_coords()}
							</Button>
							<Button icon="copy" onclick={() => copyText(`/tp @s ${selected!.x} ~ ${selected!.z}`, 'tp')}>
								{m.tools_info_copy_tp()}
							</Button>
							{#if copied}
								<span class="text-brand">{m.tools_info_copied()}</span>
							{/if}
						</div>
					</div>
				{/if}
				{#if !tools.seed}
					<div class="pointer-events-none absolute inset-0 grid place-items-center text-muted">{m.tools_no_seed()}</div>
				{/if}
				{#if hiddenSets && !compact}
					<div class="pointer-events-none absolute top-2 left-2 rounded-md bg-scrim px-2.5 py-1 text-[12px] text-muted">
						{m.tools_zoom_for_structures()}
					</div>
				{/if}
				{#if compact}
					<div class="absolute top-2 right-2 flex items-center gap-1">
						<span class="rounded-md bg-scrim px-2 py-1 text-[11.5px] whitespace-nowrap text-muted tabular-nums">
							{scaleText(tools.seedView.zoom)}
						</span>
						<button class={mapButton} onclick={() => canvas?.zoomBy(1)} aria-label={m.tools_zoom_out()} title={m.tools_zoom_out()}>
							<Icon name="minus" size={14} />
						</button>
						<button class={mapButton} onclick={() => canvas?.zoomBy(-1)} aria-label={m.tools_zoom_in()} title={m.tools_zoom_in()}>
							<Icon name="plus" size={14} />
						</button>
						{#if onexpand}
							<button class={mapButton} onclick={onexpand} aria-label={m.tools_open_seed_map()} title={m.tools_open_seed_map()}>
								<Icon name="shortcut" size={14} />
							</button>
						{/if}
					</div>
				{/if}
			{/snippet}
		</MapCanvas>

		{#if layersOpen}
			<aside class="flex w-56 shrink-0 flex-col gap-0.5 overflow-y-auto border-l border-line bg-window px-3 py-2.5">
				<span class="mb-1 flex items-center justify-between font-semibold">
					{m.tools_layers()}
					<button class="text-faint hover:text-ink" onclick={() => (layersOpen = false)} aria-label={m.common_cancel()}>
						<Icon name="close" size={14} />
					</button>
				</span>
				{#if terrainSource}
					<CheckRow compact><input type="checkbox" bind:checked={tools.terrain} />{m.tools_layer_terrain()}</CheckRow>
				{/if}
				{#if tools.dimension === 'overworld'}
					<CheckRow compact><input type="checkbox" bind:checked={tools.slime} />{m.tools_layer_slime()}</CheckRow>
					<CheckRow compact><input type="checkbox" bind:checked={tools.strongholds} />{m.tools_strongholds()}</CheckRow>
				{/if}
				{#each available.filter((n) => STRUCTURE_SETS[n].dimension === tools.dimension) as name (name)}
					<CheckRow compact>
						<input type="checkbox" checked={tools.sets.includes(name)} onchange={(e) => toggleSet(name, e.currentTarget.checked)} />
						<img src={structureSpriteUrl(name, STRUCTURE_SETS[name].color)} alt="" class="size-5 shrink-0" />
						{setLabel(name)}
					</CheckRow>
				{/each}
			</aside>
		{/if}
	</div>
</div>
