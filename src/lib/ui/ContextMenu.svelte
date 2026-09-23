<script lang="ts">
	import type { Snippet } from 'svelte';
	import { setContext } from 'svelte';
	import { MENU_CTX, type MenuContext } from './menu-context';
	import { m } from '$lib/paraglide/messages.js';

	let {
		x,
		y,
		width = '184px',
		onclose,
		children
	}: {
		x: number;
		y: number;
		width?: string;
		onclose: () => void;
		children: Snippet;
	} = $props();

	setContext<MenuContext>(MENU_CTX, { close: () => onclose() });
</script>

<svelte:window
	onkeydown={(e) => {
		if (e.key === 'Escape') onclose();
	}}
/>

<button type="button" class="fixed inset-0 z-199 cursor-default" aria-label={m.common_close_menu()} onclick={onclose}
></button>

<div
	class="fixed z-200 rounded-lg border border-line bg-raised p-1.5 shadow-md"
	style:min-width={width}
	style:left="{x}px"
	style:top="{y}px"
	role="menu"
	tabindex="-1"
>
	{@render children()}
</div>
