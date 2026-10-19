<script lang="ts">
	import { SvelteSet } from 'svelte/reactivity';
	import { Button, Dialog, Select, Spinner, Table, TableBox, TextInput, ToolButton } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import * as api from '$lib/api';
	import type { SchematicFile } from '$lib/api';
	import { formatSize, formatTime } from '$lib/format';
	import { hub } from '$lib/blueprints.svelte';
	import { app, report, reportError } from '$lib/state.svelte';
	import { openAppWindow, windowSpecs } from '$lib/windows';
	import { m } from '$lib/paraglide/messages.js';

	let { onupload }: { onupload: (file: SchematicFile, mcVersion: string) => void } = $props();

	type View = 'list' | 'folder';
	type TreeDir = { name: string; path: string; dirs: TreeDir[]; files: SchematicFile[]; count: number };
	const VIEW_KEY = 'schematics.view';

	let instanceId = $state(app.selectedId ?? app.instances[0]?.id ?? '');
	let files = $state<SchematicFile[]>([]);
	let selected = $state<string | null>(null);
	let loading = $state(false);
	let message = $state('');
	let confirmingDelete = $state(false);
	let view = $state<View>(readView());
	let query = $state('');
	const expanded = new SvelteSet<string>();

	const instance = $derived(app.instances.find((i) => i.id === instanceId) ?? null);
	const file = $derived(files.find((f) => f.relPath === selected) ?? null);
	const canUpload = $derived(!!hub.session?.user);

	const needle = $derived(query.trim().toLowerCase());
	const shown = $derived(needle ? files.filter((f) => f.relPath.toLowerCase().includes(needle)) : files);
	const tree = $derived(view === 'folder' ? buildTree(shown) : null);


	$effect(() => {
		if (!instanceId && app.instances.length) instanceId = app.selectedId ?? app.instances[0].id;
	});

	$effect(() => {
		if (instanceId) {
			expanded.clear();
			load(instanceId);
		}
	});

	let seq = 0;
	async function load(id: string) {
		const current = ++seq;
		loading = true;
		message = '';
		try {
			const found = await api.listSchematics(app.baseDir, id);
			if (current !== seq) return;
			files = found;
			if (selected && !found.some((f) => f.relPath === selected)) selected = null;
		} catch (e) {
			if (current !== seq) return;
			files = [];
			message = api.errorMessage(e);
		} finally {
			if (current === seq) loading = false;
		}
	}

	function folderOf(f: SchematicFile) {
		const parts = f.relPath.split('/');
		return parts.length > 1 ? parts.slice(0, -1).join('/') : '';
	}

	function buildTree(list: SchematicFile[]) {
		const root: TreeDir = { name: '', path: '', dirs: [], files: [], count: 0 };
		for (const f of list) {
			let node = root;
			for (const name of f.relPath.split('/').slice(0, -1)) {
				const path = node.path ? `${node.path}/${name}` : name;
				let child = node.dirs.find((d) => d.name === name);
				if (!child) {
					child = { name, path, dirs: [], files: [], count: 0 };
					node.dirs.push(child);
				}
				child.count++;
				node = child;
			}
			node.files.push(f);
		}
		const sort = (node: TreeDir) => {
			node.dirs.sort((a, b) => a.name.localeCompare(b.name));
			node.files.sort((a, b) => a.name.localeCompare(b.name));
			node.dirs.forEach(sort);
		};
		sort(root);
		return root;
	}

	function isOpen(path: string) {
		return !!needle || expanded.has(path);
	}

	function toggle(path: string) {
		if (needle) return;
		if (expanded.has(path)) expanded.delete(path);
		else expanded.add(path);
	}

	function reveal(f: SchematicFile) {
		const parts = folderOf(f).split('/').filter(Boolean);
		parts.forEach((_, i) => expanded.add(parts.slice(0, i + 1).join('/')));
	}

	function readView(): View {
		try {
			return localStorage.getItem(VIEW_KEY) === 'folder' ? 'folder' : 'list';
		} catch {
			return 'list';
		}
	}

	function setView(next: View) {
		view = next;
		try {
			localStorage.setItem(VIEW_KEY, next);
		} catch {}
	}

	function preview(f: SchematicFile) {
		if (f.format !== 'litematic') return;
		openAppWindow(
			windowSpecs.viewer({ kind: 'local', path: f.path, instance: instanceId }, f.name)
		);
	}

	async function importFiles() {
		let picked: string[] = [];
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const chosen = await open({
				multiple: true,
				filters: [{ name: m.blueprint_file_filter(), extensions: ['litematic', 'schem'] }]
			});
			picked = Array.isArray(chosen) ? chosen : typeof chosen === 'string' ? [chosen] : [];
		} catch {
			reportError(m.blueprint_error_import(), m.common_no_dialog());
			return;
		}
		if (!picked.length || !instance) return;

		let added: SchematicFile | null = null;
		for (const source of picked) {
			try {
				added = await api.importSchematic(app.baseDir, instance.id, source);
			} catch (e) {
				reportError(m.blueprint_error_import(), e);
			}
		}
		await load(instance.id);
		if (added) {
			selected = added.relPath;
			reveal(added);
			report(m.blueprint_status_imported({ count: picked.length, instance: instance.name }));
		}
	}

	async function remove() {
		confirmingDelete = false;
		const target = file;
		if (!target || !instance) return;
		try {
			await api.deleteSchematic(app.baseDir, instance.id, target.relPath);
			report(m.blueprint_status_file_deleted({ file: target.name }));
			await load(instance.id);
		} catch (e) {
			reportError(m.blueprint_error_delete_file(), e);
		}
	}

	async function openFolder() {
		if (!instance) return;
		try {
			await api.openInstanceFolder(app.baseDir, instance.id, 'schematics');
		} catch (e) {
			reportError(m.error_open_folder(), e);
		}
	}
</script>

{#snippet branch(node: TreeDir, depth: number)}
	{#each node.dirs as dir (dir.path)}
		{@const open = isOpen(dir.path)}
		<button
			class="flex w-full items-center gap-1.5 rounded-md py-1.5 pr-2.5 text-left transition-colors duration-150 hover:bg-hover"
			style="padding-left:{depth * 16 + 6}px"
			aria-expanded={open}
			title={dir.path}
			onclick={() => toggle(dir.path)}
		>
			<Icon name="chevron" size={12} class="text-faint transition-transform duration-150 {open ? 'rotate-90' : ''}" />
			<Icon name="folder" size={14} class="text-muted" />
			<span class="min-w-0 flex-1 truncate">{dir.name}</span>
			<span class="shrink-0 text-[11.5px] text-faint">{m.blueprint_folder_count({ count: dir.count })}</span>
		</button>
		{#if open}
			{@render branch(dir, depth + 1)}
		{/if}
	{/each}
	{#each node.files as f (f.relPath)}
		{@const active = selected === f.relPath}
		<button
			class="flex w-full items-center gap-1.5 rounded-md py-1.5 pr-2.5 text-left transition-colors duration-150
				{active ? 'bg-brand-soft text-ink shadow-[inset_2px_0_0_var(--color-brand)]' : 'hover:bg-hover'}"
			style="padding-left:{depth * 16 + 6}px"
			title={f.relPath}
			onclick={() => (selected = f.relPath)}
			ondblclick={() => preview(f)}
		>
			<span class="w-3 shrink-0"></span>
			<Icon name="blocks" size={14} class="text-muted" />
			<span class="min-w-0 flex-1 truncate">{f.name}</span>
			<span class="shrink-0 text-[11.5px] text-faint">{formatSize(f.size)}</span>
		</button>
	{/each}
{/snippet}

<div class="flex min-h-0 flex-1 flex-col gap-2.5">
	<div class="flex shrink-0 items-center gap-1.75">
		<label class="flex min-w-0 flex-1 items-center gap-2 text-muted">
			{m.blueprint_instance()}
			<Select class="max-w-80 min-w-0 flex-1" bind:value={instanceId} disabled={app.instances.length === 0}>
				{#if app.instances.length === 0}
					<option value="">{m.blueprint_no_instances()}</option>
				{/if}
				{#each app.instances as inst (inst.id)}
					<option value={inst.id}>{inst.name} ({inst.versionId})</option>
				{/each}
			</Select>
		</label>
		<Button icon="plus" disabled={!instance} onclick={importFiles}>{m.blueprint_import_files()}</Button>
		<Button icon="folder" disabled={!instance} onclick={openFolder}>{m.common_open_folder()}</Button>
		<Button
			icon="refresh"
			disabled={!instance}
			onclick={() => instance && load(instance.id)}
			aria-label={m.common_refresh()}
			title={m.common_refresh()}
		></Button>
	</div>

	<div class="flex shrink-0 items-center gap-1.75">
		<TextInput
			class="min-w-0 flex-1"
			bind:value={query}
			disabled={!instance}
			onkeydown={(e) => e.key === 'Escape' && (query = '')}
			placeholder={m.blueprint_search_files()}
			aria-label={m.blueprint_search_files()}
		/>
		<div class="flex shrink-0 gap-0.5 rounded-md bg-btn p-0.5" role="group" aria-label={m.blueprint_view()}>
			<ToolButton
				layout="bare"
				class="p-1.25"
				icon="list"
				iconSize={15}
				active={view === 'list'}
				aria-pressed={view === 'list'}
				title={m.blueprint_view_list()}
				aria-label={m.blueprint_view_list()}
				onclick={() => setView('list')}
			/>
			<ToolButton
				layout="bare"
				class="p-1.25"
				icon="folder"
				iconSize={15}
				active={view === 'folder'}
				aria-pressed={view === 'folder'}
				title={m.blueprint_view_folder()}
				aria-label={m.blueprint_view_folder()}
				onclick={() => setView('folder')}
			/>
		</div>
	</div>

	<div class="flex min-h-0 flex-1 gap-2.5">
		<TableBox>
			{#if tree}
				<div class="pt-1">
					{@render branch(tree, 0)}
				</div>
			{:else}
				<Table class="table-fixed">
					<thead>
						<tr>
							<th>{m.common_name()}</th>
							<th style="width:84px">{m.blueprint_size()}</th>
							<th style="width:132px">{m.blueprint_modified()}</th>
						</tr>
					</thead>
					<tbody>
						{#each shown as f (f.relPath)}
							<tr
								class:selected={selected === f.relPath}
								onclick={() => (selected = f.relPath)}
								ondblclick={() => preview(f)}
							>
								<td title={f.relPath}>
									<span class="flex min-w-0 items-center gap-2">
										<Icon name="blocks" size={14} />
										<span class="min-w-0 truncate">{f.name}</span>
									</span>
								</td>
								<td class="dim">{formatSize(f.size)}</td>
								<td class="dim">{formatTime(f.modified)}</td>
							</tr>
						{/each}
					</tbody>
				</Table>
			{/if}
			<div class="flex flex-col items-center gap-2 px-3 py-4 text-muted">
				{#if loading}
					<span class="flex items-center gap-2"><Spinner size={16} />{m.blueprint_loading_files()}</span>
				{:else if message}
					<span class="text-danger">{message}</span>
				{:else if files.length === 0 && instance}
					<span class="text-center leading-[1.7]">{m.blueprint_instance_empty()}</span>
				{:else if needle && shown.length === 0}
					<span>{m.blueprint_files_no_match()}</span>
				{/if}
			</div>
		</TableBox>

		<div class="flex w-40 shrink-0 flex-col gap-1.75">
			<Button
				variant="primary"
				icon="eye"
				disabled={!file || file.format !== 'litematic'}
				title={file && file.format !== 'litematic' ? m.blueprint_preview_litematic_only() : undefined}
				onclick={() => file && preview(file)}
			>
				{m.blueprint_preview()}
			</Button>
			<Button
				icon="upload"
				disabled={!file || !canUpload}
				title={canUpload ? undefined : m.blueprint_sign_in_to_upload()}
				onclick={() => file && onupload(file, instance?.versionId ?? '')}
			>
				{m.blueprint_upload_this()}
			</Button>
			<Button variant="danger" icon="trash" disabled={!file} onclick={() => (confirmingDelete = true)}>
				{m.common_delete()}
			</Button>
		</div>
	</div>
</div>

{#if confirmingDelete && file}
	<Dialog title={m.blueprint_delete_file_title()} width="400px" onclose={() => (confirmingDelete = false)}>
		<p class="px-5 py-5 leading-[1.75]">{m.blueprint_delete_file_confirm({ file: file.relPath })}</p>
		{#snippet footer()}
			<Button variant="destructive" onclick={remove}>{m.common_delete()}</Button>
			<Button onclick={() => (confirmingDelete = false)}>{m.common_cancel()}</Button>
		{/snippet}
	</Dialog>
{/if}
