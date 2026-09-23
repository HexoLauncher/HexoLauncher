<script lang="ts">
	import { Button } from '$lib/ui';
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import WindowShell from '$lib/components/WindowShell.svelte';
	import * as api from '$lib/api';
	import { closeSelf } from '$lib/windows';
	import { m } from '$lib/paraglide/messages.js';

	let paths = $state<api.Paths | null>(null);
	api.resolvePaths().then((p) => (paths = p));
</script>

<svelte:head><title>{m.window_about()}</title></svelte:head>

<WindowShell>
	<div class="flex flex-1 flex-col items-center justify-center gap-2.25 px-7.5 py-6 text-center">
		<InstanceIcon name="grass" size={56} />
		<h1 class="mt-1.5 text-[20px] font-bold tracking-[-0.3px]">{m.app_name()}</h1>
		<p class="text-faint">{m.app_version({ version: '0.1.0' })}</p>
		<p class="mt-2 leading-[1.9] text-muted">{m.app_tagline()}</p>

		{#if paths}
			<dl class="mt-3 w-full text-left text-[11.5px] text-faint">
				<div class="flex gap-2 py-0.5">
					<dt class="w-20 shrink-0">{m.about_data_dir()}</dt>
					<dd class="min-w-0 truncate font-mono">{paths.baseDir}</dd>
				</div>
				<div class="flex gap-2 py-0.5">
					<dt class="w-20 shrink-0">{m.about_store_dir()}</dt>
					<dd class="min-w-0 truncate font-mono">{paths.storeDir}</dd>
				</div>
			</dl>
		{/if}
	</div>

	{#snippet footer()}
		<Button variant="primary" onclick={closeSelf}>{m.common_close()}</Button>
	{/snippet}
</WindowShell>
