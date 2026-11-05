<script lang="ts">
	import { Button } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import * as api from '$lib/api';
	import type { ManualDownload } from '$lib/api';
	import { SvelteSet } from 'svelte/reactivity';
	import { onMount } from 'svelte';
	import { m } from '$lib/paraglide/messages.js';

	let {
		baseDir,
		instanceId,
		items,
		onerror
	}: {
		baseDir: string;
		instanceId: string;
		items: ManualDownload[];
		onerror?: (message: string) => void;
	} = $props();

	const done = new SvelteSet<string>();
	let downloadsDir = $state('');
	let watchError = $state('');

	const pending = $derived(items.filter((item) => !done.has(item.dest)));
	const openable = $derived(pending.filter((item) => item.website));

	onMount(() => {
		let stopped = false;
		const unlisten = api.onManualDownloadFound((found) => {
			if (found.instanceName === instanceId) done.add(found.dest);
		});
		api
			.watchManualDownloads(baseDir, instanceId, items)
			.then((watch) => {
				if (stopped) return;
				downloadsDir = watch.downloadsDir;
				watch.done.forEach((dest) => done.add(dest));
			})
			.catch((e) => {
				if (!stopped) watchError = api.errorMessage(e);
			});
		return () => {
			stopped = true;
			unlisten.then((stop) => stop());
			api.stopManualDownloads(instanceId).catch(() => {});
		};
	});

	function open(url: string) {
		api.openExternal(url).catch((e) => onerror?.(api.errorMessage(e)));
	}

	async function openAll() {
		for (const item of openable) {
			open(item.website!);
			await new Promise((resolve) => setTimeout(resolve, 250));
		}
	}
</script>

<div class="flex min-h-0 flex-col gap-2">
	<div class="flex shrink-0 items-center gap-2.5">
		<p class="min-w-0 flex-1 font-medium {pending.length ? 'text-danger' : 'text-ink'}">
			{pending.length
				? m.manual_pending({ pending: pending.length, total: items.length })
				: m.manual_all_done({ total: items.length })}
		</p>
		{#if openable.length > 1}
			<Button variant="primary" onclick={openAll}>
				<Icon name="download" size={13} />{m.manual_open_all({ count: openable.length })}
			</Button>
		{/if}
	</div>

	{#if watchError}
		<p class="leading-[1.7] text-danger">{watchError}</p>
	{:else if pending.length && downloadsDir}
		<p class="flex items-center gap-2 leading-[1.7] text-muted">
			<span class="size-2.5 shrink-0 animate-spin rounded-full border-[1.5px] border-line border-t-brand"></span>
			<span class="min-w-0">{m.manual_watching({ dir: downloadsDir })}</span>
		</p>
	{/if}

	<ul class="min-h-0 flex-1 overflow-y-auto rounded-lg border border-line bg-deep">
		{#each items as item (item.dest)}
			{@const placed = done.has(item.dest)}
			<li class="flex items-center gap-3 border-b border-line px-3 py-2 last:border-b-0">
				<div class="min-w-0 flex-1">
					<div class="truncate {placed ? 'text-faint line-through' : 'text-ink'}">{item.name}</div>
					<div class="truncate font-mono text-[11px] text-faint">{item.dest}</div>
				</div>
				{#if placed}
					<span class="flex shrink-0 items-center gap-1 text-brand-fg">
						<Icon name="check" size={13} />{m.manual_placed()}
					</span>
				{:else if item.website}
					{@const website = item.website}
					<Button onclick={() => open(website)}>
						<Icon name="download" size={13} />{m.manual_open_download()}
					</Button>
				{/if}
				<Button
					onclick={() =>
						api
							.openInstanceFolder(baseDir, instanceId, item.folder)
							.catch((e) => onerror?.(api.errorMessage(e)))}
				>
					<Icon name="folder" size={13} />{m.action_folder()}
				</Button>
			</li>
		{/each}
	</ul>
</div>
