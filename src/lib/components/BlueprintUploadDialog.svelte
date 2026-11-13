<script lang="ts">
	import { Button, Dialog, Field, Select, Spinner, TextInput } from '$lib/ui';
	import BlueprintFields from './BlueprintFields.svelte';
	import * as api from '$lib/api';
	import type { Blueprint, SchematicFile } from '$lib/api';
	import { handleAuthError, isRelease } from '$lib/blueprints.svelte';
	import { app } from '$lib/state.svelte';
	import { formatSize } from '$lib/format';
	import { buildTree, type TreeDir } from '$lib/schematicTree';
	import { SvelteSet } from 'svelte/reactivity';
	import Icon from '$lib/ui/Icon.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const MAX_IMAGES = 10;
	const SOURCES: ['file' | 'instance', () => string][] = [
		['file', m.blueprint_source_file],
		['instance', m.blueprint_source_instance]
	];

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
	// svelte-ignore state_referenced_locally
	let source = $state<'file' | 'instance'>(initialPath || app.instances.length === 0 ? 'file' : 'instance');
	let pickedFile = $state('');
	let instanceId = $state(app.selectedId ?? app.instances[0]?.id ?? '');
	let instanceFiles = $state<SchematicFile[] | null>(null);
	let autoVersion = '';
	let fileQuery = $state('');
	const matchedFiles = $derived.by(() => {
		const needle = fileQuery.trim().toLowerCase();
		const list = instanceFiles ?? [];
		return needle ? list.filter((f) => f.relPath.toLowerCase().includes(needle)) : list;
	});
	const fileTree = $derived(buildTree(matchedFiles));
	const expanded = new SvelteSet<string>();
	const searching = $derived(!!fileQuery.trim());

	function toggleDir(dirPath: string) {
		if (searching) return;
		if (expanded.has(dirPath)) expanded.delete(dirPath);
		else expanded.add(dirPath);
	}
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

	const instance = $derived(app.instances.find((i) => i.id === instanceId) ?? null);

	$effect(() => {
		if (source === 'instance' && instanceId) loadInstance(instanceId);
	});

	let seq = 0;
	async function loadInstance(id: string) {
		const current = ++seq;
		instanceFiles = null;
		fileQuery = '';
		expanded.clear();
		path = '';
		pickedFile = '';
		let found: SchematicFile[] = [];
		try {
			found = await api.listSchematics(app.baseDir, id);
		} catch (e) {
			error = api.errorMessage(e);
		}
		if (current !== seq) return;
		instanceFiles = found;
		applyInstanceVersion();
	}

	function applyInstanceVersion() {
		const version = instance?.versionId ?? '';
		if (!isRelease(version)) return;
		if (!mcVersionMin.trim() || mcVersionMin === autoVersion) {
			mcVersionMin = version;
			autoVersion = version;
		}
	}

	function selectInstanceFile(relPath: string) {
		pickedFile = relPath;
		path = instanceFiles?.find((f) => f.relPath === pickedFile)?.path ?? '';
	}

	function setSource(next: 'file' | 'instance') {
		if (source === next) return;
		source = next;
		path = '';
		pickedFile = '';
		error = '';
	}

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

{#snippet branch(node: TreeDir, depth: number)}
	{#each node.dirs as dir (dir.path)}
		{@const open = searching || expanded.has(dir.path)}
		<button
			class="flex w-full items-center gap-1.5 rounded-md py-1.25 pr-2 text-left transition-colors duration-150 hover:bg-hover"
			style="padding-left:{depth * 16 + 6}px"
			aria-expanded={open}
			title={dir.path}
			disabled={!!busy}
			onclick={() => toggleDir(dir.path)}
		>
			<Icon name="chevron" size={12} class="shrink-0 text-faint transition-transform duration-150 {open ? 'rotate-90' : ''}" />
			<Icon name="folder" size={14} class="shrink-0 text-muted" />
			<span class="min-w-0 flex-1 truncate">{dir.name}</span>
			<span class="shrink-0 text-[11.5px] text-faint">{m.blueprint_folder_count({ count: dir.count })}</span>
		</button>
		{#if open}
			{@render branch(dir, depth + 1)}
		{/if}
	{/each}
	{#each node.files as f (f.relPath)}
		{@const active = pickedFile === f.relPath}
		<button
			class="flex w-full items-center gap-1.5 rounded-md py-1.25 pr-2 text-left transition-colors duration-150
				{active ? 'bg-brand-soft text-ink shadow-[inset_2px_0_0_var(--color-brand)]' : 'hover:bg-hover'}"
			style="padding-left:{depth * 16 + 6}px"
			aria-pressed={active}
			title={f.relPath}
			disabled={!!busy}
			onclick={() => selectInstanceFile(f.relPath)}
		>
			<span class="w-3 shrink-0"></span>
			<Icon name="blocks" size={14} class="shrink-0 text-muted" />
			<span class="min-w-0 flex-1 truncate">{f.name}</span>
			<span class="shrink-0 text-[11.5px] text-faint">{formatSize(f.size)}</span>
		</button>
	{/each}
{/snippet}

<Dialog title={m.blueprint_upload_title()} width="560px" onclose={() => !busy && onclose()}>
	<div class="flex flex-col overflow-y-auto px-5 py-4.5">
		<Field label={m.blueprint_file()}>
			<div class="flex gap-1" role="group">
				{#each SOURCES as [id, label] (id)}
					<button
						class="rounded-full px-3 py-0.75 text-[12px] font-medium transition-colors duration-150
							{source === id ? 'bg-brand-soft text-brand-fg' : 'bg-btn text-muted hover:bg-btn-hi hover:text-ink'}"
						aria-pressed={source === id}
						disabled={!!busy || (id === 'instance' && app.instances.length === 0)}
						onclick={() => setSource(id)}
					>
						{label()}
					</button>
				{/each}
			</div>
		</Field>

		{#if source === 'file'}
			<Field label="" for="bp-file">
				<div class="flex gap-1.75">
					<TextInput id="bp-file" class="min-w-0 flex-1" value={path} readonly placeholder={m.blueprint_pick_file()} />
					<Button onclick={pickFile} disabled={!!busy}>{m.common_browse()}</Button>
				</div>
			</Field>
		{:else}
			<Field label={m.blueprint_instance()} for="bp-instance">
				<Select id="bp-instance" bind:value={instanceId} disabled={!!busy}>
					{#each app.instances as inst (inst.id)}
						<option value={inst.id}>{inst.name} ({inst.versionId})</option>
					{/each}
				</Select>
			</Field>
			<Field label="" for="bp-instance-file" class="items-start!">
				{#if instanceFiles === null}
					<span class="flex items-center gap-2 text-muted"><Spinner />{m.blueprint_loading_files()}</span>
				{:else if instanceFiles.length === 0}
					<span class="text-faint">{m.blueprint_instance_no_files()}</span>
				{:else}
					<div class="flex min-w-0 flex-col gap-1.5">
						<TextInput
							id="bp-instance-file"
							bind:value={fileQuery}
							disabled={!!busy}
							onkeydown={(e) => e.key === 'Escape' && fileQuery && (e.stopPropagation(), (fileQuery = ''))}
							placeholder={m.blueprint_search_files()}
							aria-label={m.blueprint_search_files()}
						/>
						<div class="h-44 overflow-y-auto rounded-md border border-line bg-deep p-1" aria-label={m.blueprint_pick_instance_file()}>
							{#if matchedFiles.length}
								{@render branch(fileTree, 0)}
							{:else}
								<p class="px-2 py-3 text-center text-faint">{m.blueprint_files_no_match()}</p>
							{/if}
						</div>
					</div>
				{/if}
			</Field>
		{/if}

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
