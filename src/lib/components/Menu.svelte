<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		open = false,
		onclose,
		align = 'left',
		width = '200px',
		children
	}: {
		open?: boolean;
		onclose: () => void;
		align?: 'left' | 'right';
		width?: string;
		children: Snippet;
	} = $props();
</script>

{#if open}
	<button type="button" class="qt-menu-scrim" aria-label="關閉選單" onclick={onclose}></button>
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div
		class="qt-menu"
		style:min-width={width}
		style:left={align === 'left' ? '0' : 'auto'}
		style:right={align === 'right' ? '0' : 'auto'}
		onclick={onclose}
	>
		{@render children()}
	</div>
{/if}
