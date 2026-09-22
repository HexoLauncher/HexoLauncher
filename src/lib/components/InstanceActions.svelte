<script lang="ts">
	import Icon from '$lib/Icon.svelte';
	import Menu from './Menu.svelte';
	import type { InstanceFolder, InstanceView, RunningGame } from '$lib/api';
	import { formatDate, formatDuration, formatTime, loaderLabel } from '$lib/format';

	let {
		instance,
		running,
		onLaunch,
		onKill,
		onEdit,
		onGroup,
		onFolder,
		onDuplicate,
		onExport,
		onDelete,
		onRepair
	}: {
		instance: InstanceView | null;

		running: RunningGame | null;
		onLaunch: () => void;
		onKill: () => void;
		onEdit: () => void;
		onGroup: () => void;
		onFolder: (target: InstanceFolder) => void;
		onDuplicate: () => void;
		onExport: () => void;
		onDelete: () => void;
		onRepair: () => void;
	} = $props();

	let folderMenu = $state(false);

	const off = $derived(instance === null);


	const launchButton =
		'flex items-center justify-center gap-2 min-h-9 rounded-md bg-brand font-semibold text-brand-ink ' +
		'transition-[background-color,box-shadow] duration-150 ' +
		'not-disabled:hover:bg-brand-hi not-disabled:hover:shadow-[0_0_0_4px_var(--color-brand-soft)] ' +
		'not-disabled:active:bg-brand-lo disabled:bg-btn disabled:text-faint disabled:shadow-none';
</script>

<div
	class="instance-toolbar flex w-43 shrink-0 flex-col gap-0.5 border-l border-line bg-window px-2.5 pb-3 pt-2.5"
>
	<button class="{launchButton} mb-1.5" disabled={off || !!running} onclick={onLaunch}>
		<Icon name="play" size={15} />
		<span>{running ? '執行中' : '啟動'}</span>
	</button>

	<button
		class="tool-button beside text-ink disabled:text-faint"
		disabled={!running}
		onclick={onKill}
	>
		<Icon name="stop" size={16} />
		<span>終止</span>
	</button>

	<span class="mx-1 my-2 h-px bg-line"></span>

	<button class="tool-button beside text-ink disabled:text-faint" disabled={off} onclick={onEdit}>
		<Icon name="edit" size={16} />
		<span>編輯</span>
	</button>

	<button class="tool-button beside text-ink disabled:text-faint" disabled={off} onclick={onGroup}>
		<Icon name="tag" size={16} />
		<span>變更群組</span>
	</button>

	<div class="relative flex gap-0.5">
		<button
			class="tool-button beside flex-1 text-ink disabled:text-faint"
			disabled={off}
			onclick={() => (folderMenu = !folderMenu)}
		>
			<Icon name="folder" size={16} />
			<span>資料夾</span>
			<Icon name="caret" size={9} />
		</button>
		<Menu open={folderMenu} onclose={() => (folderMenu = false)} align="right" width="180px">
			<button class="qt-menu-item" onclick={() => onFolder('minecraft')}>Minecraft 資料夾</button>
			<button class="qt-menu-item" onclick={() => onFolder('mods')}>模組</button>
			<button class="qt-menu-item" onclick={() => onFolder('saves')}>世界存檔</button>
			<button class="qt-menu-item" onclick={() => onFolder('config')}>模組設定</button>
			<button class="qt-menu-item" onclick={() => onFolder('logs')}>日誌</button>
			<hr />
			<button class="qt-menu-item" onclick={() => onFolder('instance')}>實例資料夾</button>
		</Menu>
	</div>

	<button
		class="tool-button beside text-ink disabled:text-faint"
		disabled={off}
		onclick={onDuplicate}
	>
		<Icon name="copy" size={16} />
		<span>複製實例</span>
	</button>

	<button class="tool-button beside text-ink disabled:text-faint" disabled={off} onclick={onExport}>
		<Icon name="export" size={16} />
		<span>匯出…</span>
	</button>

	<button
		class="tool-button beside text-ink disabled:text-faint"
		disabled={off || !!running}
		onclick={onRepair}
	>
		<Icon name="refresh" size={16} />
		<span>{instance && !instance.installed ? '下載檔案' : '修復'}</span>
	</button>

	<button
		class="tool-button beside text-ink disabled:text-faint"
		disabled={off || !!running}
		onclick={onDelete}
	>
		<Icon name="trash" size={16} />
		<span>刪除</span>
	</button>

	<div class="min-h-4 flex-1"></div>

	{#if instance}
		<div class="rounded-lg bg-deep px-3 py-2.75 text-muted">
			<div class="mb-2.5 flex flex-wrap gap-1.25">
				<span class="pill accent">{instance.versionId || '未知版本'}</span>
				<span class="pill">{loaderLabel(instance.loader)}</span>
			</div>

			{#if running}
				<p class="mb-2 rounded-md bg-brand-soft px-2 py-1.5 leading-[1.7] text-brand-fg">
					執行中 · PID {running.pid}<br />
					<span class="text-[11px] text-muted">
						{formatTime(running.startedAt)} 起 · {running.playerName}
					</span>
				</p>
			{/if}

			{#if !instance.healthy}
				<p class="mb-2 rounded-md bg-danger-soft px-2 py-1.5 text-warn">
					{instance.installed ? '檔案不完整，需要修復' : '尚未下載遊戲檔案'}
				</p>
			{/if}


			<p class="py-1">
				<span class="block text-[11.5px] text-faint">上次遊玩</span>
				<strong class="mt-px block font-medium text-ink">{formatDate(instance.lastPlayed)}</strong>
			</p>
			<p class="py-1">
				<span class="block text-[11.5px] text-faint">總時數</span>
				<strong class="mt-px block font-medium text-ink"
					>{formatDuration(instance.playTimeSeconds)}</strong
				>
			</p>
			<p class="py-1">
				<span class="block text-[11.5px] text-faint">模組</span>
				<strong class="mt-px block font-medium text-ink">{instance.modCount} 個</strong>
			</p>
		</div>
	{/if}
</div>
