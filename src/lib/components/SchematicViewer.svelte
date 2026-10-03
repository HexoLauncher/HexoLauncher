<script lang="ts">
	import { onMount } from 'svelte';
	import { Select, Spinner } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import * as api from '$lib/api';
	import type { Blueprint, InstanceView, PackEntry } from '$lib/api';
	import { compareVersions, isRelease } from '$lib/blueprints.svelte';
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
		/** 藍圖庫的藍圖（已經拿到資料就直接傳進來，不用再查一次），或本機檔案。 */
		schematic: { kind: 'remote'; id: string; blueprint?: Blueprint } | { kind: 'local'; path: string };
		/** 優先用這個實例的材質。 */
		preferInstance?: string | null;
		/** 嵌在其他畫面裡：面板預設收起來、窄一點。 */
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
	/** `instance:<id>` 用實例的 client jar；`mojang:<版本>` 從 Mojang 下載。 */
	let source = $state('');
	let packs = $state<PackEntry[]>([]);
	let packsFor = '';
	let enabledPacks = $state<string[]>([]);

	/** 切換材質來源時不重新下載。 */
	const vanillaCache: Record<string, VanillaAssets> = {};
	let applySeq = 0;

	const localInstances = $derived(
		instances.filter((i) => i.installed && isRelease(i.versionId))
	);
	const sourceInstance = $derived(
		source.startsWith('instance:') ? (instances.find((i) => `instance:${i.id}` === source) ?? null) : null
	);

	function show(result: string | undefined) {
		const parsed = parseReport(result);
		if (parsed) report = parsed;
	}

	onMount(() => {
		let disposed = false;
		start(() => disposed);
		return () => {
			disposed = true;
			applySeq++; // 讓還在跑的材質載入停下來
			if (viewer) destroyApp(viewer);
			viewer = null;
		};
	});

	async function start(disposed: () => boolean) {
		try {
			const created = await createApp(canvas!);
			// 建立期間元件已經卸載了。
			if (disposed()) return destroyApp(created);
			viewer = created;
		} catch (e) {
			fatal = m.viewer_error_gpu({ message: api.errorMessage(e) });
			working = false;
			return;
		}

		let suggested: string | null = null;
		try {
			baseDir = (await api.resolvePaths()).baseDir;
			instances = await api.listInstances(baseDir).catch(() => []);

			status = m.viewer_status_reading();
			if (schematic.kind === 'remote') {
				const blueprint = schematic.blueprint ?? (await api.blueprintGet(schematic.id));
				title = blueprint.name;
				suggested = blueprint.render.mcVersion ?? blueprint.mcVersions.max ?? blueprint.mcVersions.min;
				// 伺服器已經解析好方塊資料，不用下載整個檔案。
				const data = await api.blueprintPreview(blueprint.id, blueprint.format);
				if (!viewer) return;
				show(viewer.load_preview(base64ToBytes(data)));
			} else {
				const path = schematic.path;
				title = path.split(/[\\/]/).pop() ?? path;
				suggested = instances.find((i) => i.id === preferInstance)?.versionId ?? null;
				const data = await api.readSchematic(path);
				if (!viewer) return;
				show(viewer.load_schematic(base64ToBytes(data)));
			}
		} catch (e) {
			fatal = m.viewer_error_load({ message: api.errorMessage(e) });
			working = false;
			return;
		}

		source = pickSource(suggested);
		minecraftVersions()
			.then((list) => (mojangVersions = list))
			.catch(() => {});
		if (!source) {
			const latest = await minecraftVersions().catch(() => [] as string[]);
			source = latest[0] ? `mojang:${latest[0]}` : '';
		}
		await applySource();
	}

	function pickSource(suggested: string | null): string {
		const byId = localInstances.find((i) => i.id === preferInstance);
		if (byId) return `instance:${byId.id}`;
		if (suggested) {
			const match = localInstances.find((i) => i.versionId === suggested);
			if (match) return `instance:${match.id}`;
			if (isRelease(suggested)) return `mojang:${suggested}`;
		}
		const newest = [...localInstances].sort((a, b) => compareVersions(b.versionId, a.versionId))[0];
		return newest ? `instance:${newest.id}` : '';
	}

	async function useMojang(version: string, current: number) {
		let assets = vanillaCache[version];
		if (!assets) {
			assets = await loadVanilla(version, (msg) => {
				if (current === applySeq) status = msg;
			});
			vanillaCache[version] = assets;
		}
		if (current !== applySeq || !viewer) return;
		show(viewer.clear_packs());
		show(viewer.set_vanilla(assets));
	}

	async function applySource() {
		if (!viewer || !source) return;
		const current = ++applySeq;
		working = true;
		try {
			const inst = sourceInstance;
			if (!inst) {
				packs = [];
				packsFor = '';
				await useMojang(source.slice('mojang:'.length), current);
			} else {
				status = m.viewer_status_local_assets({ version: inst.versionId });
				const slim = await api.localBlockAssets(baseDir, inst.versionId, inst.id);
				if (current !== applySeq) return;

				if (packsFor !== inst.id) {
					packsFor = inst.id;
					packs = await api.listResourcePacks(baseDir, inst.id).catch(() => []);
					enabledPacks = [];
				}

				if (slim) {
					const bytes = await fetchLocal(slim);
					if (current !== applySeq) return;
					show(viewer.clear_packs());
					show(viewer.add_pack(bytes));
				} else {
					status = m.viewer_status_no_local_jar({ version: inst.versionId });
					await useMojang(inst.versionId, current);
				}

				// add_pack 會把新的包放在最上層，所以照清單順序由低到高加入。
				for (const path of packs.map((p) => p.path).filter((p) => enabledPacks.includes(p))) {
					status = m.viewer_status_pack({ name: packs.find((p) => p.path === path)?.name ?? '' });
					const packBytes = await fetchLocal(await api.packBlockAssets(path));
					if (current !== applySeq) return;
					show(viewer.add_pack(packBytes));
				}
			}
			if (current === applySeq) status = '';
		} catch (e) {
			if (current === applySeq) status = m.viewer_error_assets({ message: api.errorMessage(e) });
		} finally {
			if (current === applySeq) working = false;
		}
	}

	function togglePack(path: string) {
		enabledPacks = enabledPacks.includes(path)
			? enabledPacks.filter((p) => p !== path)
			: [...enabledPacks, path];
		applySource();
	}
</script>

<div class="relative size-full overflow-hidden bg-[#0b0d12]">
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

	<div
		class="absolute top-3 left-3 flex max-h-[calc(100%-24px)] flex-col {compact ? 'w-64' : 'w-76'} overflow-hidden rounded-xl border border-line bg-window/92 shadow-lg backdrop-blur-sm"
	>
		<header class="flex shrink-0 items-center gap-2 px-3.5 py-2.5">
			<Icon name="blocks" size={15} />
			<h1 class="min-w-0 flex-1 truncate font-semibold" title={title}>{title || '…'}</h1>
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
						{#if localInstances.length}
							<optgroup label={m.viewer_source_instances()}>
								{#each localInstances as inst (inst.id)}
									<option value="instance:{inst.id}">{inst.name} ({inst.versionId})</option>
								{/each}
							</optgroup>
						{/if}
						<optgroup label={m.viewer_source_mojang()}>
							{#if source.startsWith('mojang:') && !mojangVersions.includes(source.slice(7))}
								<option value={source}>{source.slice(7)}</option>
							{/if}
							{#each mojangVersions as v (v)}
								<option value="mojang:{v}">{v}</option>
							{/each}
						</optgroup>
					</Select>
				</label>

				{#if sourceInstance}
					<div class="flex flex-col gap-1.5">
						<span class="text-muted">{m.viewer_resource_packs()}</span>
						{#each packs as pack (pack.path)}
							<label class="flex cursor-pointer items-center gap-2">
								<input
									type="checkbox"
									checked={enabledPacks.includes(pack.path)}
									disabled={working}
									onchange={() => togglePack(pack.path)}
								/>
								<span class="min-w-0 truncate" title={pack.name}>{pack.name}</span>
							</label>
						{:else}
							<span class="text-faint">{m.viewer_no_packs()}</span>
						{/each}
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
								<ul class="max-h-40 overflow-y-auto font-mono leading-[1.6] text-faint">
									{#each report.warnings as line, i (i)}
										<li class="break-all">{line}</li>
									{/each}
								</ul>
							{/if}
						{/if}
					</div>
				{/if}

				<p class="text-[11px] leading-[1.6] text-faint">{m.viewer_controls()}</p>
			</div>
		{/if}
	</div>
</div>
