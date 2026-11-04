<script lang="ts">
	import { parseMcText } from '$lib/mctext';

	let { text, class: klass = '' }: { text: string; class?: string } = $props();

	const segments = $derived(parseMcText(text));
</script>

<span class={klass}
	>{#each segments as segment, i (i)}<span
			style:color={segment.color}
			class:font-bold={segment.bold}
			class:italic={segment.italic}
			class:underline={segment.underline}
			class:line-through={segment.strike}>{segment.text}</span
		>{/each}</span
>
