<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		accent = false,
		class: klass = '',
		children
	}: {
		accent?: boolean;
		class?: string;
		children: Snippet;
	} = $props();
</script>

<span
	class="inline-flex items-center gap-[5px] rounded-full px-2.5 py-[3px] text-[11.5px] font-medium whitespace-nowrap
		{accent ? 'bg-brand-soft text-brand-fg' : 'bg-btn text-muted'} {klass}"
>
	{@render children()}
</span>
