<script lang="ts">
	import { onMount } from 'svelte';
	import { Button, Dialog, Pill, Select, Spinner } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import BlueprintEditDialog from './BlueprintEditDialog.svelte';
	import * as api from '$lib/api';
	import type { Blueprint } from '$lib/api';
	import { formatCount, formatSize, releaseDate } from '$lib/format';
	import { handleAuthError, isOwner, supports, tagName, versionRange } from '$lib/blueprints.svelte';
	import { app, report, reportError } from '$lib/state.svelte';
	import { openAppWindow, windowSpecs } from '$lib/windows';
	import { m } from '$lib/paraglide/messages.js';

	let {
		blueprint,
		onchanged,
		ondeleted
	}: {
		blueprint: Blueprint;
		onchanged: (updated: Blueprint) => void;
		ondeleted: (id: string) => void;
	} = $props();

	const RENDER_LABELS: Record<api.RenderStatus, string> = {
		pending: m.blueprint_render_pending(),
		rendering: m.blueprint_render_rendering(),
		done: m.blueprint_render_done(),
		failed: m.blueprint_render_failed(),
		unsupported: m.blueprint_render_unsupported()
	};

	// svelte-ignore state_referenced_locally
	let shown = $state(blueprint.cover.url);
	let target = $state(defaultTarget());
	let busy = $state('');
	let editing = $state(false);
	let confirmingDelete = $state(false);
	/** 封面區改成顯示 3D 預覽。 */
	let previewing = $state(false);

	const mine = $derived(isOwner(blueprint));
	const working = $derived(blueprint.render.status === 'pending' || blueprint.render.status === 'rendering');
	const gallery = $derived(
		[
			...(blueprint.render.url ? [{ id: 'render', url: blueprint.render.url }] : []),
			...blueprint.images.map((i) => ({ id: i.id, url: i.url }))
		]
	);
	const targetInstance = $derived(app.instances.find((i) => i.id === target) ?? null);
	const targetFit = $derived(targetInstance ? supports(blueprint, targetInstance.versionId) : null);

	function defaultTarget(): string {
		const fits = (id: string | null) => {
			const inst = app.instances.find((i) => i.id === id);
			return inst && supports(blueprint, inst.versionId) !== false ? inst.id : null;
		};
		return (
			fits(app.selectedId) ??
			app.instances.find((i) => supports(blueprint, i.versionId) === true)?.id ??
			app.selectedId ??
			app.instances[0]?.id ??
			''
		);
	}

	// 封面還在背景渲染時，每隔幾秒問一次。
	onMount(() => {
		const timer = setInterval(async () => {
			if (!working) return;
			try {
				const updated = await api.blueprintGet(blueprint.id);
				if (updated.cover.url !== blueprint.cover.url) shown = updated.cover.url;
				onchanged(updated);
			} catch {
				// 下一輪再試
			}
		}, 4000);
		return () => clearInterval(timer);
	});

	async function run(label: string, action: () => Promise<void>) {
		busy = label;
		try {
			await action();
		} finally {
			busy = '';
		}
	}

	function install() {
		const inst = targetInstance;
		if (!inst) return;
		return run(m.blueprint_busy_downloading(), async () => {
			try {
				const result = await api.blueprintInstall(app.baseDir, inst.id, blueprint.id);
				report(
					result.existed
						? m.blueprint_status_already_installed({ file: result.fileName, instance: inst.name })
						: m.blueprint_status_installed({ file: result.fileName, instance: inst.name })
				);
			} catch (e) {
				reportError(m.blueprint_error_download(), e);
			}
		});
	}

	async function saveAs() {
		let path: string | null = null;
		try {
			const { save } = await import('@tauri-apps/api/dialog');
			path = await save({
				defaultPath: blueprint.file.name,
				filters: [{ name: m.blueprint_file_filter(), extensions: [blueprint.format] }]
			});
		} catch {
			reportError(m.blueprint_error_download(), m.common_no_dialog());
			return;
		}
		if (!path) return;
		const target = path;
		await run(m.blueprint_busy_downloading(), async () => {
			try {
				await api.blueprintSaveAs(blueprint.id, target);
				report(m.blueprint_status_saved({ path: target }));
			} catch (e) {
				reportError(m.blueprint_error_download(), e);
			}
		});
	}

	function popOut() {
		previewing = false;
		openAppWindow(windowSpecs.viewer({ kind: 'remote', id: blueprint.id }, blueprint.name));
	}

	function rerender() {
		return run(m.blueprint_busy_saving(), async () => {
			try {
				onchanged(await api.blueprintRerender(blueprint.id));
				report(m.blueprint_status_rerender({ name: blueprint.name }));
			} catch (e) {
				handleAuthError(e);
				reportError(m.blueprint_error_save(), e);
			}
		});
	}

	function remove() {
		confirmingDelete = false;
		return run(m.blueprint_busy_saving(), async () => {
			try {
				await api.blueprintDelete(blueprint.id);
				report(m.blueprint_status_deleted({ name: blueprint.name }));
				ondeleted(blueprint.id);
			} catch (e) {
				handleAuthError(e);
				reportError(m.blueprint_error_save(), e);
			}
		});
	}
</script>

<div class="flex min-h-0 flex-1 flex-col gap-2.5">
	<div class="min-h-0 flex-1 overflow-y-auto rounded-lg border border-line bg-deep">
		<div class="relative aspect-[16/10] w-full bg-card">
			{#if previewing}
				<!-- 預覽器（含 wasm）只在需要時才載入 -->
				{#await import('./SchematicViewer.svelte')}
					<span class="absolute inset-0 grid place-items-center"><Spinner size={20} /></span>
				{:then { default: SchematicViewer }}
					<div class="absolute inset-0">
						<SchematicViewer
							schematic={{ kind: 'remote', id: blueprint.id, blueprint }}
							preferInstance={target || null}
							compact
						/>
					</div>
				{/await}
				<div class="absolute top-2 right-2 flex gap-1">
					<button
						class="grid size-7 place-items-center rounded-md bg-window/85 text-muted hover:text-ink"
						title={m.blueprint_preview_pop_out()}
						aria-label={m.blueprint_preview_pop_out()}
						onclick={popOut}
					>
						<Icon name="shortcut" size={14} />
					</button>
					<button
						class="grid size-7 place-items-center rounded-md bg-window/85 text-muted hover:text-ink"
						title={m.blueprint_preview_close()}
						aria-label={m.blueprint_preview_close()}
						onclick={() => (previewing = false)}
					>
						<Icon name="close" size={14} />
					</button>
				</div>
			{:else if shown}
				<img src={shown} alt={blueprint.name} class="absolute inset-0 size-full object-contain" />
			{:else}
				<span class="absolute inset-0 grid place-items-center text-faint">
					<span class="flex flex-col items-center gap-2">
						{#if working}<Spinner size={20} />{:else}<Icon name="blocks" size={34} />{/if}
						{RENDER_LABELS[blueprint.render.status]}
					</span>
				</span>
			{/if}
		</div>

		{#if gallery.length > 1}
			<div class="flex gap-1.5 overflow-x-auto px-3 pt-2">
				{#each gallery as image (image.id)}
					<button
						class="shrink-0 overflow-hidden rounded-md border-2 p-0
							{!previewing && shown === image.url ? 'border-brand' : 'border-transparent'}"
						onclick={() => {
							shown = image.url;
							previewing = false;
						}}
						aria-label={image.id === 'render' ? m.blueprint_render_image() : m.blueprint_screenshot()}
					>
						<img src={image.url} alt="" loading="lazy" class="h-12 w-19 object-cover" />
					</button>
				{/each}
			</div>
		{/if}

		<div class="flex flex-col gap-3 px-3.5 py-3">
			<div>
				<h3 class="text-[15px] font-semibold break-words text-ink">{blueprint.name}</h3>
				<p class="text-faint">
					{m.content_by_author({ author: blueprint.owner.username })} · {releaseDate(blueprint.createdAt)}
				</p>
			</div>

			{#if blueprint.tags.length}
				<div class="flex flex-wrap gap-1.5">
					{#each blueprint.tags as slug (slug)}
						<Pill accent>{tagName(slug)}</Pill>
					{/each}
				</div>
			{/if}

			<dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-muted">
				{#if versionRange(blueprint)}
					<dt class="text-faint">{m.blueprint_versions()}</dt>
					<dd>{versionRange(blueprint)}</dd>
				{/if}
				{#if blueprint.size}
					<dt class="text-faint">{m.blueprint_dimensions()}</dt>
					<dd class="font-mono text-[11.5px]">{blueprint.size.join(' × ')}</dd>
				{/if}
				{#if blueprint.totalBlocks !== null}
					<dt class="text-faint">{m.blueprint_total_blocks()}</dt>
					<dd>{blueprint.totalBlocks.toLocaleString()}</dd>
				{/if}
				<dt class="text-faint">{m.blueprint_file()}</dt>
				<dd class="min-w-0 truncate" title={blueprint.file.name}>
					{blueprint.file.name} · {formatSize(blueprint.file.size)}
				</dd>
				<dt class="text-faint">{m.blueprint_downloads()}</dt>
				<dd>{formatCount(blueprint.downloadCount)}</dd>
				{#if blueprint.render.status === 'failed' && blueprint.render.error}
					<dt class="text-faint">{m.blueprint_render()}</dt>
					<dd class="text-danger">{blueprint.render.error}</dd>
				{/if}
			</dl>

			{#if blueprint.description.trim()}
				<p class="leading-[1.75] break-words whitespace-pre-wrap text-ink">{blueprint.description}</p>
			{:else}
				<p class="text-faint">{m.content_no_description()}</p>
			{/if}

			{#if mine}
				<div class="flex flex-wrap gap-1.75 border-t border-line pt-3">
					<Button icon="edit" disabled={!!busy} onclick={() => (editing = true)}>{m.blueprint_edit()}</Button>
					{#if blueprint.format === 'litematic'}
						<Button
							icon="rerender"
							disabled={!!busy || blueprint.render.status === 'rendering'}
							onclick={rerender}
						>
							{m.blueprint_rerender()}
						</Button>
					{/if}
					<Button variant="danger" icon="trash" disabled={!!busy} onclick={() => (confirmingDelete = true)}>
						{m.common_delete()}
					</Button>
				</div>
			{/if}
		</div>
	</div>

	<div class="flex shrink-0 flex-col gap-1.75">
		<div class="flex items-center gap-1.75">
			<Select
				class="min-w-0 flex-1"
				bind:value={target}
				disabled={!!busy || app.instances.length === 0}
				aria-label={m.blueprint_target_instance()}
			>
				{#if app.instances.length === 0}
					<option value="">{m.blueprint_no_instances()}</option>
				{/if}
				{#each app.instances as inst (inst.id)}
					<option value={inst.id}>
						{inst.name} ({inst.versionId}){supports(blueprint, inst.versionId) === false ? ` · ${m.blueprint_unsupported_short()}` : ''}
					</option>
				{/each}
			</Select>
			<Button variant="primary" icon="download" disabled={!!busy || !targetInstance} onclick={install}>
				{m.blueprint_install()}
			</Button>
		</div>
		<div class="flex items-center gap-1.75">
			{#if busy}
				<span class="flex min-w-0 flex-1 items-center gap-2 text-muted"><Spinner />{busy}</span>
			{:else if targetFit === false}
				<span class="min-w-0 flex-1 truncate text-danger">
					{m.blueprint_version_mismatch({ version: targetInstance?.versionId ?? '' })}
				</span>
			{:else}
				<span class="flex-1"></span>
			{/if}
			<Button icon="download" disabled={!!busy} onclick={saveAs}>{m.blueprint_save_as()}</Button>
			<Button
				icon="eye"
				disabled={blueprint.format !== 'litematic'}
				title={blueprint.format !== 'litematic' ? m.blueprint_preview_litematic_only() : undefined}
				onclick={() => (previewing = !previewing)}
			>
				{previewing ? m.blueprint_preview_close() : m.blueprint_preview()}
			</Button>
		</div>
	</div>
</div>

{#if editing}
	<BlueprintEditDialog
		{blueprint}
		onclose={() => (editing = false)}
		onchanged={(updated) => {
			if (updated.cover.url !== blueprint.cover.url) shown = updated.cover.url;
			onchanged(updated);
		}}
	/>
{/if}

{#if confirmingDelete}
	<Dialog title={m.blueprint_delete_title()} width="420px" onclose={() => (confirmingDelete = false)}>
		<div class="flex gap-3.75 px-5 py-5.5">
			<span class="grid size-11.5 shrink-0 place-items-center rounded-lg bg-danger-soft text-danger">
				<Icon name="trash" size={30} />
			</span>
			<div>
				<p class="mb-2 text-[15px] font-semibold">{m.blueprint_delete_confirm({ name: blueprint.name })}</p>
				<p class="leading-[1.75] text-muted">{m.blueprint_delete_desc()}</p>
			</div>
		</div>
		{#snippet footer()}
			<Button variant="destructive" onclick={remove}>{m.common_delete()}</Button>
			<Button onclick={() => (confirmingDelete = false)}>{m.common_cancel()}</Button>
		{/snippet}
	</Dialog>
{/if}
