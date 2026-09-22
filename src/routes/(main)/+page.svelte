<script lang="ts">
	import Icon from '$lib/Icon.svelte';
	import InstanceGrid from '$lib/components/InstanceGrid.svelte';
	import InstanceActions from '$lib/components/InstanceActions.svelte';
	import Window from '$lib/components/Window.svelte';
	import Toggle from '$lib/components/Toggle.svelte';
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

	/** 確認與輸入都是訊息方塊，留在主視窗裡當強制回應對話框 */
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

		// 還沒下載完的實例一定要開視窗看進度；其餘照設定。
		if (app.config.game.showProgressWindow || !target.healthy) {
			openAppWindow(windowSpecs.launch(id, target.name));
			return;
		}
		report(`正在啟動 ${target.name}…`);
		api
			.launchInstance({ baseDir: app.baseDir, instanceName: id })
			.then(() => {
				report(`${target.name} 已啟動`);
				refreshRunning();
			})
			.catch((e) => reportError('啟動失敗', e));
	}

	async function kill(id: string) {
		const name = findInstance(id)?.name ?? '';
		try {
			await api.stopInstance(id);
			await refreshRunning();
			report(`已要求 ${name} 結束`);
		} catch (e) {
			reportError('終止失敗', e);
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
			report(`已刪除實例「${target.name}」`);
		} catch (e) {
			reportError('刪除失敗', e);
		}
	}

	async function saveGroup() {
		if (!groupDialog) return;
		const { id, value } = groupDialog;
		groupDialog = null;
		try {
			await api.saveInstanceMeta(app.baseDir, id, { group: value });
			await refreshInstances();
			report(value ? `已移到群組「${value}」` : '已移出群組');
		} catch (e) {
			reportError('變更群組失敗', e);
		}
	}

	async function confirmDuplicate() {
		if (!duplicateDialog) return;
		const { id, value } = duplicateDialog;
		duplicateDialog = null;
		busy = '正在複製實例…';
		try {
			const copy = await api.duplicateInstance(app.baseDir, id, value);
			await refreshInstances();
			app.selectedId = copy.id;
			report(`已複製為「${copy.name}」`);
		} catch (e) {
			reportError('複製失敗', e);
		} finally {
			busy = '';
		}
	}

	/** 匯出成 .zip；路徑用系統的儲存對話框選 */
	async function confirmExport() {
		if (!exportDialog) return;
		const { id, name, includeSaves, includeScreenshots } = exportDialog;
		exportDialog = null;

		let targetPath: string | null = null;
		try {
			const { save } = await import('@tauri-apps/api/dialog');
			targetPath = await save({
				defaultPath: `${name}.zip`,
				filters: [{ name: '壓縮檔', extensions: ['zip'] }]
			});
		} catch {
			reportError('匯出失敗', '這個環境沒有系統對話框');
			return;
		}
		if (!targetPath) return;

		busy = '正在打包實例…';
		try {
			const result = await api.exportInstance({
				baseDir: app.baseDir,
				instanceName: id,
				targetPath,
				includeSaves,
				includeScreenshots
			});
			report(`已匯出 ${result.fileCount} 個檔案（${formatSize(result.bytes)}）到 ${result.path}`);
		} catch (e) {
			reportError('匯出失敗', e);
		} finally {
			busy = '';
		}
	}

	async function repair(id: string) {
		const target = findInstance(id);
		if (!target) return;
		if (!target.versionId) {
			report('這個實例沒有版本資訊，請用「編輯 → 版本」重新指定');
			return;
		}
		busy = `正在修復 ${target.name}…`;
		try {
			await api.repairInstance(app.baseDir, id);
			await refreshInstances();
			report(`${target.name} 已修復完成`);
		} catch (e) {
			reportError('修復失敗', e);
		} finally {
			busy = '';
		}
	}

	async function openFolder(id: string, target: api.InstanceFolder = 'minecraft') {
		try {
			await api.openInstanceFolder(app.baseDir, id, target);
		} catch (e) {
			reportError('開啟資料夾失敗', e);
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
			report('已重新整理實例清單');
		}
	}
</script>

<svelte:head><title>Hexo Launcher</title></svelte:head>
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
	onDuplicate={(id) => (duplicateDialog = { id, value: `${findInstance(id)?.name ?? ''} 的副本` })}
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
			selected && (duplicateDialog = { id: selected.id, value: `${selected.name} 的副本` })}
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
	<Window title="刪除實例" width="420px" onclose={() => (confirmingDelete = false)}>
		<div class="flex gap-3.75 px-5 py-5.5">
			<span class="grid size-11.5 shrink-0 place-items-center rounded-lg bg-danger-soft text-danger"
				><Icon name="trash" size={30} /></span
			>
			<div>
				<p class="mb-2 text-[15px] font-semibold">確定要刪除「{selected.name}」嗎？</p>
				<p class="leading-[1.75] text-muted">
					這個實例的所有世界、模組與設定都會一併移除，而且無法復原。
				</p>
			</div>
		</div>
		{#snippet footer()}
			<button class="qt-button destructive" onclick={confirmDelete}>刪除</button>
			<button class="qt-button" onclick={() => (confirmingDelete = false)}>取消</button>
		{/snippet}
	</Window>
{/if}

{#if groupDialog}
	<Window title="變更群組" width="380px" onclose={() => (groupDialog = null)}>
		<div class="px-5 py-5">
			<div class="qt-field">
				<label for="group-name">群組</label>
				<input
					id="group-name"
					class="qt-input"
					bind:value={groupDialog.value}
					list="known-groups"
					placeholder="（未分組）"
				/>
				<datalist id="known-groups">
					{#each knownGroups() as g (g)}
						<option value={g}></option>
					{/each}
				</datalist>
			</div>
		</div>
		{#snippet footer()}
			<button class="qt-button primary" onclick={saveGroup}>確定</button>
			<button class="qt-button" onclick={() => (groupDialog = null)}>取消</button>
		{/snippet}
	</Window>
{/if}

{#if duplicateDialog}
	<Window title="複製實例" width="380px" onclose={() => (duplicateDialog = null)}>
		<div class="px-5 py-5">
			<div class="qt-field">
				<label for="copy-name">新名稱</label>
				<input id="copy-name" class="qt-input" bind:value={duplicateDialog.value} />
			</div>
		</div>
		{#snippet footer()}
			<button
				class="qt-button primary"
				disabled={!duplicateDialog?.value.trim()}
				onclick={confirmDuplicate}>複製</button
			>
			<button class="qt-button" onclick={() => (duplicateDialog = null)}>取消</button>
		{/snippet}
	</Window>
{/if}

{#if exportDialog}
	<Window title="匯出實例" width="440px" onclose={() => (exportDialog = null)}>
		<div class="flex flex-col gap-1 px-5 py-5">
			<p class="mb-1 text-muted">打包「{exportDialog.name}」成 .zip</p>
			<Toggle label="包含世界存檔" bind:checked={exportDialog.includeSaves} />
			<Toggle label="包含螢幕截圖" bind:checked={exportDialog.includeScreenshots} />
		</div>
		{#snippet footer()}
			<button class="qt-button primary" onclick={confirmExport}>選擇位置並匯出</button>
			<button class="qt-button" onclick={() => (exportDialog = null)}>取消</button>
		{/snippet}
	</Window>
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
