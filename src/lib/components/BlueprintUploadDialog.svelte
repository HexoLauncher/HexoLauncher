<script lang="ts">
	import { Button, Dialog, Field, Spinner, TextInput } from '$lib/ui';
	import BlueprintFields from './BlueprintFields.svelte';
	import * as api from '$lib/api';
	import type { Blueprint } from '$lib/api';
	import { handleAuthError, isRelease } from '$lib/blueprints.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const MAX_IMAGES = 10;

	let {
		initialPath = '',
		initialVersion = '',
		onclose,
		onuploaded
	}: {
		initialPath?: string;

		initialVersion?: string;
		onclose: () => void;
		/** warning：藍圖上傳成功但截圖失敗時的訊息。 */
		onuploaded: (blueprint: Blueprint, warning?: string) => void;
	} = $props();

	// svelte-ignore state_referenced_locally
	let path = $state(initialPath);
	let name = $state('');
	let description = $state('');
	let tags = $state<string[]>([]);
	// svelte-ignore state_referenced_locally
	let mcVersionMin = $state(isRelease(initialVersion) ? initialVersion : '');
	let mcVersionMax = $state('');
	let images = $state<string[]>([]);

	let busy = $state('');
	let error = $state('');
	let fields = $state<ReturnType<typeof BlueprintFields>>();

	const fileName = $derived(path.split(/[\\/]/).pop() ?? '');
	const validFile = $derived(/\.(litematic|schem)$/i.test(path));

	async function pickFile() {
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const chosen = await open({
				filters: [{ name: m.blueprint_file_filter(), extensions: ['litematic', 'schem'] }]
			});
			if (typeof chosen === 'string') path = chosen;
		} catch {
			error = m.common_no_dialog();
		}
	}

	async function pickImages() {
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const chosen = await open({
				multiple: true,
				filters: [{ name: m.file_filter_image(), extensions: ['png', 'jpg', 'jpeg', 'webp'] }]
			});
			const picked = Array.isArray(chosen) ? chosen : typeof chosen === 'string' ? [chosen] : [];
			images = [...new Set([...images, ...picked])].slice(0, MAX_IMAGES);
		} catch {
			error = m.common_no_dialog();
		}
	}

	async function submit() {
		error = fields?.versionError() ?? '';
		if (error) return;

		busy = m.blueprint_busy_uploading();
		let created: Blueprint;
		try {
			created = await api.blueprintUpload({
				path,
				name: name.trim(),
				description,
				tags,
				mcVersionMin: mcVersionMin.trim(),
				mcVersionMax: mcVersionMax.trim()
			});
		} catch (e) {
			handleAuthError(e);
			error = api.errorMessage(e);
			busy = '';
			return;
		}

		if (images.length) {
			busy = m.blueprint_busy_images();
			try {
				created = await api.blueprintAddImages(created.id, images);
			} catch (e) {
				busy = '';
				onuploaded(created, api.errorMessage(e));
				return;
			}
		}
		busy = '';
		onuploaded(created);
	}
</script>

<Dialog title={m.blueprint_upload_title()} width="560px" onclose={() => !busy && onclose()}>
	<div class="flex flex-col overflow-y-auto px-5 py-4.5">
		<Field label={m.blueprint_file()} for="bp-file">
			<div class="flex gap-1.75">
				<TextInput id="bp-file" class="min-w-0 flex-1" value={path} readonly placeholder={m.blueprint_pick_file()} />
				<Button onclick={pickFile} disabled={!!busy}>{m.common_browse()}</Button>
			</div>
		</Field>

		<BlueprintFields
			bind:this={fields}
			bind:name
			bind:description
			bind:tags
			bind:mcVersionMin
			bind:mcVersionMax
			namePlaceholder={fileName.replace(/\.(litematic|schem)$/i, '') || m.blueprint_name_from_file()}
			minPlaceholder={m.blueprint_version_from_file()}
		/>

		<Field label={m.blueprint_screenshots()} class="items-start!">
			<div class="flex flex-col gap-1.5">
				{#each images as image (image)}
					<div class="flex items-center gap-2">
						<span class="min-w-0 flex-1 truncate text-muted" title={image}>{image.split(/[\\/]/).pop()}</span>
						<Button
							icon="close"
							disabled={!!busy}
							aria-label={m.common_remove()}
							onclick={() => (images = images.filter((i) => i !== image))}
						></Button>
					</div>
				{/each}
				<div>
					<Button icon="image" disabled={!!busy || images.length >= MAX_IMAGES} onclick={pickImages}>
						{m.blueprint_add_screenshots()}
					</Button>
				</div>
				<p class="text-faint">{m.blueprint_screenshots_hint()}</p>
			</div>
		</Field>

		{#if error}
			<p class="mt-1 leading-[1.6] text-danger" role="alert">{error}</p>
		{/if}
	</div>

	{#snippet footer()}
		{#if busy}
			<span class="mr-auto flex items-center gap-2 text-muted"><Spinner />{busy}</span>
		{/if}
		<Button variant="primary" icon="upload" disabled={!!busy || !validFile} onclick={submit}>
			{m.blueprint_upload()}
		</Button>
		<Button disabled={!!busy} onclick={onclose}>{m.common_cancel()}</Button>
	{/snippet}
</Dialog>
