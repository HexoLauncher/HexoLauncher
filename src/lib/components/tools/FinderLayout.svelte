<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Button, Progress } from '$lib/ui';
	import { formatDuration } from '$lib/tools/draw';
	import type { RunningSearch } from '$lib/tools/state.svelte';
	import SeedMapView from './SeedMapView.svelte';
	import { navigate } from '$lib/nav';
	import { tools } from '$lib/tools/state.svelte';
	import { m } from '$lib/paraglide/messages.js';

	/** 搜尋頁的版面：左邊設定、右邊結果清單與地圖預覽。 */
	let {
		estimate,
		running,
		canSearch,
		count,
		elapsed,
		onsearch,
		onclear,
		search,
		oncancel,
		settings,
		results
	}: {
		estimate: string;
		running: boolean;
		canSearch: boolean;
		count: number;
		elapsed: string;
		onsearch: () => void;
		onclear: () => void;
		/** 執行中的搜尋（進度）。 */
		search: RunningSearch | null;
		oncancel: () => void;
		settings: Snippet;
		results: Snippet;
	} = $props();

	// 執行中每 250ms 更新一次經過時間與剩餘時間。
	let now = $state(performance.now());
	$effect(() => {
		if (!running) return;
		now = performance.now();
		const timer = setInterval(() => (now = performance.now()), 250);
		return () => clearInterval(timer);
	});

	const status = $derived.by(() => {
		if (!search) return null;
		const fraction = search.total ? Math.min(1, search.done / search.total) : 0;
		const elapsed = Math.max(0, (now - search.started) / 1000);
		// 剩餘時間依目前的實際速度推算；剛開始時太不準，先不顯示。
		const remaining = fraction > 0.03 && elapsed > 0.5 ? (elapsed * (1 - fraction)) / fraction : null;
		return { fraction, elapsed, remaining };
	});
</script>

<div class="flex min-h-0 flex-1 gap-3">
	<form
		class="flex w-60 shrink-0 flex-col overflow-hidden rounded-lg border border-line bg-group xl:w-72"
		onsubmit={(e) => {
			e.preventDefault();
			if (canSearch && !running) onsearch();
		}}
	>
		<div class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-3.5 py-3.5">
			{@render settings()}
		</div>
		<div class="flex shrink-0 items-center gap-2 border-t border-line px-3.5 py-3">
			{#if running && status}
				<div class="flex min-w-0 flex-1 flex-col gap-1.5">
					<Progress value={status.fraction} max={1} indeterminate={!search?.total} />
					<span class="truncate text-[11.5px] text-faint tabular-nums">
						{Math.floor(status.fraction * 100)}% · {formatDuration(status.elapsed)}
						{#if status.remaining !== null}· {m.tools_remaining({ time: formatDuration(status.remaining) })}{/if}
					</span>
				</div>
				<Button type="button" disabled={search?.cancelling} onclick={oncancel}>
					{search?.cancelling ? m.tools_cancelling() : m.common_cancel()}
				</Button>
			{:else}
				<span class="min-w-0 flex-1 truncate text-[11.5px] text-faint" title={estimate}>
					{#if !tools.seed}{m.tools_no_seed()}{:else}{estimate}{/if}
				</span>
				<Button type="submit" variant="primary" icon="search" disabled={!canSearch || running}>
					{m.tools_search()}
				</Button>
			{/if}
		</div>
	</form>

	<div class="flex min-w-0 flex-1 flex-col gap-2">
		<div class="flex shrink-0 items-center gap-2">
			<span class="font-semibold">{m.tools_results()}</span>
			{#if count}
				<span class="text-faint">{m.tools_results_meta({ count, seconds: elapsed })}</span>
				<span class="flex-1"></span>
				<button class="text-[12px] text-faint hover:text-ink" onclick={onclear}>{m.tools_clear_results()}</button>
			{/if}
		</div>
		<div class="flex max-h-[42%] min-h-24 shrink-0 flex-col overflow-y-auto rounded-lg border border-line bg-deep p-1">
			{#if count}
				{@render results()}
			{:else}
				<div class="grid flex-1 place-items-center px-4 py-6 text-center text-muted">
					{running ? m.tools_searching() : m.tools_results_empty()}
				</div>
			{/if}
		</div>
		<div class="flex min-h-48 flex-1 flex-col">
			<SeedMapView compact onexpand={() => navigate('/tools')} />
		</div>
	</div>
</div>
