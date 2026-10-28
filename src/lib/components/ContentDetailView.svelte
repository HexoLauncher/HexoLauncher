<script lang="ts">
	import { Button, Pill } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import * as api from '$lib/api';
	import type { ContentDetail, ContentLink } from '$lib/api';
	import { formatCount, formatDate } from '$lib/format';
	import { m } from '$lib/paraglide/messages.js';
	import { renderRichText } from '$lib/richtext';

	let {
		detail,
		loading,
		error,
		author = '',
		placeholder,
		onerror
	}: {
		detail: ContentDetail | null;
		loading: boolean;
		error: string;
		author?: string;
		placeholder: string;
		onerror: (message: string) => void;
	} = $props();

	const descriptionHtml = $derived(
		detail?.description ? renderRichText(detail.description, detail.descriptionFormat) : ''
	);

	const LINK_LABELS: Record<ContentLink['kind'], string> = {
		website: m.common_website(),
		source: m.content_link_source(),
		issues: m.content_link_issues(),
		wiki: 'Wiki',
		discord: 'Discord'
	};

	function openSite(url: string) {
		api.openExternal(url).catch((e) => onerror(api.errorMessage(e)));
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

<div class="min-h-0 flex-1 overflow-y-auto rounded-lg border border-line bg-deep px-3.5 py-3">
	{#if loading}
		<p class="grid h-full place-items-center text-muted">
			<span class="flex items-center gap-2">
				<span
					class="size-4 animate-spin rounded-full border-2 border-line border-t-brand"
				></span>
				{m.content_loading_detail()}
			</span>
		</p>
	{:else if error}
		<p class="leading-[1.7] text-danger">{error}</p>
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
					{#if author}
						<p class="truncate text-faint">{m.content_by_author({ author })}</p>
					{/if}
				</div>
			</div>

			{#if info.summary}
				<p class="leading-[1.7] text-ink">{info.summary}</p>
			{/if}

			<div class="flex flex-wrap items-center gap-x-3 gap-y-1 text-faint">
				{#if info.downloads}
					<span class="flex items-center gap-1">
						<Icon name="download" size={11} />{m.content_downloads({ count: formatCount(info.downloads) })}
					</span>
				{/if}
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
							<Icon name="globe" size={13} />{LINK_LABELS[link.kind] ?? link.kind}
						</Button>
					{/each}
				</div>
			{/if}
		</div>
	{:else}
		<p class="grid h-full place-items-center text-center text-faint">{placeholder}</p>
	{/if}
</div>
