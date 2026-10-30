<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from './Icon.svelte';

	let {
		title,
		icon,
		disabled = false,
		class: klass = '',
		header,
		children
	}: {
		title?: string;
		icon?: string;
		disabled?: boolean;
		class?: string;
		header?: Snippet;
		children: Snippet;
	} = $props();
</script>

<div class="relative mt-4 rounded-lg border border-line bg-group shadow-group px-3.5 pt-3.5 pb-3 {klass}">
	{#if title || header}
		<span class="mb-2.5 flex items-center gap-[9px] font-semibold text-ink">
			{#if icon}
				<Icon name={icon} size={15} />
			{/if}
			{#if title}{title}{/if}
			{@render header?.()}
		</span>
	{/if}
	<div class={disabled ? 'pointer-events-none opacity-40' : ''}>
		{@render children()}
	</div>
</div>
