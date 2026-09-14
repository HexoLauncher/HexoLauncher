<script lang="ts">
	import Icon from '$lib/Icon.svelte';
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import type { InstanceView } from '$lib/api';
	import { formatDate, loaderLabel } from '$lib/format';

	let {
		instances,
		groupOrder,
		selectedId,
		running,
		showBadge = true,
		loading = false,
		search = $bindable(''),
		onSelect,
		onLaunch,
		onKill,
		onEdit,
		onDelete,
		onDuplicate,
		onExport,
		onRepair,
		onOpenFolder
	}: {
		instances: InstanceView[];
		groupOrder: string[];
		selectedId: string | null;
		/** 執行中的實例 id */
		running: string[];
		showBadge?: boolean;
		loading?: boolean;
		search?: string;
		onSelect: (id: string) => void;
		onLaunch: (id: string) => void;
		onKill: (id: string) => void;
		onEdit: (id: string) => void;
		onDelete: (id: string) => void;
		onDuplicate: (id: string) => void;
		onExport: (id: string) => void;
		onRepair: (id: string) => void;
		onOpenFolder: (id: string) => void;
	} = $props();

	let collapsed = $state<Record<string, boolean>>({});
	let context = $state<{ x: number; y: number; id: string } | null>(null);

	const matches = $derived(
		instances.filter((i) => {
			const q = search.trim().toLowerCase();
			if (!q) return true;
			return (
				i.name.toLowerCase().includes(q) ||
				i.versionId.includes(q) ||
				i.loader.includes(q) ||
				i.group.toLowerCase().includes(q)
			);
		})
	);

	const grouped = $derived(
		groupOrder
			.map((group) => ({ group, items: matches.filter((i) => i.group === group) }))
			.filter((g) => g.items.length > 0)
	);

	function label(group: string) {
		return group === '' ? '未分組' : group;
	}

	function openContext(event: MouseEvent, id: string) {
		event.preventDefault();
		onSelect(id);
		context = { x: event.clientX, y: event.clientY, id };
	}

	function title(item: InstanceView) {
		const state = !item.installed ? '尚未安裝' : !item.healthy ? '需要修復' : '';
		return [
			item.name,
			`Minecraft ${item.versionId} · ${loaderLabel(item.loader)}`,
			`上次遊玩：${formatDate(item.lastPlayed)}`,
			state
		]
			.filter(Boolean)
			.join('\n');
	}
</script>

<div class="flex min-w-0 flex-1 flex-col bg-deep">
	<div class="flex shrink-0 items-center gap-3 px-3.5 py-2.25">
		<label class="qt-search max-w-105 flex-1 bg-window">
			<Icon name="search" size={15} />
			<input placeholder="搜尋實例…" bind:value={search} aria-label="搜尋實例" />
			{#if search}
				<button
					class="grid size-4.75 place-items-center rounded-full bg-btn text-muted
						transition-colors duration-150 hover:bg-btn-hi hover:text-ink"
					aria-label="清除搜尋"
					onclick={() => (search = '')}
				>
					<Icon name="close" size={12} />
				</button>
			{/if}
		</label>
		<span class="ml-auto text-faint">{matches.length} / {instances.length}</span>
	</div>

	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div
		class="min-h-0 flex-1 overflow-y-auto px-3.5 pb-6 pt-0.5"
		onclick={(e) => e.target === e.currentTarget && onSelect('')}
	>
		{#each grouped as section (section.group)}
			<div class="mb-2 mt-3.5 flex items-center gap-2.5">
				<button
					class="flex items-center gap-2 rounded-full py-1 pl-1.5 pr-2.5 font-semibold text-muted
						transition-colors duration-150 hover:bg-window hover:text-ink"
					onclick={() => (collapsed[section.group] = !collapsed[section.group])}
					aria-expanded={!collapsed[section.group]}
				>
					<span
						class="flex transition-transform duration-150"
						class:-rotate-90={collapsed[section.group]}
					>
						<Icon name="caret" size={11} />
					</span>
					{label(section.group)}
					<span class="pill px-2 py-px text-[11px]">{section.items.length}</span>
				</button>
				<span class="h-px flex-1 bg-linear-to-r from-line to-transparent"></span>
			</div>

			{#if !collapsed[section.group]}
				<div class="flex flex-wrap gap-1.5">
					{#each section.items as item (item.id)}
						{@const isRunning = running.includes(item.id)}
						<div
							class="group relative w-30.5 rounded-lg transition-[background-color,box-shadow,transform] duration-150
								{selectedId === item.id
								? 'bg-brand-soft shadow-[inset_0_0_0_1px_var(--color-brand-line)]'
								: 'hover:-translate-y-0.5 hover:bg-window'}"
						>
							<button
								class="flex w-full flex-col items-center gap-2.25 rounded-[inherit] px-2 pb-2.75 pt-3 text-inherit"
								onclick={() => onSelect(item.id)}
								ondblclick={() => onLaunch(item.id)}
								oncontextmenu={(e) => openContext(e, item.id)}
								title={title(item)}
							>
								<span
									class="relative grid place-items-center rounded-lg bg-card p-2 shadow-sm
										transition-shadow duration-150 group-hover:shadow-md"
								>
									<InstanceIcon name={item.icon} size={52} />
									{#if showBadge && isRunning}
										<span
											class="absolute -bottom-0.75 -right-0.75 size-3.25 rounded-full border-3 border-deep
												bg-brand shadow-[0_0_10px_var(--color-brand-line)]"
											title="執行中"
										></span>
									{:else if !item.healthy}
										<span
											class="absolute -bottom-0.75 -right-0.75 grid size-4.5 place-items-center rounded-full
												border-2 border-deep bg-warn text-[10px] font-bold text-black"
											title={item.installed ? '檔案不完整，需要修復' : '還沒下載遊戲檔案，啟動時會自動下載'}
										>!</span>
									{/if}
								</span>

								<span
									class="line-clamp-2 min-h-[2.7em] wrap-break-word text-center font-medium leading-[1.35]
										{selectedId === item.id ? 'text-brand' : ''}"
								>
									{item.name}
								</span>
								<span class="text-[11.5px] text-faint">{item.versionId || '—'}</span>
							</button>


							<button
								class="pointer-events-none absolute right-3 top-11.5 grid size-7.5 scale-70 place-items-center
									rounded-full bg-brand pl-0.5 text-brand-ink opacity-0 shadow-md
									transition-[opacity,transform,background-color] duration-150 hover:bg-brand-hi
									group-hover:not-disabled:pointer-events-auto group-hover:not-disabled:scale-100
									group-hover:not-disabled:opacity-100"
								aria-label={isRunning ? `終止 ${item.name}` : `啟動 ${item.name}`}
								onclick={() => (isRunning ? onKill(item.id) : onLaunch(item.id))}
							>
								<Icon name={isRunning ? 'stop' : 'play'} size={15} />
							</button>
						</div>
					{/each}
				</div>
			{/if}
		{/each}

		{#if grouped.length === 0}
			<div class="flex flex-col items-center gap-3.5 py-17.5 text-muted">
				{#if loading}
					<span class="size-6 animate-spin rounded-full border-2 border-line border-t-brand"></span>
					<p>正在讀取實例…</p>
				{:else if search}
					<Icon name="search" size={34} />
					<p>沒有符合「{search}」的實例。</p>
					<button class="qt-button" onclick={() => (search = '')}>清除搜尋</button>
				{:else}
					<Icon name="grid" size={34} />
					<p>還沒有任何實例。</p>
				{/if}
			</div>
		{/if}
	</div>
</div>

{#if context}
	{@const item = instances.find((i) => i.id === context?.id)}
	<button type="button" class="qt-menu-scrim" aria-label="關閉選單" onclick={() => (context = null)}
	></button>
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div
		class="qt-menu fixed left-0 top-0 min-w-46"
		style:left={`${context.x}px`}
		style:top={`${context.y}px`}
		onclick={() => (context = null)}
	>
		<button class="qt-menu-item" onclick={() => onLaunch(context!.id)}>
			<Icon name="play" size={14} />啟動
		</button>
		<button
			class="qt-menu-item"
			disabled={!running.includes(context.id)}
			onclick={() => onKill(context!.id)}
		>
			<Icon name="stop" size={14} />終止
		</button>
		<hr />
		<button class="qt-menu-item" onclick={() => onEdit(context!.id)}>
			<Icon name="edit" size={14} />編輯…
		</button>
		<button class="qt-menu-item" disabled={item?.healthy} onclick={() => onRepair(context!.id)}>
			<Icon name="refresh" size={14} />修復／重新安裝
		</button>
		<button class="qt-menu-item" onclick={() => onOpenFolder(context!.id)}>
			<Icon name="folder" size={14} />實例資料夾
		</button>
		<hr />
		<button class="qt-menu-item" onclick={() => onDuplicate(context!.id)}>
			<Icon name="copy" size={14} />複製…
		</button>
		<button class="qt-menu-item" onclick={() => onExport(context!.id)}>
			<Icon name="export" size={14} />匯出為 .zip…
		</button>
		<hr />
		<button class="qt-menu-item danger" onclick={() => onDelete(context!.id)}>
			<Icon name="trash" size={14} />刪除
		</button>
	</div>
{/if}
