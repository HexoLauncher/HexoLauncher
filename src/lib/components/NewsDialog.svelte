<script lang="ts">
	import { Button, Dialog } from '$lib/ui';
	import * as api from '$lib/api';
	import type { NewsItem } from '$lib/api';
	import { renderRichText } from '$lib/richtext';
	import { m } from '$lib/paraglide/messages.js';

	let { news, onclose }: { news: NewsItem; onclose: () => void } = $props();

	function absolutize(source: string, base: string | null) {
		if (!base) return source;
		const doc = new DOMParser().parseFromString(source, 'text/html');
		for (const [selector, attr] of [
			['a[href]', 'href'],
			['img[src]', 'src']
		] as const) {
			for (const el of doc.querySelectorAll(selector)) {
				const value = el.getAttribute(attr);
				if (!value || value.startsWith('#')) continue;
				try {
					el.setAttribute(attr, new URL(value, base).href);
				} catch {
					el.removeAttribute(attr);
				}
			}
		}
		return doc.body.innerHTML;
	}

	const html = $derived(
		news.description ? renderRichText(absolutize(news.description, news.link), 'html') : ''
	);

	const date = $derived.by(() => {
		if (!news.pubDate) return null;
		const d = new Date(news.pubDate);
		return Number.isNaN(d.getTime()) ? news.pubDate : d.toLocaleDateString();
	});

	function externalLinks(node: HTMLElement) {
		function onClick(event: MouseEvent) {
			const href = (event.target as HTMLElement | null)?.closest('a')?.getAttribute('href');
			if (!href) return;
			event.preventDefault();
			if (/^https?:\/\//i.test(href)) api.openExternal(href).catch(() => {});
		}
		node.addEventListener('click', onClick);
		return { destroy: () => node.removeEventListener('click', onClick) };
	}
</script>

<Dialog title={news.title} width="620px" {onclose}>
	<div class="flex min-h-0 flex-col gap-2.5 px-5 py-4.5">
		{#if date}
			<p class="text-[12px] text-faint">{date}</p>
		{/if}
		{#if html}
			<div class="rich-text max-h-[60vh] min-h-0 overflow-y-auto" use:externalLinks>
				{@html html}
			</div>
		{:else}
			<p class="text-muted">{m.news_no_content()}</p>
		{/if}
	</div>

	{#snippet footer()}
		{#if news.link}
			{@const link = news.link}
			<Button icon="shortcut" onclick={() => api.openExternal(link).catch(() => {})}>
				{m.news_open_in_browser()}
			</Button>
		{/if}
		<Button variant="primary" onclick={onclose}>{m.common_close()}</Button>
	{/snippet}
</Dialog>
