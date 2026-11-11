<script lang="ts">
	import { Select, Switch } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import CenterField from './CenterField.svelte';
	import FinderLayout from './FinderLayout.svelte';
	import NumberField from './NumberField.svelte';
	import Segmented from './Segmented.svelte';
	import * as api from '$lib/api';
	import type { ToolSpreadSet, ToolStructureCluster } from '$lib/api';
	import { report, reportError } from '$lib/state.svelte';
	import type { Dimension } from '$lib/tools/seedmap';
	import { estimateText } from '$lib/tools/draw';
	import {
		STRUCTURE_SETS,
		clusterWork,
		estimateSeconds,
		persistResults,
		recordRun,
		setLabel,
		showOnMap,
		tools,
		cancelSearch,
		finishSearch,
		searches,
		startSearch
	} from '$lib/tools/state.svelte';
	import { m } from '$lib/paraglide/messages.js';

	let sets = $state<ToolSpreadSet[]>([]);
	const form = tools.clusterForm;

	$effect(() => {
		const version = tools.version;
		api
			.toolsStructureSets(version)
			.then((found) => {
				sets = found;
				if (!found.some((s) => s.name === form.setName)) form.setName = found[0]?.name ?? '';
			})
			.catch((e) => reportError(m.tools_error_search(), e));
	});

	const counts = [
		{ value: '2', label: m.tools_count_double() },
		{ value: '3', label: m.tools_count_triple() },
		{ value: '4', label: m.tools_count_quad() }
	];
	const modes: { value: 'enclose' | 'coverage'; label: string }[] = [
		{ value: 'enclose', label: m.tools_scoring_enclose() },
		{ value: 'coverage', label: m.tools_scoring_coverage() }
	];
	const radii = [
		{ value: 100_000, label: '100k' },
		{ value: 1_000_000, label: '1M' },
		{ value: 5_000_000, label: '5M' },
		{ value: 30_000_000, label: m.tools_whole_world() }
	];

	const work = $derived(clusterWork(Number(form.radius), form.mode === 'coverage'));
	const estimate = $derived(estimateText(estimateSeconds('cluster', work)));

	function pickSet(name: string) {
		// 海底神殿通常只會部分落在範圍內，預設改用覆蓋率。
		if (name === 'ocean_monuments') form.mode = 'coverage';
	}

	async function run() {
		tools.clusterRunning = true;
		const taskId = startSearch('cluster');
		const started = performance.now();
		const runWork = work;
		const { setName, mode, afkRadius } = form;
		try {
			tools.clusterResults = await api.toolsStructureClusters({
				seed: tools.seed,
				version: tools.version,
				set: setName,
				centerX: Number(form.centerX),
				centerZ: Number(form.centerZ),
				radius: Number(form.radius),
				minCount: Number(form.minCount),
				ringMax: Number(afkRadius),
				minCoverage: mode === 'coverage' ? Number(form.minCoverage) / 100 : null,
				checkBiomes: form.checkBiomes
			}, taskId);
			tools.clusterResultMode = mode;
			tools.clusterResultSet = setName;
			tools.clusterResultRing = Number(afkRadius);
			const seconds = (performance.now() - started) / 1000;
			tools.clusterElapsed = seconds.toFixed(2);
			recordRun('cluster', runWork, seconds);
			persistResults();
			report(m.tools_status_found({ count: tools.clusterResults.length, seconds: tools.clusterElapsed }));
			if (tools.clusterResults.length) focus(tools.clusterResults[0], 0);
		} catch (e) {
			if (api.isCancelled(e)) report(m.tools_search_cancelled());
			else reportError(m.tools_error_search(), e);
		} finally {
			finishSearch('cluster');
			tools.clusterRunning = false;
		}
	}

	function focus(c: ToolStructureCluster, i: number) {
		tools.clusterSelected = i;
		const setName = tools.clusterResultSet;
		const dimension = (sets.find((s) => s.name === setName)?.dimension.replace('minecraft:', '') ??
			'overworld') as Dimension;
		tools.dimension = dimension;
		if (!tools.sets.includes(setName)) tools.sets = [...tools.sets, setName];
		showOnMap(
			{
				x: c.afkX,
				z: c.afkZ,
				label: m.tools_cluster_label({ count: c.structures.length, name: setLabel(setName) }),
				ring: { min: 24, max: tools.clusterResultRing },
				points: c.structures.map((h, k) => ({
					x: h.blockX,
					z: h.blockZ,
					label: `${h.structure ?? setLabel(h.set)} · ${h.blockX}, ${h.blockZ} · ${(c.memberCoverage[k] * 100).toFixed(0)}%`
				}))
			},
			Math.max(tools.clusterResultRing, c.requiredRadius)
		);
	}

	async function copy(c: ToolStructureCluster) {
		const text = `${Math.round(c.afkX)} ~ ${Math.round(c.afkZ)}`;
		try {
			await navigator.clipboard.writeText(text);
			report(m.tools_copied({ text }));
		} catch {
			// 剪貼簿不可用時忽略
		}
	}

	function clear() {
		tools.clusterResults = [];
		tools.clusterElapsed = '';
		tools.focus = null;
		tools.clusterSelected = -1;
		persistResults();
	}
</script>

<FinderLayout
	{estimate}
	running={tools.clusterRunning}
	canSearch={!!tools.seed && !!form.setName}
	count={tools.clusterResults.length}
	elapsed={tools.clusterElapsed}
	onsearch={run}
	onclear={clear}
	search={searches.cluster}
	oncancel={() => cancelSearch('cluster')}
>
	{#snippet settings()}
		<section class="flex flex-col gap-2.5">
			<h3 class="text-[11.5px] font-semibold tracking-wide text-faint">{m.tools_section_target()}</h3>
			<div class="flex flex-col gap-1">
				<label for="cluster-set" class="text-[12px] font-medium text-muted">{m.tools_structure()}</label>
				<Select id="cluster-set" class="w-full" bind:value={form.setName} onchange={() => pickSet(form.setName)}>
					{#each sets as s (s.name)}
						<option value={s.name}>{setLabel(s.name)}</option>
					{/each}
				</Select>
			</div>
			<div class="flex flex-col gap-1">
				<span class="text-[12px] font-medium text-muted">{m.tools_min_count()}</span>
				<Segmented label={m.tools_min_count()} options={counts} bind:value={form.minCount} stretch />
			</div>
			<div class="flex flex-col gap-1">
				<span class="text-[12px] font-medium text-muted">{m.tools_scoring()}</span>
				<Segmented label={m.tools_scoring()} options={modes} bind:value={form.mode} stretch />
				<span class="text-[11.5px] leading-[1.5] text-faint">
					{form.mode === 'coverage' ? m.tools_scoring_coverage_hint() : m.tools_scoring_enclose_hint()}
				</span>
			</div>
			{#if form.mode === 'coverage'}
				<NumberField label={m.tools_min_coverage()} unit="%" min={1} max={100} bind:value={form.minCoverage} />
			{/if}
		</section>

		<section class="flex flex-col gap-2.5">
			<h3 class="text-[11.5px] font-semibold tracking-wide text-faint">{m.tools_section_area()}</h3>
			<CenterField bind:x={form.centerX} bind:z={form.centerZ} />
			<NumberField
				label={m.tools_search_radius()}
				unit={m.tools_unit_blocks()}
				min={512}
				max={30_000_000}
				presets={radii}
				bind:value={form.radius}
			/>
		</section>

		<section class="flex flex-col gap-2.5">
			<h3 class="text-[11.5px] font-semibold tracking-wide text-faint">{m.tools_section_afk()}</h3>
			<NumberField
				label={m.tools_afk_radius()}
				unit={m.tools_unit_blocks()}
				min={16}
				max={256}
				hint={m.tools_afk_radius_hint()}
				bind:value={form.afkRadius}
			/>
			<Switch label={m.tools_check_biomes()} bind:checked={form.checkBiomes} />
			<span class="-mt-1 text-[11.5px] leading-[1.5] text-faint">{m.tools_cluster_note()}</span>
		</section>
	{/snippet}

	{#snippet results()}
		{@const coverage = tools.clusterResultMode === 'coverage'}
		{@const color = STRUCTURE_SETS[tools.clusterResultSet]?.color ?? 'var(--color-brand)'}
		{#each tools.clusterResults as c, i (`${c.afkX},${c.afkZ},${c.structures.length}`)}
			<div
				class="flex items-center gap-3 rounded-md px-2.5 py-2 transition-colors duration-150
					{tools.clusterSelected === i ? 'bg-brand-soft shadow-[inset_2px_0_0_var(--color-brand)]' : 'hover:bg-hover'}"
			>
				<button class="flex min-w-0 flex-1 items-center gap-3 text-left" onclick={() => focus(c, i)}>
					<span
						class="grid h-6 w-8 shrink-0 place-items-center rounded-md text-[12px] font-semibold text-black/80"
						style:background={color}
					>
						{c.structures.length}×
					</span>
					<span class="flex min-w-0 flex-1 flex-col gap-1">
						<span class="truncate font-medium tabular-nums">
							{m.tools_afk_at({ x: Math.round(c.afkX), z: Math.round(c.afkZ) })}
						</span>
						<span class="flex min-w-0 flex-wrap gap-1">
							{#each c.structures as h, k (`${h.chunkX},${h.chunkZ}`)}
								<span class="rounded-sm bg-btn px-1.5 py-px text-[11px] text-muted tabular-nums" title={h.structure ?? setLabel(h.set)}>
									{h.blockX}, {h.blockZ}{coverage ? ` · ${(c.memberCoverage[k] * 100).toFixed(0)}%` : ''}
								</span>
							{/each}
						</span>
					</span>
					<span class="flex w-20 shrink-0 flex-col items-end gap-0.5 tabular-nums">
						{#if coverage}
							<span class="text-[13px] font-medium">{(c.coverage * 100).toFixed(1)}%</span>
							<span class="text-[11px] text-faint">{m.tools_coverage()}</span>
						{:else}
							<span class="text-[13px] font-medium">{c.requiredRadius.toFixed(1)}</span>
							<span class="text-[11px] text-faint">{m.tools_required_radius()}</span>
						{/if}
					</span>
				</button>
				<button
					class="grid size-7 shrink-0 place-items-center rounded-md text-faint transition-colors duration-150 hover:bg-btn-hi hover:text-ink"
					onclick={() => copy(c)}
					title={m.tools_copy_coords()}
					aria-label={m.tools_copy_coords()}
				>
					<Icon name="copy" size={14} />
				</button>
			</div>
		{/each}
	{/snippet}
</FinderLayout>
