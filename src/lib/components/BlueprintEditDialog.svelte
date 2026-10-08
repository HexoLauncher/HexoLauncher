<script lang="ts">
	import { Button, Dialog, Field, Spinner } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import BlueprintFields from './BlueprintFields.svelte';
	import * as api from '$lib/api';
	import type { Blueprint } from '$lib/api';
	import { handleAuthError } from '$lib/blueprints.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const MAX_IMAGES = 10;

	let {
		blueprint,
		onclose,
		onchanged
	}: {
		blueprint: Blueprint;
		onclose: () => void;
		onchanged: (updated: Blueprint) => void;
	} = $props();

	// svelte-ignore state_referenced_locally
	let name = $state(blueprint.name);
	// svelte-ignore state_referenced_locally
	let description = $state(blueprint.description);
	// svelte-ignore state_referenced_locally
	let tags = $state([...blueprint.tags]);
	// svelte-ignore state_referenced_locally
	let mcVersionMin = $state(blueprint.mcVersions.min ?? '');
	// svelte-ignore state_referenced_locally
	let mcVersionMax = $state(blueprint.mcVersions.max ?? '');

	let busy = $state('');
	let error = $state('');
	let fields = $state<ReturnType<typeof BlueprintFields>>();

	async function apply(action: () => Promise<Blueprint>) {
		busy = m.blueprint_busy_saving();
		error = '';
		try {
			const updated = await action();
			onchanged(updated);
			return updated;
		} catch (e) {
			handleAuthError(e);
			error = api.errorMessage(e);
			return null;
		} finally {
			busy = '';
		}
	}

	async function addImages() {
		let picked: string[] = [];
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const chosen = await open({
				multiple: true,
				filters: [{ name: m.file_filter_image(), extensions: ['png', 'jpg', 'jpeg', 'webp'] }]
			});
			picked = Array.isArray(chosen) ? chosen : typeof chosen === 'string' ? [chosen] : [];
		} catch {
			error = m.common_no_dialog();
			return;
		}
		if (!picked.length) return;
		const room = MAX_IMAGES - blueprint.images.length;
		await apply(() => api.blueprintAddImages(blueprint.id, picked.slice(0, Math.max(0, room))));
	}

	async function save() {
		error = fields?.versionError() ?? '';
		if (error || !name.trim()) return;
		const updated = await apply(() =>
			api.blueprintUpdate(blueprint.id, {
				name: name.trim(),
				description,
				tags,
				mcVersionMin: mcVersionMin.trim(),
				mcVersionMax: mcVersionMax.trim()
			})
		);
		if (updated) onclose();
	}
</script>

<Dialog title={m.blueprint_edit_title({ name: blueprint.name })} width="600px" onclose={() => !busy && onclose()}>
	<div class="flex flex-col overflow-y-auto px-5 py-4.5">
		<BlueprintFields
			bind:this={fields}
			bind:name
			bind:description
			bind:tags
			bind:mcVersionMin
			bind:mcVersionMax
		/>

		<Field label={m.blueprint_screenshots()} class="items-start!">
			<div class="flex flex-col gap-2">
				<div class="grid grid-cols-[repeat(auto-fill,minmax(118px,1fr))] gap-2">
					{#each blueprint.images as image (image.id)}
						{@const isCover = blueprint.cover.imageId === image.id}
						<div
							class="overflow-hidden rounded-md border {isCover ? 'border-brand' : 'border-line'} bg-deep"
						>
							<img src={image.url} alt="" loading="lazy" class="aspect-[16/10] w-full object-cover" />
							<div class="flex items-center gap-1 px-1.5 py-1">
								{#if isCover}
									<span class="flex-1 text-[11.5px] text-brand-fg">{m.blueprint_is_cover()}</span>
								{:else}
									<button
										class="flex-1 text-left text-[11.5px] text-muted hover:text-ink disabled:text-faint"
										disabled={!!busy}
										onclick={() => apply(() => api.blueprintSetCover(blueprint.id, image.id))}
									>
										{m.blueprint_set_cover()}
									</button>
								{/if}
								<button
									class="grid size-5.5 place-items-center rounded-sm text-muted hover:bg-danger hover:text-danger-ink disabled:text-faint"
									disabled={!!busy}
									aria-label={m.common_delete()}
									onclick={() => apply(() => api.blueprintDeleteImage(blueprint.id, image.id))}
								>
									<Icon name="trash" size={12} />
								</button>
							</div>
						</div>
					{/each}
				</div>
				<div class="flex flex-wrap gap-1.75">
					<Button icon="image" disabled={!!busy || blueprint.images.length >= MAX_IMAGES} onclick={addImages}>
						{m.blueprint_add_screenshots()}
					</Button>
					{#if blueprint.cover.source === 'image'}
						<Button disabled={!!busy} onclick={() => apply(() => api.blueprintSetCover(blueprint.id, null))}>
							{m.blueprint_cover_use_render()}
						</Button>
					{/if}
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
		<Button variant="primary" disabled={!!busy || !name.trim()} onclick={save}>{m.common_save()}</Button>
		<Button disabled={!!busy} onclick={onclose}>{m.common_cancel()}</Button>
	{/snippet}
</Dialog>
