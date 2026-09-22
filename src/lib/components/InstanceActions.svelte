<script lang="ts">
	import { Icon, Menu, MenuItem, MenuSeparator, Pill, ToolButton } from '$lib/ui';
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

	const off = $derived(instance === null);

	const launchButton =
		'flex items-center justify-center gap-2 min-h-9 rounded-md bg-brand font-semibold text-brand-ink ' +
		'transition-[background-color,box-shadow] duration-150 ' +
		'not-disabled:hover:bg-brand-hi not-disabled:hover:shadow-[0_0_0_4px_var(--color-brand-soft)] ' +
		'not-disabled:active:bg-brand-lo disabled:bg-btn disabled:text-faint disabled:shadow-none';

	const item = 'text-ink disabled:text-faint';
</script>

<div
	class="instance-toolbar flex w-43 shrink-0 flex-col gap-0.5 border-l border-line bg-window px-2.5 pt-2.5 pb-3"
>
	<button class="{launchButton} mb-1.5" disabled={off || !!running} onclick={onLaunch}>
		<Icon name="play" size={15} />
		<span>{running ? '執行中' : '啟動'}</span>
	</button>

	<ToolButton layout="beside" icon="stop" class={item} disabled={!running} onclick={onKill}>
		<span>終止</span>
	</ToolButton>

	<span class="mx-1 my-2 h-px bg-line"></span>

	<ToolButton layout="beside" icon="edit" class={item} disabled={off} onclick={onEdit}>
		<span>編輯</span>
	</ToolButton>

	<ToolButton layout="beside" icon="tag" class={item} disabled={off} onclick={onGroup}>
		<span>變更群組</span>
	</ToolButton>

	<Menu width="180px" placement="bottom-end">
		{#snippet trigger(attrs)}
			<ToolButton {...attrs} layout="beside" icon="folder" class={item} disabled={off}>
				<span>資料夾</span>
				<Icon name="caret" size={9} />
			</ToolButton>
		{/snippet}

		<MenuItem onclick={() => onFolder('minecraft')}>Minecraft 資料夾</MenuItem>
		<MenuItem onclick={() => onFolder('mods')}>模組</MenuItem>
		<MenuItem onclick={() => onFolder('saves')}>世界存檔</MenuItem>
		<MenuItem onclick={() => onFolder('config')}>模組設定</MenuItem>
		<MenuItem onclick={() => onFolder('logs')}>日誌</MenuItem>
		<MenuSeparator />
		<MenuItem onclick={() => onFolder('instance')}>實例資料夾</MenuItem>
	</Menu>

	<ToolButton layout="beside" icon="copy" class={item} disabled={off} onclick={onDuplicate}>
		<span>複製實例</span>
	</ToolButton>

	<ToolButton layout="beside" icon="export" class={item} disabled={off} onclick={onExport}>
		<span>匯出…</span>
	</ToolButton>

	<ToolButton
		layout="beside"
		icon="refresh"
		class={item}
		disabled={off || !!running}
		onclick={onRepair}
	>
		<span>{instance && !instance.installed ? '下載檔案' : '修復'}</span>
	</ToolButton>

	<ToolButton
		layout="beside"
		icon="trash"
		class={item}
		disabled={off || !!running}
		onclick={onDelete}
	>
		<span>刪除</span>
	</ToolButton>

	<div class="min-h-4 flex-1"></div>

	{#if instance}
		<div class="rounded-lg bg-deep px-3 py-2.75 text-muted">
			<div class="mb-2.5 flex flex-wrap gap-1.25">
				<Pill accent>{instance.versionId || '未知版本'}</Pill>
				<Pill>{loaderLabel(instance.loader)}</Pill>
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
