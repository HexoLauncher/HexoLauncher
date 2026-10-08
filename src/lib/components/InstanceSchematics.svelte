<script lang="ts">
	import { Button, Dialog, Select, Spinner, Table, TableBox } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import * as api from '$lib/api';
	import type { SchematicFile } from '$lib/api';
	import { formatSize, formatTime } from '$lib/format';
	import { hub } from '$lib/blueprints.svelte';
	import { app, report, reportError } from '$lib/state.svelte';
	import { openAppWindow, windowSpecs } from '$lib/windows';
	import { m } from '$lib/paraglide/messages.js';

	let { onupload }: { onupload: (file: SchematicFile, mcVersion: string) => void } = $props();

	let instanceId = $state(app.selectedId ?? app.instances[0]?.id ?? '');
	let files = $state<SchematicFile[]>([]);
	let selected = $state<string | null>(null);
	let loading = $state(false);
	let message = $state('');
	let confirmingDelete = $state(false);

	const instance = $derived(app.instances.find((i) => i.id === instanceId) ?? null);
	const file = $derived(files.find((f) => f.relPath === selected) ?? null);
	const canUpload = $derived(!!hub.session?.user);


	$effect(() => {
		if (!instanceId && app.instances.length) instanceId = app.selectedId ?? app.instances[0].id;
	});

	$effect(() => {
		if (instanceId) load(instanceId);
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

	<div class="flex min-h-0 flex-1 gap-2.5">
		<TableBox>
			<Table>
				<thead>
					<tr>
						<th style="width:42%">{m.common_name()}</th>
						<th>{m.blueprint_folder()}</th>
						<th style="width:84px">{m.blueprint_size()}</th>
						<th style="width:132px">{m.blueprint_modified()}</th>
					</tr>
				</thead>
				<tbody>
					{#each files as f (f.relPath)}
						<tr
							class:selected={selected === f.relPath}
							onclick={() => (selected = f.relPath)}
							ondblclick={() => preview(f)}
						>
							<td title={f.name}>
								<span class="flex items-center gap-2">
									<Icon name="blocks" size={14} />
									<span class="truncate">{f.name}</span>
								</span>
							</td>
							<td class="dim">{folderOf(f)}</td>
							<td class="dim">{formatSize(f.size)}</td>
							<td class="dim">{formatTime(f.modified)}</td>
						</tr>
					{/each}
				</tbody>
			</Table>
			<div class="flex flex-col items-center gap-2 px-3 py-4 text-muted">
				{#if loading}
					<span class="flex items-center gap-2"><Spinner size={16} />{m.blueprint_loading_files()}</span>
				{:else if message}
					<span class="text-danger">{message}</span>
				{:else if files.length === 0 && instance}
					<span class="text-center leading-[1.7]">{m.blueprint_instance_empty()}</span>
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
			<p class="mt-2 leading-[1.7] text-faint">{m.blueprint_instance_hint()}</p>
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
