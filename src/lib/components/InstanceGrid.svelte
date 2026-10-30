<script lang="ts">
	import { Button, ContextMenu, Icon, MenuItem, MenuSeparator, Pill, SearchField, Spinner } from '$lib/ui';
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import type { InstanceView } from '$lib/api';
	import { formatDate, loaderLabel } from '$lib/format';
	import { m } from '$lib/paraglide/messages.js';

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
		return group === '' ? m.group_ungrouped() : group;
	}

	function openContext(event: MouseEvent, id: string) {
		event.preventDefault();
		onSelect(id);
		context = { x: event.clientX, y: event.clientY, id };
	}

	function title(item: InstanceView) {
		const state = !item.installed
			? m.instance_state_not_installed()
			: !item.healthy
				? m.instance_state_needs_repair()
				: '';
		return [
			item.name,
			m.instance_tooltip_version({ version: item.versionId, loader: loaderLabel(item.loader) }),
			m.instance_tooltip_last_played({ date: formatDate(item.lastPlayed) }),
			state
		]
			.filter(Boolean)
			.join('\n');
	}
</script>

<div class="flex min-w-0 flex-1 flex-col bg-deep">
	<div class="flex shrink-0 items-center gap-3 px-3.5 py-2.25">
		<div class="flex max-w-105 flex-1 items-center">
			<SearchField
				class="w-full bg-window"
				placeholder={m.search_instances()}
				aria-label={m.search_instances()}
				bind:value={search}
			/>
			{#if search}
				<button
					class="-ml-8 grid size-4.75 shrink-0 place-items-center rounded-full bg-btn text-muted
						transition-colors duration-150 hover:bg-btn-hi hover:text-ink"
					aria-label={m.search_clear()}
					onclick={() => (search = '')}
				>
					<Icon name="close" size={12} />
				</button>
			{/if}
		</div>
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
					<Pill class="px-2 py-px text-[11px]">{section.items.length}</Pill>
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
									<InstanceIcon name={item.icon} image={item.iconImage} size={52} />
									{#if showBadge && isRunning}
										<span
											class="absolute -bottom-0.75 -right-0.75 size-3.25 rounded-full border-3 border-deep
												bg-brand shadow-[0_0_10px_var(--color-brand-line)]"
											title={m.common_running()}
										></span>
									{:else if !item.healthy}
										<span
											class="absolute -bottom-0.75 -right-0.75 grid size-4.5 place-items-center rounded-full
												border-2 border-deep bg-warn text-[10px] font-bold text-black"
											title={item.installed ? m.badge_incomplete() : m.badge_not_downloaded()}
										>!</span>
									{/if}
								</span>

								<span
									class="line-clamp-2 min-h-[2.7em] w-full wrap-anywhere text-center font-medium leading-[1.35]
										{selectedId === item.id ? 'text-brand-fg' : ''}"
								>
									{item.name}
								</span>
								<span class="w-full truncate text-center text-[11.5px] text-faint">{item.versionId || m.common_dash()}</span>
							</button>

						</div>
					{/each}
				</div>
			{/if}
		{/each}

		{#if grouped.length === 0}
			<div class="flex flex-col items-center gap-3.5 py-17.5 text-muted">
				{#if loading}
					<Spinner size={24} />
					<p>{m.grid_loading()}</p>
				{:else if search}
					<Icon name="search" size={34} />
					<p>{m.grid_no_match({ query: search })}</p>
					<Button onclick={() => (search = '')}>{m.search_clear()}</Button>
				{:else}
					<Icon name="grid" size={34} />
					<p>{m.grid_empty()}</p>
				{/if}
			</div>
		{/if}
	</div>
</div>

{#if context}
	{@const item = instances.find((i) => i.id === context?.id)}
	<ContextMenu x={context.x} y={context.y} width="184px" onclose={() => (context = null)}>
		<MenuItem icon="play" onclick={() => onLaunch(context!.id)}>{m.action_launch()}</MenuItem>
		<MenuItem
			icon="stop"
			disabled={!running.includes(context.id)}
			onclick={() => onKill(context!.id)}
		>
			{m.action_stop()}
		</MenuItem>
		<MenuSeparator />
		<MenuItem icon="edit" onclick={() => onEdit(context!.id)}>{m.action_edit_ellipsis()}</MenuItem>
		<MenuItem icon="refresh" disabled={item?.healthy} onclick={() => onRepair(context!.id)}>
			{m.action_repair_reinstall()}
		</MenuItem>
		<MenuItem icon="folder" onclick={() => onOpenFolder(context!.id)}>{m.folder_instance()}</MenuItem>
		<MenuSeparator />
		<MenuItem icon="copy" onclick={() => onDuplicate(context!.id)}>{m.action_duplicate_ellipsis()}</MenuItem>
		<MenuItem icon="export" onclick={() => onExport(context!.id)}>{m.action_export_zip()}</MenuItem>
		<MenuSeparator />
		<MenuItem icon="trash" danger onclick={() => onDelete(context!.id)}>{m.common_delete()}</MenuItem>
	</ContextMenu>
{/if}
