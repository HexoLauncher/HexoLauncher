<script lang="ts">
	import { Button, Select, Spinner, TextInput, ToolButton } from '$lib/ui';
	import * as api from '$lib/api';
	import type { WorldEntry } from '$lib/api';
	import { app } from '$lib/state.svelte';
	import { mainSeedMap } from '$lib/tools/seedmap';
	import { tools, VERSIONS, worldRef } from '$lib/tools/state.svelte';
	import { m } from '$lib/paraglide/messages.js';

	/** 只顯示世界選擇（地圖檢視用）。 */
	let { worldOnly = false }: { worldOnly?: boolean } = $props();

	let worlds = $state<WorldEntry[]>([]);
	let loadingWorlds = $state(false);
	let loadingInfo = $state(false);

	const instance = $derived(app.instances.find((i) => i.id === tools.instanceId) ?? null);
	const showWorld = $derived(worldOnly || tools.mode === 'world');

	$effect(() => {
		if (!tools.instanceId && app.instances.length) tools.instanceId = app.selectedId ?? app.instances[0].id;
	});

	let worldSeq = 0;
	$effect(() => {
		const id = tools.instanceId;
		if (!id || !app.baseDir) return;
		const current = ++worldSeq;
		loadingWorlds = true;
		api
			.listWorlds(app.baseDir, id)
			.then((found) => {
				if (current !== worldSeq) return;
				worlds = found.sort((a, b) => b.lastPlayed - a.lastPlayed);
				if (!worlds.some((w) => w.folder === tools.worldFolder)) tools.worldFolder = worlds[0]?.folder ?? '';
			})
			.catch((e) => {
				if (current !== worldSeq) return;
				worlds = [];
				tools.worldFolder = '';
				tools.worldError = api.errorMessage(e);
			})
			.finally(() => {
				if (current === worldSeq) loadingWorlds = false;
			});
	});

	// 選到的世界：讀種子、維度、玩家位置。
	let infoSeq = 0;
	$effect(() => {
		void [tools.instanceId, tools.worldFolder];
		const ref = worldRef(app.baseDir);
		const current = ++infoSeq;
		tools.world = null;
		tools.worldError = '';
		if (!ref) return;
		loadingInfo = true;
		api
			.toolsWorldInfo(ref)
			.then((info) => {
				if (current === infoSeq) tools.world = info;
			})
			.catch((e) => {
				if (current === infoSeq) tools.worldError = api.errorMessage(e);
			})
			.finally(() => {
				if (current === infoSeq) loadingInfo = false;
			});
	});

	// 決定目前生效的種子。
	let seedSeq = 0;
	$effect(() => {
		const { mode, manualSeed, manualVersion, world, worldError } = tools;
		const current = ++seedSeq;
		tools.sourceError = '';
		tools.sourceNote = '';
		if (mode === 'manual') {
			applySeed(current, manualSeed.trim(), manualVersion, '');
			return;
		}
		if (worldError) {
			tools.seed = '';
			tools.sourceError = worldError;
			return;
		}
		if (!world) {
			tools.seed = '';
			return;
		}
		if (!world.version || !world.seed) {
			tools.seed = '';
			tools.sourceError = m.tools_world_too_old();
			return;
		}
		applySeed(current, world.seed, world.version, m.tools_world_seed({ seed: world.seed, version: world.version }));
	});

	/** 先在 wasm 建一次，確認種子和版本有效，再交給地圖與搜尋。 */
	async function applySeed(current: number, seed: string, version: string, note: string) {
		if (!seed) {
			tools.seed = '';
			return;
		}
		try {
			const map = await mainSeedMap(seed, version);
			if (current !== seedSeq) return;
			tools.seed = map.seed;
			tools.version = map.version;
			tools.sourceNote = note || m.tools_manual_seed({ seed: map.seed });
		} catch (e) {
			if (current !== seedSeq) return;
			tools.seed = '';
			tools.sourceError = e instanceof Error ? e.message : String(e);
		}
	}

	async function copySeed() {
		try {
			await navigator.clipboard.writeText(tools.seed);
		} catch {
			// 剪貼簿不可用時忽略
		}
	}

	const note = $derived.by(() => {
		if (!worldOnly) return { error: tools.sourceError, text: tools.seed ? tools.sourceNote : '' };
		if (tools.worldError) return { error: tools.worldError, text: '' };
		const w = tools.world;
		return { error: '', text: w ? m.tools_world_summary({ name: w.name, dims: w.dimensions.length }) : '' };
	});
</script>

<div class="flex shrink-0 flex-wrap items-center gap-1.75">
	{#if !worldOnly}
		<div class="flex shrink-0 gap-0.5 rounded-md bg-btn p-0.5" role="group" aria-label={m.tools_seed_source()}>
			<ToolButton layout="bare" class="px-2.5 py-1" active={tools.mode === 'world'} onclick={() => (tools.mode = 'world')}>
				{m.tools_source_world()}
			</ToolButton>
			<ToolButton layout="bare" class="px-2.5 py-1" active={tools.mode === 'manual'} onclick={() => (tools.mode = 'manual')}>
				{m.tools_source_manual()}
			</ToolButton>
		</div>
	{/if}

	{#if showWorld}
		<Select
			class="max-w-60 min-w-0 flex-1"
			bind:value={tools.instanceId}
			disabled={app.instances.length === 0}
			aria-label={m.blueprint_instance()}
		>
			{#if app.instances.length === 0}
				<option value="">{m.blueprint_no_instances()}</option>
			{/if}
			{#each app.instances as inst (inst.id)}
				<option value={inst.id}>{inst.name} ({inst.versionId})</option>
			{/each}
		</Select>
		<Select
			class="max-w-60 min-w-0 flex-1"
			bind:value={tools.worldFolder}
			disabled={!instance || worlds.length === 0}
			aria-label={m.tools_world()}
		>
			{#if worlds.length === 0}
				<option value="">{loadingWorlds ? m.common_loading() : m.tools_no_worlds()}</option>
			{/if}
			{#each worlds as w (w.folder)}
				<option value={w.folder}>{w.name}{w.version ? ` (${w.version})` : ''}</option>
			{/each}
		</Select>
	{:else}
		<TextInput class="w-60 min-w-0" bind:value={tools.manualSeed} placeholder={m.tools_seed_placeholder()} aria-label={m.tools_seed()} />
		<Select class="w-28" bind:value={tools.manualVersion} aria-label={m.tools_version()}>
			{#each VERSIONS as v (v)}
				<option value={v}>{v === '26' ? '26.x' : `${v}+`}</option>
			{/each}
		</Select>
	{/if}

	<span class="flex min-w-0 flex-1 items-center gap-1.5 truncate text-muted">
		{#if loadingInfo && showWorld}<Spinner />{/if}
		{#if note.error}
			<span class="truncate text-danger" title={note.error}>{note.error}</span>
		{:else if note.text}
			<span class="truncate" title={note.text}>{note.text}</span>
		{/if}
	</span>
	{#if tools.seed && !worldOnly}
		<Button icon="copy" onclick={copySeed} title={m.tools_copy_seed()} aria-label={m.tools_copy_seed()}></Button>
	{/if}
</div>
