<script lang="ts">
	import { blockIcon } from '$lib/icons';

	let {
		name = 'grass',
		image = null,
		size = 48
	}: { name?: string; image?: string | null; size?: number } = $props();

	const custom = $derived(name === 'custom' && !!image);
	const src = $derived(custom ? image! : blockIcon(name));

	let natural = $state(0);
</script>

<img
	{src}
	alt=""
	width={size}
	height={size}
	draggable="false"
	class="block object-contain {custom && natural && natural < size
		? '[image-rendering:pixelated]'
		: ''}"
	onload={(e) => (natural = (e.currentTarget as HTMLImageElement).naturalWidth)}
	style:width="{size}px"
	style:height="{size}px"
/>
