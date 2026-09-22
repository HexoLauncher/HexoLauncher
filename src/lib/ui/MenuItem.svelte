<script lang="ts">
	import type { Snippet } from 'svelte';
	import { getContext } from 'svelte';
	import Icon from './Icon.svelte';
	import { MENU_CTX, type MenuContext } from './menu-context';

	let {
		icon,
		checked,
		shortcut,
		danger = false,
		disabled = false,
		onclick,
		children
	}: {
		icon?: string;
		checked?: boolean;
		shortcut?: string;
		danger?: boolean;
		disabled?: boolean;
		onclick?: () => void;
		children: Snippet;
	} = $props();

	const menu = getContext<MenuContext | undefined>(MENU_CTX);
</script>

<button
	type="button"
	{disabled}
	class="flex w-full items-center gap-2.5 rounded-md px-2.5 py-[7px] text-left transition-colors duration-150 disabled:text-faint
		{danger
		? 'not-disabled:hover:bg-danger-soft not-disabled:hover:text-danger'
		: 'not-disabled:hover:bg-brand-soft not-disabled:hover:text-brand-fg'}"
	onclick={() => {
		menu?.close();
		onclick?.();
	}}
>
	{#if icon}
		<Icon name={icon} size={14} />
	{:else if checked !== undefined}
		<span class="w-[13px] shrink-0 text-center text-brand-fg">
			{#if checked}<Icon name="check" size={12} class="inline" />{/if}
		</span>
	{/if}
	{@render children()}
	{#if shortcut}
		<span class="ml-auto pl-5 text-faint">{shortcut}</span>
	{/if}
</button>
