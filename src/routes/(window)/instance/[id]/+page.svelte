<script lang="ts">
	import { Button } from '$lib/ui';
	import { page } from '$app/state';
	import InstanceEditor from '$lib/components/InstanceEditor.svelte';
	import * as api from '$lib/api';
	import type { InstanceView } from '$lib/api';
	import { closeSelf } from '$lib/windows';
	import { m } from '$lib/paraglide/messages.js';


	const id = $derived(page.params.id ?? '');

	let baseDir = $state('');
	let instance = $state<InstanceView | null>(null);
	let error = $state('');
	let loadedId = '';

	$effect(() => {
		if (!id || loadedId === id) return;
		loadedId = id;
		load(id);
	});

	async function load(instanceName: string) {
		error = '';
		try {
			const paths = await api.resolvePaths();
			baseDir = paths.baseDir;
			instance = await api.getInstance(baseDir, instanceName);
		} catch (e) {
			error = api.errorMessage(e);
		}
	}
</script>

<svelte:head>
	<title>
		{instance
			? m.window_edit_instance({ name: instance.name })
			: m.window_edit_instance_fallback()}
	</title>
</svelte:head>

{#if instance}
	<InstanceEditor
		{baseDir}
		{instance}
		onclose={closeSelf}
		onchanged={(updated) => (instance = updated)}
	/>
{:else if error}
	<div class="flex h-full flex-col items-center justify-center gap-3.5 text-muted">
		<h2 class="text-[17px] font-semibold text-ink">{m.instance_not_found()}</h2>
		<p>{error}</p>
		<p class="text-faint">
			{m.instance_route_param()}
			<code class="rounded-sm bg-deep px-1.5 py-0.5 font-mono"
				>{id || m.instance_route_param_empty()}</code
			>
		</p>
		<Button variant="primary" onclick={closeSelf}>{m.common_close_window()}</Button>
	</div>
{:else}
	<div class="flex h-full items-center justify-center gap-3 text-muted">
		<span class="size-5 animate-spin rounded-full border-2 border-line border-t-brand"></span>
		{m.instance_loading()}
	</div>
{/if}
