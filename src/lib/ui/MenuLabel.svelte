<script lang="ts">
	import type { Snippet } from 'svelte';

	let { children }: { children: Snippet } = $props();
</script>

<div class="px-2.5 pt-[7px] pb-1 text-[11px] font-semibold tracking-[0.4px] text-faint uppercase">
	{@render children()}
</div>
