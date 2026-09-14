<script lang="ts">
	import Icon from '$lib/Icon.svelte';
	import type { Page } from '$lib/ui';

	let {
		pages,
		selected,
		onselect,
		width = '172px'
	}: {
		pages: Page[];
		selected: string;
		onselect: (id: string) => void;
		width?: string;
	} = $props();
</script>

<nav
	class="flex shrink-0 flex-col gap-0.5 overflow-y-auto border-r border-line bg-deep px-2 py-2.5"
	style:width
	aria-label="頁面"
>
	{#each pages as page (page.id)}
		<button
			class="flex items-center gap-2.5 rounded-md px-2.75 py-2 text-left font-medium
				transition-colors duration-150
				{selected === page.id
				? 'bg-brand-soft text-brand'
				: 'text-muted hover:bg-window hover:text-ink'}"
			onclick={() => onselect(page.id)}
			aria-current={selected === page.id}
		>
			<Icon name={page.icon} size={16} />
			<span>{page.label}</span>
		</button>
	{/each}
</nav>
