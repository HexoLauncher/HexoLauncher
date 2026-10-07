<script lang="ts">
	import { Button, Field, TextInput } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import InstanceGrid from '$lib/components/InstanceGrid.svelte';
	import InstanceActions from '$lib/components/InstanceActions.svelte';
	import Dialog from '$lib/ui/Dialog.svelte';
	import Switch from '$lib/ui/Switch.svelte';
	import { formatSize } from '$lib/format';
	import * as api from '$lib/api';
	import {
		app,
		findInstance,
		groupsOf,
		knownGroups,
		refreshInstances,
		refreshRunning,
		report,
		reportError,
		runningIds,
		runningOf
	} from '$lib/state.svelte';
	import { openAppWindow, windowSpecs } from '$lib/windows';
	import { m } from '$lib/paraglide/messages.js';


	let confirmingDelete = $state(false);
	let groupDialog = $state<{ id: string; value: string } | null>(null);
	let duplicateDialog = $state<{ id: string; value: string } | null>(null);
	let exportDialog = $state<{
		id: string;
		name: string;
		includeSaves: boolean;
		includeScreenshots: boolean;
	} | null>(null);
	let busy = $state('');

	const selected = $derived(findInstance(app.selectedId));
	const groups = $derived(groupsOf(app.instances));

	function launch(id: string) {
		const target = findInstance(id);
		if (!target) return;
		app.selectedId = id;

		if (app.config.game.showProgressWindow || !target.healthy) {
			openAppWindow(windowSpecs.launch(id, target.name));
			return;
		}
		report(m.status_launching({ name: target.name }));
		api
			.launchInstance({ baseDir: app.baseDir, instanceName: id })
			.then(() => {
				report(m.status_launched({ name: target.name }));
				refreshRunning();
			})
			.catch((e) => reportError(m.error_launch(), e));
	}

	async function kill(id: string) {
		const name = findInstance(id)?.name ?? '';
		try {
			await api.stopInstance(id);
			await refreshRunning();
			report(m.status_kill_requested({ name }));
		} catch (e) {
			reportError(m.error_kill(), e);
		}
	}

	function edit(id: string) {
		const target = findInstance(id);
		if (!target) return;
		app.selectedId = id;
		openAppWindow(windowSpecs.instance(id, target.name));
	}

	async function confirmDelete() {
		const target = selected;
		confirmingDelete = false;
		if (!target) return;
		try {
			await api.deleteInstance(app.baseDir, target.id);
			await refreshInstances();
			report(m.status_instance_deleted({ name: target.name }));
		} catch (e) {
			reportError(m.error_delete(), e);
		}
	}

	async function saveGroup() {
		if (!groupDialog) return;
		const { id, value } = groupDialog;
		groupDialog = null;
		try {
			await api.saveInstanceMeta(app.baseDir, id, { group: value });
			await refreshInstances();
			report(value ? m.status_group_moved({ group: value }) : m.status_group_cleared());
		} catch (e) {
			reportError(m.error_change_group(), e);
		}
	}

	async function confirmDuplicate() {
		if (!duplicateDialog) return;
		const { id, value } = duplicateDialog;
		duplicateDialog = null;
		busy = m.busy_duplicating();
		try {
			const copy = await api.duplicateInstance(app.baseDir, id, value);
			await refreshInstances();
			app.selectedId = copy.id;
			report(m.status_duplicated({ name: copy.name }));
		} catch (e) {
			reportError(m.error_duplicate(), e);
		} finally {
			busy = '';
		}
	}


	async function confirmExport() {
		if (!exportDialog) return;
		const { id, name, includeSaves, includeScreenshots } = exportDialog;
		exportDialog = null;

		let targetPath: string | null = null;
		try {
			const { save } = await import('@tauri-apps/api/dialog');
			targetPath = await save({
				defaultPath: `${name}.zip`,
				filters: [{ name: m.file_filter_zip(), extensions: ['zip'] }]
			});
		} catch {
			reportError(m.error_export(), m.common_no_dialog());
			return;
		}
		if (!targetPath) return;

		busy = m.busy_packing();
		try {
			const result = await api.exportInstance({
				baseDir: app.baseDir,
				instanceName: id,
				targetPath,
				includeSaves,
				includeScreenshots
			});
			report(
				m.status_exported({
					count: result.fileCount,
					size: formatSize(result.bytes),
					path: result.path
				})
			);
		} catch (e) {
			reportError(m.error_export(), e);
		} finally {
			busy = '';
		}
	}

	async function repair(id: string) {
		const target = findInstance(id);
		if (!target) return;
		if (!target.versionId) {
			report(m.status_no_version_info());
			return;
		}
		busy = m.busy_repairing({ name: target.name });
		try {
			await api.repairInstance(app.baseDir, id);
			await refreshInstances();
			report(m.status_repaired({ name: target.name }));
		} catch (e) {
			reportError(m.error_repair(), e);
		} finally {
			busy = '';
		}
	}

	async function openFolder(id: string, target: api.InstanceFolder = 'minecraft') {
		try {
			await api.openInstanceFolder(app.baseDir, id, target);
		} catch (e) {
			reportError(m.error_open_folder(), e);
		}
	}

	function shortcut(event: KeyboardEvent) {
		if (confirmingDelete || groupDialog || duplicateDialog || exportDialog) return;
		if (event.key === 'Delete' && app.selectedId) {
			confirmingDelete = true;
		} else if (event.key === 'F5') {
			event.preventDefault();
			refreshInstances();
			refreshRunning();
			report(m.status_refreshed());
		}
	}
</script>

<svelte:head><title>Nolauncher</title></svelte:head>
<svelte:window onkeydown={shortcut} />

<InstanceGrid
	instances={app.instances}
	groupOrder={groups}
	selectedId={app.selectedId}
	running={runningIds()}
	showBadge={app.config.ui.runningBadge}
	loading={!app.ready}
	bind:search={app.search}
	onSelect={(id) => (app.selectedId = id === '' ? null : id)}
	onLaunch={launch}
	onKill={kill}
	onEdit={edit}
	onRepair={repair}
	onDuplicate={(id) => (duplicateDialog = { id, value: m.copy_suffix({ name: findInstance(id)?.name ?? '' }) })}
	onExport={(id) =>
		(exportDialog = {
			id,
			name: findInstance(id)?.name ?? id,
			includeSaves: false,
			includeScreenshots: false
		})}
	onOpenFolder={(id) => openFolder(id)}
	onDelete={(id) => {
		app.selectedId = id;
		confirmingDelete = true;
	}}
/>

{#if app.config.ui.instanceToolbar}
	<InstanceActions
		instance={selected}
		running={runningOf(app.selectedId)}
		onLaunch={() => app.selectedId && launch(app.selectedId)}
		onKill={() => app.selectedId && kill(app.selectedId)}
		onEdit={() => app.selectedId && edit(app.selectedId)}
		onRepair={() => app.selectedId && repair(app.selectedId)}
		onGroup={() => selected && (groupDialog = { id: selected.id, value: selected.group })}
		onDuplicate={() =>
			selected && (duplicateDialog = { id: selected.id, value: m.copy_suffix({ name: selected.name }) })}
		onExport={() =>
			selected &&
			(exportDialog = {
				id: selected.id,
				name: selected.name,
				includeSaves: false,
				includeScreenshots: false
			})}
		onFolder={(target) => app.selectedId && openFolder(app.selectedId, target)}
		onDelete={() => (confirmingDelete = true)}
	/>
{/if}

{#if confirmingDelete && selected}
	<Dialog title={m.dialog_delete_instance_title()} width="420px" onclose={() => (confirmingDelete = false)}>
		<div class="flex gap-3.75 px-5 py-5.5">
			<span class="grid size-11.5 shrink-0 place-items-center rounded-lg bg-danger-soft text-danger"
				><Icon name="trash" size={30} /></span
			>
			<div>
				<p class="mb-2 text-[15px] font-semibold">{m.dialog_delete_confirm({ name: selected.name })}</p>
				<p class="leading-[1.75] text-muted">
					{m.dialog_delete_desc()}
				</p>
			</div>
		</div>
		{#snippet footer()}
			<Button variant="destructive" onclick={confirmDelete}>{m.common_delete()}</Button>
			<Button onclick={() => (confirmingDelete = false)}>{m.common_cancel()}</Button>
		{/snippet}
	</Dialog>
{/if}

{#if groupDialog}
	<Dialog title={m.dialog_change_group_title()} width="380px" onclose={() => (groupDialog = null)}>
		<div class="px-5 py-5">
			<Field label={m.common_group()} for="group-name">
				<TextInput id="group-name" bind:value={groupDialog.value} list="known-groups" placeholder={m.common_ungrouped()} />
				<datalist id="known-groups">
					{#each knownGroups() as g (g)}
						<option value={g}></option>
					{/each}
				</datalist>
			</Field>
		</div>
		{#snippet footer()}
			<Button variant="primary" onclick={saveGroup}>{m.common_ok()}</Button>
			<Button onclick={() => (groupDialog = null)}>{m.common_cancel()}</Button>
		{/snippet}
	</Dialog>
{/if}

{#if duplicateDialog}
	<Dialog title={m.dialog_duplicate_title()} width="380px" onclose={() => (duplicateDialog = null)}>
		<div class="px-5 py-5">
			<Field label={m.dialog_new_name()} for="copy-name">
				<TextInput id="copy-name" bind:value={duplicateDialog.value} />
			</Field>
		</div>
		{#snippet footer()}
			<Button variant="primary" disabled={!duplicateDialog?.value.trim()} onclick={confirmDuplicate}>{m.editor_copy()}</Button>
			<Button onclick={() => (duplicateDialog = null)}>{m.common_cancel()}</Button>
		{/snippet}
	</Dialog>
{/if}

{#if exportDialog}
	<Dialog title={m.dialog_export_title()} width="440px" onclose={() => (exportDialog = null)}>
		<div class="flex flex-col gap-1 px-5 py-5">
			<p class="mb-1 text-muted">{m.dialog_export_desc({ name: exportDialog.name })}</p>
			<Switch label={m.export_include_saves()} bind:checked={exportDialog.includeSaves} />
			<Switch label={m.export_include_screenshots()} bind:checked={exportDialog.includeScreenshots} />
		</div>
		{#snippet footer()}
			<Button variant="primary" onclick={confirmExport}>{m.export_choose_location()}</Button>
			<Button onclick={() => (exportDialog = null)}>{m.common_cancel()}</Button>
		{/snippet}
	</Dialog>
{/if}

{#if busy}
	<div class="fixed inset-0 z-100 grid place-items-center bg-scrim backdrop-blur-[3px]">
		<div
			class="flex items-center gap-3 rounded-xl border border-line bg-window px-6 py-5 shadow-lg"
			role="status"
		>
			<span class="size-5 animate-spin rounded-full border-2 border-line border-t-brand"></span>
			{busy}
		</div>
	</div>
{/if}
