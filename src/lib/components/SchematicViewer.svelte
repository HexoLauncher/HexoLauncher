<script lang="ts">
	import { onMount } from 'svelte';
	import { Progress, Select, Slider, Spinner } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import * as api from '$lib/api';
	import type { Blueprint, InstanceView, PackEntry } from '$lib/api';
	import { isRelease } from '$lib/blueprints.svelte';
	import {
		App,
		VanillaAssets,
		base64ToBytes,
		createApp,
		destroyApp,
		fetchLocal,
		loadVanilla,
		minecraftVersions,
		parseReport,
		type MeshReport
	} from '$lib/schematic/viewer';
	import { m } from '$lib/paraglide/messages.js';

	let {
		schematic,
		preferInstance = null,
		compact = false,
		title = $bindable('')
	}: {
		schematic: { kind: 'remote'; id: string; blueprint?: Blueprint } | { kind: 'local'; path: string };
		preferInstance?: string | null;
		compact?: boolean;
		title?: string;
	} = $props();

	let canvas = $state<HTMLCanvasElement>();
	let viewer: App | null = null;

	let fatal = $state('');
	let status = $state<string>(m.viewer_status_starting());
	let working = $state(true);
	let report = $state<MeshReport | null>(null);
	let showWarnings = $state(false);
	// svelte-ignore state_referenced_locally
	let panelOpen = $state(!compact);

	let baseDir = '';
	let instances = $state<InstanceView[]>([]);
	let mojangVersions = $state<string[]>([]);
	let source = $state('');
	let packGroups = $state<{ name: string; packs: PackEntry[] }[]>([]);
	let active = $state<string[]>([]);
	let picking = $state('');
	let dragging = $state<string | null>(null);
	let dragFrom = -1;
	const rows: Record<string, HTMLElement> = {};
	let layerCount = $state(0);
	let layer = $state(0);
	let time = $state('day');
	const times = [
		{ value: 'day', label: () => m.viewer_time_day() },
		{ value: 'sunset', label: () => m.viewer_time_sunset() },
		{ value: 'night', label: () => m.viewer_time_night() }
	];
	let meshing = $state<number | null>(null);


	const vanillaCache: Record<string, VanillaAssets> = {};
	let applySeq = 0;

	const allPacks = $derived(packGroups.flatMap((g) => g.packs));
	const activePacks = $derived(active.map((path) => allPacks.find((p) => p.path === path)).filter((p) => !!p));

	async function show(result: Promise<unknown>) {
		const parsed = parseReport((await result) as string | null);
		meshing = null;
		if (!parsed || !viewer) return;
		report = parsed;
		layerCount = viewer.layer_count;
		if (layer >= layerCount) layer = 0;
	}

	onMount(() => {
		let disposed = false;
		start(() => disposed);
		return () => {
			disposed = true;
			applySeq++;
			if (viewer) destroyApp(viewer);
			viewer = null;
		};
	});

	async function start(disposed: () => boolean) {
		try {
			const created = await createApp(canvas!);

			if (disposed()) return destroyApp(created);
			viewer = created;
			viewer.set_progress((fraction: number) => (meshing = fraction < 1 ? fraction : null));
		} catch (e) {
			fatal = m.viewer_error_gpu({ message: api.errorMessage(e) });
			working = false;
			return;
		}

		let suggested: string | null = null;
		const failed = (e: unknown) => (fatal = m.viewer_error_load({ message: api.errorMessage(e) }));
		try {
			baseDir = (await api.resolvePaths()).baseDir;
			instances = await api.listInstances(baseDir).catch(() => []);
			loadPacks();

			status = m.viewer_status_reading();
			if (schematic.kind === 'remote') {
				const blueprint = schematic.blueprint ?? (await api.blueprintGet(schematic.id));
				title = blueprint.name;
				suggested = blueprint.render.mcVersion ?? blueprint.mcVersions.max ?? blueprint.mcVersions.min;

				const data = await api.blueprintPreview(blueprint.id, blueprint.format);
				if (!viewer) return;
				show(viewer.load_preview(base64ToBytes(data))).catch(failed);
			} else {
				const path = schematic.path;
				title = path.split(/[\\/]/).pop() ?? path;
				suggested = instances.find((i) => i.id === preferInstance)?.versionId ?? null;
				const data = await api.readSchematic(path);
				if (!viewer) return;
				show(viewer.load_schematic(base64ToBytes(data))).catch(failed);
			}
		} catch (e) {
			failed(e);
			working = false;
			return;
		}

		const preferred = instances.find((i) => i.id === preferInstance)?.versionId;
		if (preferred && isRelease(preferred)) suggested = preferred;
		if (suggested && isRelease(suggested)) source = suggested;
		minecraftVersions()
			.then((list) => (mojangVersions = list))
			.catch(() => {});
		if (!source) source = (await minecraftVersions().catch(() => [] as string[]))[0] ?? '';
		await applySource();
	}

	async function loadPacks() {
		const groups = await Promise.all(
			instances.map(async (inst) => ({
				name: inst.name,
				packs: await api.listResourcePacks(baseDir, inst.id).catch(() => [] as PackEntry[])
			}))
		);
		packGroups = groups.filter((g) => g.packs.length);
	}

	async function vanilla(version: string, current: number) {
		let assets = vanillaCache[version];
		if (!assets) {
			assets = await loadVanilla(version, (msg) => {
				if (current === applySeq) status = msg;
			});
			vanillaCache[version] = assets;
		}
		return assets;
	}

	async function task(job: (viewer: App, current: number) => Promise<void>) {
		if (!viewer) return;
		const current = ++applySeq;
		working = true;
		try {
			await job(viewer, current);
			if (current === applySeq) status = '';
		} catch (e) {
			if (current === applySeq) status = m.viewer_error_assets({ message: api.errorMessage(e) });
		} finally {
			if (current === applySeq) working = false;
		}
	}

	function packName(path: string) {
		return allPacks.find((p) => p.path === path)?.name ?? path;
	}

	async function packBytes(path: string) {
		status = m.viewer_status_pack({ name: packName(path) });
		return fetchLocal(await api.packBlockAssets(path));
	}

	function applySource() {
		if (!source) return;
		return task(async (_, current) => {
			const base = await vanilla(source, current);
			if (current !== applySeq) return;


			const lowestFirst = [...active].reverse();
			const extra: Uint8Array[] = [];
			for (const path of lowestFirst) {
				extra.push(await packBytes(path));
				if (current !== applySeq) return;
			}
			if (!viewer) return;

			viewer.clear_packs();
			let result = viewer.set_vanilla(base);
			lowestFirst.forEach((path, i) => {
				result = viewer!.add_pack(packName(path), extra[i]);
			});
			await show(result);
		});
	}

	function addPack(path: string) {
		picking = '';
		if (!path || active.includes(path)) return;
		return task(async (_, current) => {
			const bytes = await packBytes(path);
			if (current !== applySeq || !viewer) return;
			active = [path, ...active];
			await show(viewer.add_pack(packName(path), bytes));
		});
	}

	function removePack(path: string) {
		const index = active.indexOf(path);
		active = active.filter((p) => p !== path);
		return task((viewer) => show(viewer.remove_pack(index)));
	}

	function startDrag(e: PointerEvent, path: string) {
		if (working || e.button !== 0) return;
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		dragging = path;
		dragFrom = active.indexOf(path);
	}

	function moveDrag(e: PointerEvent) {
		if (!dragging) return;
		const target = active.find((path) => {
			const rect = rows[path]?.getBoundingClientRect();
			return rect && e.clientY >= rect.top && e.clientY < rect.bottom;
		});
		if (!target || target === dragging) return;
		const next = active.filter((path) => path !== dragging);
		next.splice(active.indexOf(target), 0, dragging);
		active = next;
	}

	function endDrag() {
		if (!dragging) return;
		const to = active.indexOf(dragging);
		const from = dragFrom;
		dragging = null;
		if (to !== from) task((viewer) => show(viewer.move_pack(from, to)));
	}

	function setLayer(value: number) {
		layer = value >= layerCount ? 0 : value;
		viewer?.set_layer(layer || undefined);
	}

	function setTime(value: string) {
		time = value;
		viewer?.set_time(value);
	}
</script>

<div class="viewer relative size-full overflow-hidden bg-backdrop text-ink">
	<canvas
		bind:this={canvas}
		class="block size-full touch-none outline-none"
		tabindex="0"
		aria-label={m.viewer_canvas_label()}
	></canvas>

	{#if fatal}
		<div class="absolute inset-0 grid place-items-center bg-window px-8 text-center">
			<div class="flex max-w-120 flex-col items-center gap-3">
				<Icon name="blocks" size={36} />
				<p class="leading-[1.75] text-danger">{fatal}</p>
			</div>
		</div>
	{/if}

	{#if layerCount > 1}
		<div
			class="absolute top-3 bottom-16 left-3 flex w-12 flex-col items-center gap-1.5 rounded-xl border border-line bg-window py-3 shadow-lg"
		>
			<span class="text-[11px] text-muted">{m.viewer_layers()}</span>
			<span class="font-semibold tabular-nums">{layer || layerCount}</span>
			<div class="flex min-h-0 flex-1 py-2">
				<Slider
					orientation="vertical"
					ariaLabel={m.viewer_layers()}
					min={1}
					max={layerCount}
					value={layer || layerCount}
					onchange={setLayer}
				/>
			</div>
			<span class="text-[11px] text-muted tabular-nums">/ {layerCount}</span>
		</div>
	{/if}

	<div
		class="absolute bottom-3 left-3 flex gap-0.5 rounded-xl border border-line bg-window p-1 shadow-lg"
		role="radiogroup"
		aria-label={m.viewer_lighting()}
	>
		{#each times as option (option.value)}
			<button
				class="flex items-center gap-1.5 rounded-lg px-2.5 py-1 {time === option.value
					? 'bg-brand text-brand-ink'
					: 'text-muted hover:bg-btn-hi hover:text-ink'}"
				role="radio"
				aria-checked={time === option.value}
				disabled={!!fatal}
				onclick={() => setTime(option.value)}
			>
				{#if option.value !== 'sunset'}
					<Icon name={option.value === 'day' ? 'sun' : 'moon'} size={13} />
				{/if}
				{option.label()}
			</button>
		{/each}
	</div>

	<div
		class="absolute top-3 right-3 flex max-h-[calc(100%-24px)] flex-col {panelOpen
			? compact
				? 'w-64'
				: 'w-76'
			: 'max-w-64'} overflow-hidden rounded-xl border border-line bg-window shadow-lg"
	>
		<header class="flex shrink-0 items-center gap-2 px-3.5 py-2.5">
			<Icon name="blocks" size={15} />
			<h1 class="min-w-0 flex-1 truncate font-semibold" title={title}>{title || '…'}</h1>
			{#if !panelOpen && (working || meshing !== null)}<Spinner />{/if}
			<button
				class="grid size-6 place-items-center rounded-md text-muted hover:bg-btn-hi hover:text-ink"
				aria-label={panelOpen ? m.viewer_collapse() : m.viewer_expand()}
				aria-expanded={panelOpen}
				onclick={() => (panelOpen = !panelOpen)}
			>
				<Icon name={panelOpen ? 'up' : 'down'} size={14} />
			</button>
		</header>

		{#if panelOpen}
			<div class="flex min-h-0 flex-col gap-3 overflow-y-auto border-t border-line px-3.5 pt-3 pb-3.5">
				<label class="flex flex-col gap-1.5">
					<span class="text-muted">{m.viewer_textures()}</span>
					<Select bind:value={source} onchange={applySource} disabled={!!fatal}>
						{#if !source}<option value="">{m.common_loading_versions()}</option>{/if}
						{#if source && !mojangVersions.includes(source)}<option value={source}>{source}</option>{/if}
						{#each mojangVersions as v (v)}
							<option value={v}>{v}</option>
						{/each}
					</Select>
				</label>

				<div class="flex flex-col gap-1.5">
					<span class="text-muted">{m.viewer_resource_packs()}</span>
					{#if packGroups.length}
						<Select bind:value={picking} onchange={() => addPack(picking)} disabled={working || !!fatal}>
							<option value="">{m.viewer_add_pack()}</option>
							{#each packGroups as group (group.name)}
								<optgroup label={group.name}>
									{#each group.packs as pack (pack.path)}
										<option value={pack.path} disabled={active.includes(pack.path)}>{pack.name}</option>
									{/each}
								</optgroup>
							{/each}
						</Select>
					{:else}
						<span class="text-muted">{m.viewer_no_packs()}</span>
					{/if}
					{#if activePacks.length}
						<ul class="flex flex-col">
							{#each activePacks as pack (pack.path)}
								<li
									bind:this={rows[pack.path]}
									class="flex items-center gap-1.5 rounded-md py-0.5 pr-0.5 {dragging === pack.path
										? 'bg-btn-hi'
										: ''}"
								>
									<button
										class="grid size-5 shrink-0 touch-none place-items-center rounded text-muted hover:text-ink disabled:opacity-40 {dragging ===
										pack.path
											? 'cursor-grabbing'
											: 'cursor-grab'}"
										aria-label={m.viewer_pack_drag({ name: pack.name })}
										disabled={working}
										onpointerdown={(e) => startDrag(e, pack.path)}
										onpointermove={moveDrag}
										onpointerup={endDrag}
										onpointercancel={endDrag}
									>
										<Icon name="menu" size={12} />
									</button>
									<span class="min-w-0 flex-1 truncate" title={pack.name}>{pack.name}</span>
									<button
										class="grid size-5 shrink-0 place-items-center rounded text-muted hover:bg-btn-hi hover:text-ink disabled:opacity-40"
										aria-label={m.viewer_remove_pack({ name: pack.name })}
										disabled={working}
										onclick={() => removePack(pack.path)}
									>
										<Icon name="close" size={12} />
									</button>
								</li>
							{/each}
						</ul>
						{#if activePacks.length > 1}
							<p class="text-[11px] leading-[1.6] text-muted">{m.viewer_packs_hint()}</p>
						{/if}
					{/if}
				</div>

				{#if meshing !== null}
					<div class="flex flex-col gap-1.5" role="status">
						<span class="text-muted">{m.viewer_status_meshing({ percent: Math.round(meshing * 100) })}</span>
						<Progress value={meshing} max={1} class="bg-btn" />
					</div>
				{/if}

				{#if status}
					<p class="flex items-start gap-2 leading-[1.6] text-muted" role="status">
						{#if working}<Spinner class="mt-0.5" />{/if}
						<span class="min-w-0 break-words">{status}</span>
					</p>
				{/if}

				{#if report}
					<div class="flex flex-col gap-1 text-[11.5px]">
						<p class="leading-[1.6] break-words text-muted">{report.summary}</p>
						{#if report.warnings.length}
							<button
								class="self-start text-warn hover:underline"
								onclick={() => (showWarnings = !showWarnings)}
							>
								{m.viewer_warnings({ count: report.warnings.length })}
							</button>
							{#if showWarnings}
								<ul class="max-h-40 overflow-y-auto font-mono leading-[1.6] text-muted">
									{#each report.warnings as line, i (i)}
										<li class="break-all">{line}</li>
									{/each}
								</ul>
							{/if}
						{/if}
					</div>
				{/if}

				<p class="text-[11px] leading-[1.6] text-muted">{m.viewer_controls()}</p>
			</div>
		{/if}
	</div>
</div>

<style>
	.viewer {
		color-scheme: dark;
		--color-backdrop: #0b0d12;
		--color-deep: #101217;
		--color-window: #181a20;
		--color-input-off: #14161b;
		--color-btn: #282b33;
		--color-btn-hi: #33363f;
		--color-line: #2c2f38;
		--color-line-hi: #3d414c;
		--color-ink: #e8e9ec;
		--color-muted: #aeb2bb;
		--color-faint: #8a8f99;
		--color-brand: #4f8cff;
		--color-brand-hi: #6c9fff;
		--color-brand-ink: #ffffff;
		--color-brand-soft: rgb(79 140 255 / 0.2);
		--color-brand-line: rgb(79 140 255 / 0.55);
		--color-knob-on: #0b0d12;
		--color-warn: #ffad5c;
		--color-danger: #ff7070;
	}
</style>
