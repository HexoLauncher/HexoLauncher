<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import type { NewsItem } from '$lib/api';
	import NewsDialog from './NewsDialog.svelte';

	let {
		message,
		instanceCount,
		account
	}: {
		message: string;
		instanceCount: number;
		account: string;
	} = $props();

	let news = $state<NewsItem | null>(null);
	let showNews = $state(false);

	onMount(() => {
		api.newsLatest().then(
			(item) => (news = item),
			() => {}
		);
	});
</script>

<footer
	class="flex h-8.5 shrink-0 items-center gap-1 border-t border-line bg-window px-3 text-[12px] text-muted"
>
	{#if news}
		<button
			class="flex min-w-0 items-center gap-1.75 rounded-full bg-brand-soft px-2.75 py-1 font-medium text-brand-fg hover:brightness-110"
			title={news.title}
			onclick={() => (showNews = true)}
		>
			<Icon name="megaphone" size={13} />
			<span class="overflow-hidden text-ellipsis whitespace-nowrap">{news.title}</span>
		</button>
	{:else}
		<span
			class="flex min-w-0 items-center gap-1.75 rounded-full bg-brand-soft px-2.75 py-1 font-medium text-brand-fg"
		>
			<Icon name="megaphone" size={13} />
			<span class="overflow-hidden text-ellipsis whitespace-nowrap"
				>{m.statusbar_no_announcement()}</span
			>
		</span>
	{/if}

	<span class="min-w-0 flex-1 overflow-hidden pl-3 text-right text-ellipsis whitespace-nowrap">
		{message}
	</span>

	<span class="mx-1.25 h-4 w-px self-center bg-line"></span>
	<span class="px-1.5 whitespace-nowrap"
		>{m.statusbar_instance_count({ count: instanceCount })}</span
	>
	<span class="mx-1.25 h-4 w-px self-center bg-line"></span>
	<span class="px-1.5 whitespace-nowrap">{account}</span>
</footer>

{#if news && showNews}
	<NewsDialog {news} onclose={() => (showNews = false)} />
{/if}
