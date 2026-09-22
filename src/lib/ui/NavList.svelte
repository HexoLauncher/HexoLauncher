<script lang="ts">
	import { Tabs } from 'melt/builders';
	import Icon from './Icon.svelte';
	import type { Page } from './types';

	let {
		pages,
		selected = $bindable(''),
		width = '172px',
		onselect
	}: {
		pages: Page[];
		selected?: string;
		width?: string;
		onselect?: (id: string) => void;
	} = $props();

	const tabs = new Tabs({
		value: () => selected,
		orientation: 'vertical',
		onValueChange: (id) => {
			selected = id;
			onselect?.(id);
		}
	});
</script>

<nav
	{...tabs.triggerList}
	class="flex shrink-0 flex-col gap-0.5 overflow-y-auto border-r border-line bg-deep px-2 py-2.5"
	style:width
	aria-label="頁面"
>
	{#each pages as page (page.id)}
		<button
			{...tabs.getTrigger(page.id)}
			class="flex items-center gap-2.5 rounded-md px-2.75 py-2 text-left font-medium
				transition-colors duration-150
				{selected === page.id
				? 'bg-brand-soft text-brand-fg'
				: 'text-muted hover:bg-window hover:text-ink'}"
		>
			<Icon name={page.icon} size={16} />
			<span>{page.label}</span>
		</button>
	{/each}
</nav>
