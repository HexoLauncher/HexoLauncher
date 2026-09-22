<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		class: klass = '',
		children
	}: {
		class?: string;
		children: Snippet;
	} = $props();
</script>

<div class="min-h-0 flex-1 overflow-auto rounded-lg border border-line bg-deep px-1 pb-1 {klass}">
	{@render children()}
</div>
