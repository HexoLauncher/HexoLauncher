<script lang="ts">
	import { Button } from '$lib/ui';
	import AddInstancePanel from '$lib/components/AddInstancePanel.svelte';
	import * as api from '$lib/api';
	import { closeSelf } from '$lib/windows';
	import { m } from '$lib/paraglide/messages.js';


	let baseDir = $state('');
	let groups = $state<string[]>([]);
	let created = $state<string | null>(null);
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

{#if created}
	<div class="flex h-full flex-col items-center justify-center gap-3 text-muted">
		<h2 class="text-[17px] font-semibold text-ink">{m.add_created_title({ name: created })}</h2>
		<p>{m.add_created_desc()}</p>
		<Button variant="primary" onclick={closeSelf}>{m.common_close_window()}</Button>
	</div>
{:else if error}
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
		oncreated={(instance) => (created = instance.name)}
	/>
{/if}
