<script lang="ts">
	import { Button } from '$lib/ui';
	import AddInstancePanel from '$lib/components/AddInstancePanel.svelte';
	import * as api from '$lib/api';
	import { closeSelf } from '$lib/windows';
	import { m } from '$lib/paraglide/messages.js';


	let baseDir = $state('');
	let groups = $state<string[]>([]);
	let error = $state('');

	async function load() {
		try {
			const paths = await api.resolvePaths();
			baseDir = paths.baseDir;
			const instances = await api.listInstances(baseDir);
			groups = instances
				.map((i) => i.group)
				.filter((g, i, all) => g !== '' && all.indexOf(g) === i)
				.sort();
		} catch (e) {
			error = api.errorMessage(e);
		}
	}

	load();
</script>

<svelte:head><title>{m.window_new_instance_title()}</title></svelte:head>

{#if error}
	<div class="flex h-full flex-col items-center justify-center gap-3 text-muted">
		<h2 class="text-[17px] font-semibold text-ink">{m.error_load_config()}</h2>
		<p>{error}</p>
		<Button onclick={load}>{m.common_retry()}</Button>
	</div>
{:else if baseDir}
	<AddInstancePanel
		{baseDir}
		{groups}
		onclose={closeSelf}
		oncreated={closeSelf}
	/>
{/if}
