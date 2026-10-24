<script lang="ts">
	import { Button, CheckRow, Pill, Select, TextInput } from '$lib/ui';
	import { onMount } from 'svelte';
	import Icon from '$lib/ui/Icon.svelte';
	import Dialog from '$lib/ui/Dialog.svelte';
	import * as api from '$lib/api';
	import type {
		ContentDetail,
		ContentKind,
		ContentPlatform,
		ContentVersion,
		InstanceView,
		ModpackSort,
		PackHit
	} from '$lib/api';
	import { formatCount, formatDate, loaderLabel } from '$lib/format';
	import { m } from '$lib/paraglide/messages.js';
	import { renderRichText } from '$lib/richtext';

	let {
		baseDir,
		instance,
		kind,
		oninstalled
	}: {
		baseDir: string;
		instance: InstanceView;
		kind: ContentKind;
		oninstalled: (fileName: string) => void;
	} = $props();

	const PLATFORMS: { id: ContentPlatform; label: string }[] = [
		{ id: 'modrinth', label: 'Modrinth' },
		{ id: 'curseforge', label: 'CurseForge' }
	];

	const SORTS: { id: ModpackSort; label: string }[] = [
		{ id: 'relevance', label: m.sort_relevance() },
		{ id: 'downloads', label: m.sort_downloads() },
		{ id: 'updated', label: m.sort_updated() }
	];

	const KIND_LABELS: Record<string, string> = {
		beta: m.release_beta(),
		alpha: m.release_alpha()
	};

	const TEXT: Record<ContentKind, { search: string; empty: string; pick: string }> = {
		mod: {
			search: m.content_search_mod(),
			empty: m.content_empty_mod(),
			pick: m.content_pick_mod()
		},
		resourcepack: {
			search: m.content_search_resourcepack(),
			empty: m.content_empty_resourcepack(),
			pick: m.content_pick_resourcepack()
		},
		shader: {
			search: m.content_search_shader(),
			empty: m.content_empty_shader(),
			pick: m.content_pick_shader()
		}
	};

	const loader = $derived(kind === 'mod' ? instance.loader : null);

	let platform = $state<ContentPlatform>('modrinth');
	let query = $state('');
	let sort = $state<ModpackSort>('relevance');
	let matchVersion = $state(true);

	let hits = $state<PackHit[]>([]);
	let total = $state(0);
	let nextOffset = $state<number | null>(null);
	let loading = $state(false);
	let message = $state('');

	let hit = $state<PackHit | null>(null);
	let detail = $state<ContentDetail | null>(null);
	let detailError = $state('');
	let loadingDetail = $state(false);

	let versions = $state<ContentVersion[]>([]);
	let version = $state<ContentVersion | null>(null);
	let loadingVersions = $state(false);

	let busy = $state(false);
	let status = $state('');
	let includeOptional = $state(true);
	let installPlan = $state<api.ModInstallPlan | null>(null);
	let preparing = $state(false);
	let committing = $state(false);
	let disposed = false;

	let list = $state<HTMLElement>();
	let timer: ReturnType<typeof setTimeout> | undefined;
	let seq = 0;

	const mcVersion = $derived(matchVersion ? instance.versionId : null);


	const descriptionHtml = $derived(
		detail?.description ? renderRichText(detail.description, detail.descriptionFormat) : ''
	);

	const compatLabel = $derived(
		loader && loader !== 'vanilla'
			? `${instance.versionId} / ${loaderLabel(loader)}`
			: instance.versionId
	);

	onMount(() => {
		search();
		return () => {
			disposed = true;
			clearTimeout(timer);
			if (installPlan) api.discardModInstall(installPlan.id).catch(() => {});
		};
	});

	function searchSoon() {
		clearTimeout(timer);
		timer = setTimeout(() => search(), 400);
	}


	function reset() {
		clearSelection();
		status = '';
		search();
	}

	function clearSelection() {
		hit = null;
		detail = null;
		detailError = '';
		versions = [];
		version = null;
	}

	async function search(append = false) {
		clearTimeout(timer);
		const current = ++seq;
		loading = true;
		message = '';
		try {
			const page = await api.searchContent({
				platform,
				kind,
				query: query.trim(),
				sort,
				mcVersion,
				loader,
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
		if (busy || hit?.id === item.id) return;
		clearSelection();
		hit = item;
		status = '';

		loadDetail(item);
		loadVersions(item);
	}

	async function loadDetail(item: PackHit) {
		loadingDetail = true;
		try {
			const found = await api.getContentDetail({ platform, kind, projectId: item.id });
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
			const found = await api.listContentVersions({
				platform,
				kind,
				projectId: item.id,
				mcVersion,
				loader
			});
			if (hit?.id !== item.id) return;

			versions = found;
			version = found.find((v) => v.kind === 'release' && v.downloadable) ?? found[0] ?? null;
		} catch (e) {
			if (hit?.id === item.id) status = api.errorMessage(e);
		} finally {
			if (hit?.id === item.id) loadingVersions = false;
		}
	}

	async function install() {
		const item = hit;
		const picked = version;
		if (!item || !picked || busy) return;

		busy = true;
		status = '';
		try {
			if (kind === 'mod') {
				preparing = true;
				const plan = await api.prepareModInstall({
					baseDir,
					instanceName: instance.id,
					platform,
					projectId: item.id,
					versionId: picked.id,
					includeOptional
				});
				if (disposed) await api.discardModInstall(plan.id);
				else installPlan = plan;
				return;
			}
			const installed = await api.installContent({
				baseDir,
				instanceName: instance.id,
				platform,
				kind,
				projectId: item.id,
				versionId: picked.id
			});
			status = m.content_installed({ file: installed.fileName });
			oninstalled(installed.fileName);
		} catch (e) {
			status = api.errorMessage(e);
		} finally {
			preparing = false;
			busy = false;
		}
	}

	function cancelInstall() {
		if (committing || !installPlan) return;
		const id = installPlan.id;
		installPlan = null;
		api.discardModInstall(id).catch((e) => (status = api.errorMessage(e)));
	}

	async function confirmInstall() {
		if (!installPlan || committing || installPlan.items.some((item) => item.conflict)) return;
		const plan = installPlan;
		committing = true;
		busy = true;
		try {
			const installed = await api.commitModInstall(plan.id);
			status = m.content_installed({ file: installed.fileName });
			oninstalled(installed.fileName);
		} catch (e) {
			status = api.errorMessage(e);
		} finally {
			installPlan = null;
			committing = false;
			busy = false;
		}
	}

	function versionLabel(v: ContentVersion) {
		const parts = [v.name];
		if (v.mcVersion) parts.push(`MC ${v.mcVersion}`);
		if (KIND_LABELS[v.kind]) parts.push(KIND_LABELS[v.kind]);
		if (v.published) parts.push(formatDate(v.published));
		if (!v.downloadable) parts.push(m.content_not_downloadable());
		return parts.join(' · ');
	}


	function openSite(url: string) {
		api.openExternal(url).catch((e) => (status = api.errorMessage(e)));
	}


	function externalLinks(node: HTMLElement) {
		function onClick(event: MouseEvent) {
			const link = (event.target as HTMLElement | null)?.closest('a');
			const href = link?.getAttribute('href');
			if (!href) return;

			event.preventDefault();
			if (/^https?:\/\//i.test(href)) openSite(href);
		}

		node.addEventListener('click', onClick);
		return { destroy: () => node.removeEventListener('click', onClick) };
	}
</script>

<div class="flex min-h-0 flex-1 flex-col gap-2.5">
	<div class="flex shrink-0 gap-1.75">
		<Select class="w-28" bind:value={platform} onchange={reset} disabled={busy} aria-label={m.content_platform()}>
			{#each PLATFORMS as p (p.id)}
				<option value={p.id}>{p.label}</option>
			{/each}
		</Select>
		<TextInput class="min-w-0 flex-1" bind:value={query} oninput={searchSoon} onkeydown={(e) => e.key === 'Enter' && search()} disabled={busy} placeholder={TEXT[kind].search} aria-label={TEXT[kind].search} />
		<Select class="w-26" bind:value={sort} onchange={() => search()} disabled={busy} aria-label={m.common_sort()}>
			{#each SORTS as s (s.id)}
				<option value={s.id}>{s.label}</option>
			{/each}
		</Select>
	</div>

	<CheckRow compact class="shrink-0">
		<input type="checkbox" bind:checked={matchVersion} onchange={reset} disabled={busy} />
		{m.content_filter_compatible({ target: compatLabel })}
	</CheckRow>
	{#if kind === 'mod'}
		<CheckRow compact class="shrink-0">
			<input type="checkbox" bind:checked={includeOptional} disabled={busy} />
			{m.content_include_optional()}
		</CheckRow>
	{/if}

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
					<span>{TEXT[kind].empty}</span>
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
			<div class="min-h-0 flex-1 overflow-y-auto rounded-lg border border-line bg-deep px-3.5 py-3">
				{#if !hit}
					<p class="grid h-full place-items-center text-center text-faint">{TEXT[kind].pick}</p>
				{:else if loadingDetail}
					<p class="grid h-full place-items-center text-muted">
						<span class="flex items-center gap-2">
							<span
								class="size-4 animate-spin rounded-full border-2 border-line border-t-brand"
							></span>
							{m.content_loading_detail()}
						</span>
					</p>
				{:else if detailError}
					<p class="leading-[1.7] text-danger">{detailError}</p>
				{:else if detail}
					{@const info = detail}
					<div class="flex flex-col gap-3">
						<div class="flex items-start gap-2.5">
							{#if info.iconUrl}
								<img
									src={info.iconUrl}
									alt=""
									class="size-12 shrink-0 rounded-md bg-window object-cover"
								/>
							{/if}
							<div class="min-w-0 flex-1">
								<h4 class="truncate font-semibold text-ink">{info.name}</h4>
								{#if hit.author}
									<p class="truncate text-faint">{m.content_by_author({ author: hit.author })}</p>
								{/if}
							</div>
						</div>

						{#if info.summary}
							<p class="leading-[1.7] text-ink">{info.summary}</p>
						{/if}

						<div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-faint">
							<span class="flex items-center gap-1">
								<Icon name="download" size={11} />{m.content_downloads({ count: formatCount(info.downloads) })}
							</span>
							{#if info.follows}
								<span class="flex items-center gap-1">
									<Icon name="user" size={11} />{formatCount(info.follows)}
								</span>
							{/if}
							{#if info.updated}
								<span class="flex items-center gap-1">
									<Icon name="clock" size={11} />{m.content_updated({ date: formatDate(info.updated) })}
								</span>
							{/if}
							{#if info.license}
								<span class="flex items-center gap-1">
									<Icon name="shield" size={11} />{info.license}
								</span>
							{/if}
						</div>

						{#if info.categories.length || info.loaders.length}
							<div class="flex flex-wrap gap-1.5">
								{#each info.loaders as name (name)}
									<Pill accent>{name}</Pill>
								{/each}
								{#each info.categories as name (name)}
									<Pill>{name}</Pill>
								{/each}
							</div>
						{/if}

						{#if info.gallery.length}
							<div class="flex gap-2 overflow-x-auto pb-1">
								{#each info.gallery.slice(0, 10) as image (image.url)}
									<button
										class="shrink-0 rounded-md border border-line p-0"
										title={image.title || m.content_open_image()}
										onclick={() => openSite(image.url)}
									>
										<img
											src={image.url}
											alt={image.title}
											loading="lazy"
											class="h-20 w-32 rounded-md object-cover"
										/>
									</button>
								{/each}
							</div>
						{/if}

						{#if info.gameVersions.length}
							<p class="leading-[1.7] text-muted">
								<span class="text-faint">{m.content_supported_versions()}</span>
								{info.gameVersions.slice(-12).reverse().join(m.common_list_separator())}
							</p>
						{/if}

						{#if descriptionHtml}
							<!-- eslint-disable-next-line svelte/no-at-html-tags -->
							<div class="rich-text" use:externalLinks>{@html descriptionHtml}</div>
						{:else}
							<p class="text-faint">{m.content_no_description()}</p>
						{/if}

						{#if info.links.length}
							<div class="flex flex-wrap gap-1.75">
								{#each info.links as link (link.url)}
									<Button onclick={() => openSite(link.url)}>
										<Icon name="globe" size={13} />{link.label}
									</Button>
								{/each}
							</div>
						{/if}
					</div>
				{/if}
			</div>

			{#if hit}
				<div class="flex shrink-0 items-center gap-2.5">
					<label class="flex min-w-0 flex-1 items-center gap-2 text-muted">
						{m.common_version()}
						<Select class="min-w-0 flex-1" value={version?.id ?? ''} onchange={(e) => (version = versions.find((v) => v.id === e.currentTarget.value) ?? null)} disabled={busy || loadingVersions || versions.length === 0}>
							{#if loadingVersions}
								<option value="">{m.common_loading_versions()}</option>
							{:else if versions.length === 0}
								<option value="">{m.content_no_compatible_version()}</option>
							{/if}
							{#each versions as v (v.id)}
								<option value={v.id}>{versionLabel(v)}</option>
							{/each}
						</Select>
					</label>
					<Button variant="primary" disabled={busy || !version || !version.downloadable} onclick={install}>
						<Icon name="download" size={13} />{m.common_install()}
					</Button>
				</div>
			{/if}
		</div>
	</div>

	{#if busy || status}
		<p class="max-h-24 shrink-0 overflow-y-auto break-words px-0.5 text-muted" role="status">
			{busy ? (preparing ? m.content_preparing_mods() : m.content_downloading()) : status}
		</p>
	{/if}
</div>

{#if installPlan}
	{@const plan = installPlan}
	<Dialog title={m.content_install_plan_title()} width="640px" onclose={cancelInstall}>
		<div class="flex min-h-0 flex-col gap-3 overflow-y-auto px-4.5 py-4">
			{#each plan.items as item (item.fileName)}
				<div class="flex shrink-0 flex-col gap-1 rounded-lg border border-line p-2.5">
					<div class="flex items-center gap-2">
						<span class="min-w-0 flex-1 break-words font-medium">{item.name} {item.version}</span>
						<Pill>{item.dependency ? m.content_dependency() : m.content_selected_mod()}</Pill>
					</div>
					<span class="break-all text-[11.5px] text-faint">{item.fileName}</span>
					{#if item.conflict}
						<p class="break-words text-danger">{m.content_mod_conflict({ file: item.conflict })}</p>
					{:else}
						<span class="text-muted">{item.installed ? m.content_mod_already_installed() : m.content_mod_ready()}</span>
					{/if}
				</div>
			{/each}
		</div>
		{#snippet footer()}
			<Button disabled={committing} onclick={cancelInstall}>{m.common_cancel()}</Button>
			<Button variant="primary" disabled={committing || plan.items.some((item) => item.conflict)} onclick={confirmInstall}>
				{committing ? m.content_installing_mods() : m.common_install()}
			</Button>
		{/snippet}
	</Dialog>
{/if}
