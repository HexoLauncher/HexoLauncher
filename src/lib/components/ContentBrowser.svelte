<script lang="ts">
	import { onMount } from 'svelte';
	import Icon from '$lib/Icon.svelte';
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
	import { formatDate, loaderLabel } from '$lib/format';
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
		{ id: 'relevance', label: '相關性' },
		{ id: 'downloads', label: '熱門' },
		{ id: 'updated', label: '最近更新' }
	];

	const KIND_LABELS: Record<string, string> = {
		beta: '測試版',
		alpha: 'Alpha'
	};

	const TEXT: Record<ContentKind, { search: string; empty: string; pick: string }> = {
		mod: { search: '搜尋模組', empty: '找不到符合的模組', pick: '選一個模組看介紹' },
		resourcepack: {
			search: '搜尋資源包',
			empty: '找不到符合的資源包',
			pick: '選一個資源包看介紹'
		},
		shader: { search: '搜尋光影包', empty: '找不到符合的光影包', pick: '選一個光影包看介紹' }
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
		return () => clearTimeout(timer);
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
		if (hit?.id === item.id) return;
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
		if (!item || !picked) return;

		busy = true;
		status = '';
		try {
			const installed = await api.installContent({
				baseDir,
				instanceName: instance.id,
				platform,
				kind,
				projectId: item.id,
				versionId: picked.id
			});
			status = `已安裝 ${installed.fileName}`;
			oninstalled(installed.fileName);
		} catch (e) {
			status = api.errorMessage(e);
		} finally {
			busy = false;
		}
	}

	function versionLabel(v: ContentVersion) {
		const parts = [v.name];
		if (v.mcVersion) parts.push(`MC ${v.mcVersion}`);
		if (KIND_LABELS[v.kind]) parts.push(KIND_LABELS[v.kind]);
		if (v.published) parts.push(formatDate(v.published));
		if (!v.downloadable) parts.push('不可自動下載');
		return parts.join(' · ');
	}

	function formatCount(n: number) {
		if (n >= 1e8) return `${(n / 1e8).toFixed(1)} 億`;
		if (n >= 1e4) return `${(n / 1e4).toFixed(1)} 萬`;
		return String(n);
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
		<select class="qt-select w-28" bind:value={platform} onchange={reset} aria-label="平台">
			{#each PLATFORMS as p (p.id)}
				<option value={p.id}>{p.label}</option>
			{/each}
		</select>
		<input
			class="qt-input min-w-0 flex-1"
			bind:value={query}
			oninput={searchSoon}
			onkeydown={(e) => e.key === 'Enter' && search()}
			placeholder={TEXT[kind].search}
			aria-label={TEXT[kind].search}
		/>
		<select class="qt-select w-26" bind:value={sort} onchange={() => search()} aria-label="排序">
			{#each SORTS as s (s.id)}
				<option value={s.id}>{s.label}</option>
			{/each}
		</select>
	</div>

	<label class="qt-check compact shrink-0">
		<input type="checkbox" bind:checked={matchVersion} onchange={reset} />
		只顯示相容 {compatLabel} 的版本
	</label>

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
						正在搜尋…
					</span>
				{:else if message}
					<span class="leading-[1.7] text-danger">{message}</span>
					<button class="qt-button" onclick={() => search(hits.length > 0)}>重試</button>
				{:else if hits.length === 0}
					<span>{TEXT[kind].empty}</span>
				{:else if nextOffset !== null}
					<button class="qt-button" onclick={() => search(true)}>
						載入更多（{hits.length} / {total}）
					</button>
				{:else}
					<span class="text-faint">共 {total} 個</span>
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
							正在讀取介紹…
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
									<p class="truncate text-faint">由 {hit.author} 製作</p>
								{/if}
							</div>
						</div>

						{#if info.summary}
							<p class="leading-[1.7] text-ink">{info.summary}</p>
						{/if}

						<div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-faint">
							<span class="flex items-center gap-1">
								<Icon name="download" size={11} />{formatCount(info.downloads)} 次下載
							</span>
							{#if info.follows}
								<span class="flex items-center gap-1">
									<Icon name="user" size={11} />{formatCount(info.follows)}
								</span>
							{/if}
							{#if info.updated}
								<span class="flex items-center gap-1">
									<Icon name="clock" size={11} />{formatDate(info.updated)} 更新
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
									<span class="pill accent">{name}</span>
								{/each}
								{#each info.categories as name (name)}
									<span class="pill">{name}</span>
								{/each}
							</div>
						{/if}

						{#if info.gallery.length}
							<div class="flex gap-2 overflow-x-auto pb-1">
								{#each info.gallery.slice(0, 10) as image (image.url)}
									<button
										class="shrink-0 rounded-md border border-line p-0"
										title={image.title || '開啟原圖'}
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
								<span class="text-faint">支援版本：</span>
								{info.gameVersions.slice(-12).reverse().join('、')}
							</p>
						{/if}

						{#if descriptionHtml}
							<!-- eslint-disable-next-line svelte/no-at-html-tags -->
							<div class="rich-text" use:externalLinks>{@html descriptionHtml}</div>
						{:else}
							<p class="text-faint">這個項目沒有提供詳細說明。</p>
						{/if}

						{#if info.links.length}
							<div class="flex flex-wrap gap-1.75">
								{#each info.links as link (link.url)}
									<button class="qt-button" onclick={() => openSite(link.url)}>
										<Icon name="globe" size={13} />{link.label}
									</button>
								{/each}
							</div>
						{/if}
					</div>
				{/if}
			</div>

			{#if hit}
				<div class="flex shrink-0 items-center gap-2.5">
					<label class="flex min-w-0 flex-1 items-center gap-2 text-muted">
						版本
						<select
							class="qt-select min-w-0 flex-1"
							value={version?.id ?? ''}
							onchange={(e) =>
								(version = versions.find((v) => v.id === e.currentTarget.value) ?? null)}
							disabled={busy || loadingVersions || versions.length === 0}
						>
							{#if loadingVersions}
								<option value="">讀取版本中…</option>
							{:else if versions.length === 0}
								<option value="">沒有相容的版本</option>
							{/if}
							{#each versions as v (v.id)}
								<option value={v.id}>{versionLabel(v)}</option>
							{/each}
						</select>
					</label>
					<button
						class="qt-button primary"
						disabled={busy || !version || !version.downloadable}
						onclick={install}
					>
						<Icon name="download" size={13} />安裝
					</button>
				</div>
			{/if}
		</div>
	</div>

	{#if busy || status}
		<p class="shrink-0 truncate px-0.5 text-muted" role="status">
			{busy ? '正在下載…' : status}
		</p>
	{/if}
</div>
