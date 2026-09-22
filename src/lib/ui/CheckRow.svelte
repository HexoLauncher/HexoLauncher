<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		compact = false,
		disabled = false,
		class: klass = '',
		children
	}: {
		compact?: boolean;
		disabled?: boolean;
		class?: string;
		children: Snippet;
	} = $props();
</script>

<label
	class="flex cursor-pointer items-center gap-[9px] {compact
		? 'min-h-0 py-0'
		: 'min-h-[26px] py-1'} {klass}"
	class:text-faint={disabled}
>
	{@render children()}
</label>
