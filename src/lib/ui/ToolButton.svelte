<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';
	import Icon from './Icon.svelte';

	let {
		icon,
		iconSize,
		layout = 'under',
		active = false,
		class: klass = '',
		children,
		...rest
	}: {
		icon?: string;
		iconSize?: number;
		layout?: 'under' | 'beside' | 'bare';
		active?: boolean;
		class?: string;
		children?: Snippet;
	} & HTMLButtonAttributes = $props();

	const LAYOUT = {
		under: 'min-w-[68px] flex-col gap-1 px-2.5 pt-[7px] pb-1.5',
		beside: 'w-full gap-2.5 px-2.5 py-2 text-left',
		bare: ''
	};
</script>

<button
	type="button"
	{...rest}
	class="flex items-center rounded-md border border-transparent transition-colors duration-150
		not-disabled:active:bg-btn-lo disabled:text-faint {LAYOUT[layout]}
		{active
		? 'bg-brand-soft text-brand-fg hover:bg-brand-soft-hi hover:text-brand-fg'
		: 'text-muted not-disabled:hover:bg-btn-hi not-disabled:hover:text-ink'} {klass}"
>
	{#if icon}
		<Icon name={icon} size={iconSize ?? (layout === 'under' ? 22 : 16)} />
	{/if}
	{@render children?.()}
</button>
