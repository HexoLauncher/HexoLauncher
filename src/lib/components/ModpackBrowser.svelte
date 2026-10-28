<script lang="ts">
	import { Button, Select, TextInput } from '$lib/ui';
	import { onMount } from 'svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Switch from '$lib/ui/Switch.svelte';
	import ContentDetailView from './ContentDetailView.svelte';
	import * as api from '$lib/api';
	import type { ContentDetail, ModpackPlatform, ModpackSort, PackHit, RemoteVersion } from '$lib/api';
	import { formatCount, formatDate } from '$lib/format';
	import { m } from '$lib/paraglide/messages.js';


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
		{ id: 'relevance', label: m.sort_relevance() },
		{ id: 'downloads', label: m.sort_downloads() },
		{ id: 'updated', label: m.sort_updated() },
		{ id: 'name', label: m.sort_name() }
	];

	const KIND_LABELS: Record<string, string> = {
		beta: m.release_beta(),
		alpha: m.release_alpha(),
		archived: m.release_archived()
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

	let detail = $state<ContentDetail | null>(null);
	let detailError = $state('');
	let loadingDetail = $state(false);

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

	function choose(item: PackHit) {
		if (hit?.id === item.id) return;
		hit = item;
		version = null;
		versions = [];
		detail = null;
		detailError = '';
		onchoose?.(item);

		loadDetail(item);
		loadVersions(item);
	}

	async function loadDetail(item: PackHit) {
		loadingDetail = true;
		try {
			const found = await api.getModpackDetail(platform, item.id);
			if (hit?.id !== item.id) return;
			detail = found;
		} catch (e) {
			if (hit?.id === item.id) detailError = api.errorMessage(e);
		} finally {
			if (hit?.id === item.id) loadingDetail = false;
		}
	}

	async function loadVersions(item: PackHit) {
		loadingVersions = true;
		try {
			const found = await api.listModpackVersions(platform, item.id);
			if (hit?.id !== item.id) return;

			versions = found;
			const mc = mcVersion.trim();
			version =
				(mc && found.find((v) => v.mcVersion === mc && v.releaseType === 'release')) ||
				found.find((v) => v.releaseType === 'release') ||
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
		if (KIND_LABELS[v.releaseType]) parts.push(KIND_LABELS[v.releaseType]);
		if (v.published) parts.push(formatDate(v.published));
		return parts.join(' · ');
	}

</script>

<div class="flex min-h-0 flex-1 flex-col gap-2.5">
	<div class="flex shrink-0 gap-1.75">
		<TextInput class="min-w-0 flex-1" bind:value={query} oninput={searchSoon} onkeydown={(e) => e.key === 'Enter' && search()} placeholder={m.modpack_search()} aria-label={m.modpack_search()} />
		<TextInput class="w-24" bind:value={mcVersion} onchange={() => search()} placeholder={m.modpack_mc_version()} aria-label={m.modpack_mc_version_aria()} />
		<Select class="w-26" bind:value={sort} onchange={() => search()} aria-label={m.common_sort()}>
			{#each sorts as s (s.id)}
				<option value={s.id}>{s.label}</option>
			{/each}
		</Select>
	</div>

	<div class="flex min-h-0 flex-1 gap-2.5">
		<div
			bind:this={list}
			class="w-[42%] shrink-0 overflow-y-auto rounded-lg border border-line bg-deep"
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
							<span class="truncate font-semibold {hit?.id === item.id ? 'text-brand-fg' : 'text-ink'}">
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
						{m.common_searching()}
					</span>
				{:else if message}
					<span class="leading-[1.7] text-danger">{message}</span>
					<Button onclick={() => search(hits.length > 0)}>{m.common_retry()}</Button>
				{:else if hits.length === 0}
					<span>{m.modpack_none_found()}</span>
				{:else if nextOffset !== null}
					<Button onclick={() => search(true)}>
						{m.list_load_more({ loaded: hits.length, total })}
					</Button>
				{:else}
					<span class="text-faint">{m.list_total({ total })}</span>
				{/if}
			</div>
		</div>

		<div class="flex min-w-0 flex-1 flex-col gap-2.5">
			<ContentDetailView
				{detail}
				loading={!!hit && loadingDetail}
				error={detailError}
				author={hit?.author}
				placeholder={m.modpack_pick()}
				{onerror}
			/>

			{#if hit}
				<div class="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-1.5">
					<label class="flex min-w-0 flex-1 items-center gap-2 text-muted">
						{m.common_version()}
						<Select class="min-w-0 flex-1" value={version?.id ?? ''} onchange={(e) => (version = versions.find((v) => v.id === e.currentTarget.value) ?? null)} disabled={disabled || loadingVersions || versions.length === 0}>
							{#if loadingVersions}
								<option value="">{m.common_loading_versions()}</option>
							{:else if versions.length === 0}
								<option value="">{m.modpack_no_installable_version()}</option>
							{/if}
							{#each versions as v (v.id)}
								<option value={v.id}>{versionLabel(v)}</option>
							{/each}
						</Select>
					</label>
					{#if hasOptional}
						<Switch bind:checked={includeOptional} label={m.modpack_install_optional()} {disabled} />
					{/if}
				</div>
			{/if}
		</div>
	</div>
</div>
