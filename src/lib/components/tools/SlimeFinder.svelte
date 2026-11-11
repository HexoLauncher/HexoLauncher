<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import CenterField from './CenterField.svelte';
	import FinderLayout from './FinderLayout.svelte';
	import NumberField from './NumberField.svelte';
	import Segmented from './Segmented.svelte';
	import * as api from '$lib/api';
	import type { ToolSlimeSpot } from '$lib/api';
	import { report, reportError } from '$lib/state.svelte';
	import { estimateText } from '$lib/tools/draw';
	import {
		estimateSeconds,
		persistResults,
		recordRun,
		showOnMap,
		slimeWork,
		tools,
		cancelSearch,
		finishSearch,
		searches,
		startSearch
	} from '$lib/tools/state.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const form = tools.slimeForm;

	/** 掛機點和刷怪層有高度差時，水平方向的有效範圍會縮小。 */
	const ring = $derived({
		min: Math.sqrt(Math.max(0, 24 ** 2 - Number(form.heightDiff) ** 2)),
		max: Math.sqrt(Math.max(0, 128 ** 2 - Number(form.heightDiff) ** 2))
	});

	const work = $derived(slimeWork(Number(form.radius), Number(form.step)));
	const estimate = $derived(estimateText(estimateSeconds('slime', work)));

	const best = $derived(Math.max(...tools.slimeResults.map((s) => s.coverage), 0.0001));

	const steps = ['1', '2', '4', '8', '16'].map((v) => ({ value: v, label: v }));
	const radii = [1000, 3000, 10_000, 30_000].map((v) => ({ value: v, label: v >= 1000 ? `${v / 1000}k` : `${v}` }));

	async function run() {
		tools.slimeRunning = true;
		const taskId = startSearch('slime');
		const started = performance.now();
		const runWork = work;
		try {
			tools.slimeResults = await api.toolsSlimeSpots({
				seed: tools.seed,
				centerX: Number(form.centerX),
				centerZ: Number(form.centerZ),
				radius: Number(form.radius),
				step: Number(form.step),
				limit: Number(form.limit),
				minSeparation: Number(form.separation),
				ringMin: ring.min,
				ringMax: ring.max
			}, taskId);
			const seconds = (performance.now() - started) / 1000;
			tools.slimeElapsed = seconds.toFixed(2);
			recordRun('slime', runWork, seconds);
			persistResults();
			report(m.tools_status_found({ count: tools.slimeResults.length, seconds: tools.slimeElapsed }));
			if (tools.slimeResults.length) focus(tools.slimeResults[0], 0);
		} catch (e) {
			if (api.isCancelled(e)) report(m.tools_search_cancelled());
			else reportError(m.tools_error_search(), e);
		} finally {
			finishSearch('slime');
			tools.slimeRunning = false;
		}
	}

	function focus(s: ToolSlimeSpot, i: number) {
		tools.slimeSelected = i;
		tools.dimension = 'overworld';
		tools.slime = true;
		showOnMap(
			{
				x: s.x + 0.5,
				z: s.z + 0.5,
				label: m.tools_slime_spot_label({ rank: i + 1 }),
				ring: { min: ring.min, max: ring.max },
				chunks: s.chunks
			},
			ring.max
		);
	}

	async function copy(s: ToolSlimeSpot) {
		try {
			await navigator.clipboard.writeText(`${s.x} ~ ${s.z}`);
			report(m.tools_copied({ text: `${s.x} ~ ${s.z}` }));
		} catch {
			// 剪貼簿不可用時忽略
		}
	}

	function clear() {
		tools.slimeResults = [];
		tools.slimeElapsed = '';
		tools.focus = null;
		tools.slimeSelected = -1;
		persistResults();
	}
</script>

<FinderLayout
	{estimate}
	running={tools.slimeRunning}
	canSearch={!!tools.seed}
	count={tools.slimeResults.length}
	elapsed={tools.slimeElapsed}
	onsearch={run}
	onclear={clear}
	search={searches.slime}
	oncancel={() => cancelSearch('slime')}
>
	{#snippet settings()}
		<section class="flex flex-col gap-2.5">
			<h3 class="text-[11.5px] font-semibold tracking-wide text-faint">{m.tools_section_area()}</h3>
			<CenterField bind:x={form.centerX} bind:z={form.centerZ} />
			<NumberField label={m.tools_search_radius()} unit={m.tools_unit_blocks()} min={16} max={50_000} presets={radii} bind:value={form.radius} />
		</section>

		<section class="flex flex-col gap-2.5">
			<h3 class="text-[11.5px] font-semibold tracking-wide text-faint">{m.tools_section_afk()}</h3>
			<div class="flex flex-col gap-1">
				<span class="text-[12px] font-medium text-muted">{m.tools_slime_step()}</span>
				<Segmented label={m.tools_slime_step()} options={steps} bind:value={form.step} stretch />
				<span class="text-[11.5px] leading-[1.5] text-faint">{m.tools_slime_step_hint()}</span>
			</div>
			<NumberField
				label={m.tools_height_diff()}
				unit={m.tools_unit_blocks()}
				min={0}
				max={120}
				hint={m.tools_ring_effective({ min: ring.min.toFixed(1), max: ring.max.toFixed(1) })}
				bind:value={form.heightDiff}
			/>
		</section>

		<section class="flex flex-col gap-2.5">
			<h3 class="text-[11.5px] font-semibold tracking-wide text-faint">{m.tools_section_results()}</h3>
			<div class="grid grid-cols-2 gap-2">
				<NumberField label={m.tools_result_limit()} min={1} max={100} bind:value={form.limit} />
				<NumberField label={m.tools_min_separation_short()} unit={m.tools_unit_blocks()} min={0} bind:value={form.separation} />
			</div>
		</section>
	{/snippet}

	{#snippet results()}
		{#each tools.slimeResults as s, i (`${s.x},${s.z}`)}
			<div
				class="group flex items-center gap-3 rounded-md px-2.5 py-2 transition-colors duration-150
					{tools.slimeSelected === i ? 'bg-brand-soft shadow-[inset_2px_0_0_var(--color-brand)]' : 'hover:bg-hover'}"
			>
				<button class="flex min-w-0 flex-1 items-center gap-3 text-left" onclick={() => focus(s, i)}>
					<span class="w-6 shrink-0 text-center font-semibold text-faint tabular-nums">{i + 1}</span>
					<span class="flex min-w-0 flex-1 flex-col gap-1">
						<span class="truncate font-medium tabular-nums">X {s.x} · Z {s.z}</span>
						<span class="text-[11.5px] text-muted tabular-nums">
							{m.tools_slime_row({ chunks: s.slimeChunks, columns: s.spawnableColumns })}
						</span>
					</span>
					<span class="flex w-28 shrink-0 items-center gap-2 xl:w-36">
						<span class="h-1.5 flex-1 overflow-hidden rounded-full bg-btn">
							<span class="block h-full rounded-full bg-[#5ce66e]" style:width="{(s.coverage / best) * 100}%"></span>
						</span>
						<span class="w-12 text-right text-[12px] tabular-nums">{(s.coverage * 100).toFixed(1)}%</span>
					</span>
				</button>
				<button
					class="grid size-7 shrink-0 place-items-center rounded-md text-faint transition-colors duration-150 hover:bg-btn-hi hover:text-ink"
					onclick={() => copy(s)}
					title={m.tools_copy_coords()}
					aria-label={m.tools_copy_coords()}
				>
					<Icon name="copy" size={14} />
				</button>
			</div>
		{/each}
	{/snippet}
</FinderLayout>
