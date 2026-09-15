<script lang="ts">
	import { onMount } from 'svelte';
	import Icon from '$lib/Icon.svelte';
	import Toggle from './Toggle.svelte';
	import * as api from '$lib/api';
	import type { ModpackPlatform, ModpackSort, PackHit, RemoteVersion } from '$lib/api';
	import { formatDate } from '$lib/format';


	let {
		platform,
		hit = $bindable(null),
		version = $bindable(null),
		includeOptional = $bindable(true),
		disabled = false,
		onchoose,
		onerror
	}: {
		platform: ModpackPlatform;
		hit?: PackHit | null;
		version?: RemoteVersion | null;
		includeOptional?: boolean;
		disabled?: boolean;
		onchoose?: (hit: PackHit) => void;
		onerror: (message: string) => void;
	} = $props();

	const SORTS: { id: ModpackSort; label: string }[] = [
		{ id: 'relevance', label: '相關性' },
		{ id: 'downloads', label: '熱門' },
		{ id: 'updated', label: '最近更新' },
		{ id: 'name', label: '名稱' }
	];

	const KIND_LABELS: Record<string, string> = {
		beta: '測試版',
		alpha: 'Alpha',
		archived: '已封存'
	};

	const sorts = $derived(
		platform === 'modrinth'
			? SORTS.filter((s) => s.id !== 'name')
			: platform === 'atlauncher'
				? SORTS.filter((s) => s.id !== 'downloads')
				: SORTS
	);
	const hasOptional = $derived(platform === 'atlauncher' || platform === 'ftb');

	let query = $state('');
	let sort = $state<ModpackSort>('relevance');
	let mcVersion = $state('');

	let hits = $state<PackHit[]>([]);
	let total = $state(0);
	let nextOffset = $state<number | null>(null);
	let loading = $state(false);
	let message = $state('');

	let versions = $state<RemoteVersion[]>([]);
	let loadingVersions = $state(false);

	let list = $state<HTMLElement>();
	let timer: ReturnType<typeof setTimeout> | undefined;
	let seq = 0;

	onMount(() => {
		search();
		return () => clearTimeout(timer);
	});

	function searchSoon() {
		clearTimeout(timer);
		timer = setTimeout(() => search(), 400);
	}

	async function search(append = false) {
		clearTimeout(timer);
		const current = ++seq;
		loading = true;
		message = '';
		try {
			const page = await api.searchModpacks({
				platform,
				query: query.trim(),
				sort,
				mcVersion: mcVersion.trim() || null,
				offset: append ? (nextOffset ?? 0) : 0
			});
			if (current !== seq) return;

			hits = append ? [...hits, ...page.hits] : page.hits;
			total = page.total;
			nextOffset = page.nextOffset;
			if (!append) list?.scrollTo({ top: 0 });
		} catch (e) {
			if (current !== seq) return;
			if (!append) {
				hits = [];
				total = 0;
				nextOffset = null;
			}
			message = api.errorMessage(e);
		} finally {
			if (current === seq) loading = false;
		}
	}

	async function choose(item: PackHit) {
		if (hit?.id === item.id) return;
		hit = item;
		version = null;
		versions = [];
		onchoose?.(item);

		loadingVersions = true;
		try {
			const found = await api.listModpackVersions(platform, item.id);
			if (hit?.id !== item.id) return;

			versions = found;
			const mc = mcVersion.trim();
			version =
				(mc && found.find((v) => v.mcVersion === mc && v.kind === 'release')) ||
				found.find((v) => v.kind === 'release') ||
				found[0] ||
				null;
		} catch (e) {
			if (hit?.id === item.id) onerror(api.errorMessage(e));
		} finally {
			if (hit?.id === item.id) loadingVersions = false;
		}
	}

	function versionLabel(v: RemoteVersion) {
		const parts = [v.name];
		if (v.mcVersion) parts.push(`MC ${v.mcVersion}`);
		if (KIND_LABELS[v.kind]) parts.push(KIND_LABELS[v.kind]);
		if (v.published) parts.push(formatDate(v.published));
		return parts.join(' · ');
	}

	function formatCount(n: number) {
		if (n >= 1e8) return `${(n / 1e8).toFixed(1)} 億`;
		if (n >= 1e4) return `${(n / 1e4).toFixed(1)} 萬`;
		return String(n);
	}
</script>

<div class="flex min-h-0 flex-1 flex-col gap-2.5">
	<div class="flex shrink-0 gap-1.75">
		<input
			class="qt-input min-w-0 flex-1"
			bind:value={query}
			oninput={searchSoon}
			onkeydown={(e) => e.key === 'Enter' && search()}
			placeholder="搜尋模組包"
			aria-label="搜尋模組包"
		/>
		<input
			class="qt-input w-24"
			bind:value={mcVersion}
			onchange={() => search()}
			placeholder="MC 版本"
			aria-label="Minecraft 版本"
		/>
		<select class="qt-select w-26" bind:value={sort} onchange={() => search()} aria-label="排序">
			{#each sorts as s (s.id)}
				<option value={s.id}>{s.label}</option>
			{/each}
		</select>
	</div>

	<div
		bind:this={list}
		class="min-h-0 flex-1 overflow-y-auto rounded-lg border border-line bg-deep"
		aria-busy={loading}
	>
		{#each hits as item (item.id)}
			<button
				class="flex w-full items-start gap-2.5 border-b border-line px-2.5 py-2 text-left transition-colors duration-150 hover:bg-window
					{hit?.id === item.id ? 'bg-brand-soft' : ''}"
				onclick={() => choose(item)}
			>
				{#if item.iconUrl}
					<img
						src={item.iconUrl}
						alt=""
						loading="lazy"
						class="size-10 shrink-0 rounded-md bg-window object-cover"
					/>
				{:else}
					<span class="grid size-10 shrink-0 place-items-center rounded-md bg-window text-faint">
						<Icon name="package" size={18} />
					</span>
				{/if}
				<span class="min-w-0 flex-1">
					<span class="flex items-baseline gap-2">
						<span class="truncate font-semibold {hit?.id === item.id ? 'text-brand' : 'text-ink'}">
							{item.name}
						</span>
						{#if item.author}
							<span class="truncate text-faint">{item.author}</span>
						{/if}
						{#if item.downloads}
							<span class="ml-auto flex shrink-0 items-center gap-1 text-faint">
								<Icon name="download" size={11} />{formatCount(item.downloads)}
							</span>
						{/if}
					</span>
					<span class="line-clamp-2 leading-[1.6] text-muted">{item.summary}</span>
				</span>
			</button>
		{/each}

		<div class="flex flex-col items-center gap-2 px-3 py-3 text-muted">
			{#if loading}
				<span class="flex items-center gap-2">
					<span
						class="size-4 animate-spin rounded-full border-2 border-line border-t-brand"
					></span>
					正在搜尋…
				</span>
			{:else if message}
				<span class="leading-[1.7] text-danger">{message}</span>
				<button class="qt-button" onclick={() => search(hits.length > 0)}>重試</button>
			{:else if hits.length === 0}
				<span>找不到符合的模組包</span>
			{:else if nextOffset !== null}
				<button class="qt-button" onclick={() => search(true)}>
					載入更多（{hits.length} / {total}）
				</button>
			{:else}
				<span class="text-faint">共 {total} 個</span>
			{/if}
		</div>
	</div>

	{#if hit}
		{@const selected = hit}
		<div class="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-1.5">
			<label class="flex min-w-0 flex-1 items-center gap-2 text-muted">
				版本
				<select
					class="qt-select min-w-0 flex-1"
					value={version?.id ?? ''}
					onchange={(e) =>
						(version = versions.find((v) => v.id === e.currentTarget.value) ?? null)}
					disabled={disabled || loadingVersions || versions.length === 0}
				>
					{#if loadingVersions}
						<option value="">讀取版本中…</option>
					{:else if versions.length === 0}
						<option value="">沒有可安裝的版本</option>
					{/if}
					{#each versions as v (v.id)}
						<option value={v.id}>{versionLabel(v)}</option>
					{/each}
				</select>
			</label>
			{#if hasOptional}
				<Toggle bind:checked={includeOptional} label="安裝選用模組" {disabled} />
			{/if}
			{#if selected.website}
				{@const website = selected.website}
				<button
					class="qt-button"
					onclick={() => api.openExternal(website).catch((e) => onerror(api.errorMessage(e)))}
				>
					<Icon name="globe" size={13} />網站
				</button>
			{/if}
		</div>
	{/if}
</div>
