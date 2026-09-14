<script lang="ts">
	import type { Snippet } from 'svelte';


	let { children, footer }: { children: Snippet; footer?: Snippet } = $props();
</script>

<div class="flex h-full flex-col bg-window">
	<div class="flex min-h-0 flex-1 flex-col overflow-hidden">
		{@render children()}
	</div>
	{#if footer}
		<div class="flex shrink-0 justify-end gap-2 border-t border-line px-4 py-3">
			{@render footer()}
		</div>
	{/if}
</div>
