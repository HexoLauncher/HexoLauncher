<script lang="ts">
	import type { Snippet } from 'svelte';
	import { setContext } from 'svelte';
	import { Popover } from 'melt/builders';
	import type { Placement } from '@floating-ui/dom';
	import { MENU_CTX, type MenuContext } from './menu-context';

	let {
		width = '200px',
		placement = 'bottom-start',
		trigger,
		children
	}: {
		width?: string;
		placement?: Placement;
		trigger: Snippet<[Record<string, unknown>]>;
		children: Snippet;
	} = $props();

	const popover = new Popover({
		floatingConfig: () => ({ computePosition: { placement }, offset: { mainAxis: 2 } })
	});

	setContext<MenuContext>(MENU_CTX, {
		close: () => (popover.open = false)
	});
</script>

{@render trigger(popover.trigger)}

<div
	{...popover.content}
	class="z-200 rounded-lg border border-line bg-raised p-1.5 shadow-md"
	style:min-width={width}
>
	{@render children()}
</div>
