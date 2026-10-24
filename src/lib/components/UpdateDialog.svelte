<script lang="ts">
	import type { Update } from '@tauri-apps/plugin-updater';
	import { relaunch } from '@tauri-apps/plugin-process';
	import { Button, Dialog, Progress } from '$lib/ui';
	import * as api from '$lib/api';
	import { m } from '$lib/paraglide/messages.js';

	let { update, onclose }: { update: Update; onclose: () => void } = $props();

	let installing = $state(false);
	let downloaded = $state(0);
	let total = $state(0);
	let error = $state('');

	async function install() {
		installing = true;
		error = '';
		try {
			await update.downloadAndInstall((event) => {
				if (event.event === 'Started') total = event.data.contentLength ?? 0;
				else if (event.event === 'Progress') downloaded += event.data.chunkLength;
			});
			await relaunch();
		} catch (e) {
			error = api.errorMessage(e);
			installing = false;
		}
	}
</script>

<Dialog title={m.update_title()} width="480px" onclose={() => !installing && onclose()}>
	<div class="flex flex-col gap-3 overflow-y-auto px-5 py-4.5">
		<p>{m.update_available({ current: update.currentVersion, version: update.version })}</p>

		{#if update.body?.trim()}
			<pre
				class="max-h-60 overflow-y-auto rounded-md border border-line bg-deep px-3 py-2 font-sans text-[12.5px] whitespace-pre-wrap text-muted">{update.body.trim()}</pre>
		{/if}

		{#if installing}
			<div class="flex flex-col gap-1.5">
				<Progress value={downloaded} max={total || 1} indeterminate={!total} />
				<span class="text-faint">{m.update_downloading()}</span>
			</div>
		{/if}

		{#if error}
			<p class="leading-[1.6] text-danger" role="alert">{error}</p>
		{/if}
	</div>

	{#snippet footer()}
		<Button variant="primary" disabled={installing} onclick={install}>{m.update_install()}</Button>
		<Button disabled={installing} onclick={onclose}>{m.update_later()}</Button>
	{/snippet}
</Dialog>
