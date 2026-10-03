<script lang="ts">
	import { page } from '$app/state';
	import SchematicViewer from '$lib/components/SchematicViewer.svelte';
	import { decodeRef } from '$lib/windows';
	import { m } from '$lib/paraglide/messages.js';

	const ref = page.params.ref ?? '';
	const schematic =
		page.params.kind === 'remote'
			? ({ kind: 'remote', id: ref } as const)
			: ({ kind: 'local', path: decodeRef(ref) } as const);
	const preferInstance = page.params.instance ? decodeURIComponent(page.params.instance) : null;

	let title = $state('');
</script>

<svelte:head><title>{m.window_viewer({ name: title || '…' })}</title></svelte:head>

<div class="h-full">
	<SchematicViewer {schematic} {preferInstance} bind:title />
</div>
