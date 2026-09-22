<script lang="ts">
	import { Progress } from 'melt/builders';

	let {
		value = 0,
		max = 100,
		indeterminate = false,
		class: klass = ''
	}: {
		value?: number;
		max?: number;
		indeterminate?: boolean;
		class?: string;
	} = $props();

	const progress = new Progress({
		value: () => value,
		max: () => max
	});
</script>

<div {...progress.root} class="h-1.5 overflow-hidden rounded-full bg-window {klass}">
	{#if indeterminate}
		<span class="block h-full w-2/5 animate-march rounded-full bg-brand"></span>
	{:else}
		<span
			{...progress.progress}
			class="block h-full rounded-full bg-brand transition-[width] duration-100 ease-linear"
			style:width="{max > 0 ? Math.min(100, (value / max) * 100) : 0}%"
		></span>
	{/if}
</div>
