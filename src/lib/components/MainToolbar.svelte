<script lang="ts">
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import { Icon, Menu, MenuItem, MenuLabel, MenuSeparator, ToolButton } from '$lib/ui';
	import type { AccountView, LauncherFolder, UiConfig } from '$lib/api';
	import { m } from '$lib/paraglide/messages.js';

	let {
		accounts,
		view,
		currentPath,
		onNavigate,
		onAddInstance,
		onAbout,
		onOpenFolder,
		onSelectAccount,
		onToggleView,
		onManageAccounts
	}: {
		accounts: AccountView[];
		view: UiConfig;

		currentPath: string;
		onNavigate: (path: string) => void;
		onAddInstance: () => void;
		onAbout: () => void;
		onOpenFolder: (target: LauncherFolder) => void;
		onSelectAccount: (id: string) => void;
		onToggleView: (key: 'instanceToolbar' | 'statusBar') => void;
		onManageAccounts: () => void;
	} = $props();

	const onInstances = $derived(currentPath === '/');
	const onSettings = $derived(currentPath.startsWith('/settings'));
	const current = $derived(accounts.find((a) => a.selected) ?? null);
</script>

{#snippet caption(text: string)}
	<span class="flex items-center gap-1 text-[11.5px] font-medium">{text}</span>
{/snippet}

<div class="toolbar flex shrink-0 items-stretch gap-1 border-b border-line bg-window px-3 py-2">
	<Menu width="236px">
		{#snippet trigger(attrs)}
			<ToolButton
				{...attrs}
				layout="bare"
				icon="menu"
				iconSize={18}
				class="w-9.5 justify-center"
				aria-label={m.menu_main()}
				title={m.menu_main()}
			/>
		{/snippet}

		<MenuItem checked={false} shortcut="Ctrl+N" onclick={onAddInstance}>{m.menu_new_instance()}</MenuItem>
		<MenuSeparator />
		<MenuLabel>{m.menu_folders()}</MenuLabel>
		<MenuItem checked={false} onclick={() => onOpenFolder('instances')}>{m.folder_instances()}</MenuItem>
		<MenuItem checked={false} onclick={() => onOpenFolder('base')}>{m.folder_base()}</MenuItem>
		<MenuItem checked={false} onclick={() => onOpenFolder('store')}>{m.folder_store()}</MenuItem>
		<MenuSeparator />
		<MenuLabel>{m.menu_view()}</MenuLabel>
		<MenuItem checked={onInstances} onclick={() => onNavigate('/')}>{m.nav_instances()}</MenuItem>
		<MenuItem checked={onSettings} onclick={() => onNavigate('/settings')}>{m.nav_settings()}</MenuItem>
		<MenuSeparator />
		<MenuItem checked={view.instanceToolbar} onclick={() => onToggleView('instanceToolbar')}>
			{m.view_instance_toolbar()}
		</MenuItem>
		<MenuItem checked={view.statusBar} onclick={() => onToggleView('statusBar')}>{m.view_status_bar()}</MenuItem>
		<MenuSeparator />
		<MenuItem checked={false} onclick={onAbout}>{m.menu_about()}</MenuItem>
	</Menu>

	<span class="mx-1.25 my-2.25 w-px self-stretch bg-line"></span>

	<ToolButton icon="grid" active={onInstances} aria-current={onInstances} onclick={() => onNavigate('/')}>
		{@render caption(m.nav_instances())}
	</ToolButton>

	<ToolButton
		icon="settings"
		active={onSettings}
		aria-current={onSettings}
		onclick={() => onNavigate('/settings')}
	>
		{@render caption(m.nav_settings())}
	</ToolButton>

	<span class="mx-1.25 my-2.25 w-px self-stretch bg-line"></span>

	<ToolButton icon="plus" onclick={onAddInstance}>
		{@render caption(m.window_new_instance())}
	</ToolButton>

	<Menu width="200px">
		{#snippet trigger(attrs)}
			<ToolButton {...attrs} icon="folder">
				<span class="flex items-center gap-1 text-[11.5px] font-medium">
					{m.menu_folders()} <Icon name="caret" size={9} />
				</span>
			</ToolButton>
		{/snippet}

		<MenuItem icon="folder" onclick={() => onOpenFolder('instances')}>{m.folder_instances()}</MenuItem>
		<MenuItem icon="folder" onclick={() => onOpenFolder('base')}>{m.folder_base()}</MenuItem>
		<MenuItem icon="coffee" onclick={() => onOpenFolder('java')}>{m.folder_java()}</MenuItem>
		<MenuItem icon="settings" onclick={() => onOpenFolder('store')}>{m.folder_store()}</MenuItem>
	</Menu>

	<span class="flex-1"></span>

	<Menu width="240px" placement="bottom-end">
		{#snippet trigger(attrs)}
			<ToolButton
				{...attrs}
				layout="bare"
				class="max-w-52.5 gap-2.25 py-1 pr-3.25 pl-1.25 not-disabled:hover:bg-btn-hi"
			>
				<span class="grid place-items-center rounded-full bg-card p-1">
					<InstanceIcon name="steve" size={22} />
				</span>
				<span class="overflow-hidden font-medium text-ellipsis whitespace-nowrap text-ink">
					{current?.playerName ?? m.account_not_signed_in()}
				</span>
				<Icon name="caret" size={9} />
			</ToolButton>
		{/snippet}

		<MenuLabel>{m.menu_accounts()}</MenuLabel>
		{#each accounts as item (item.id)}
			<MenuItem
				checked={item.selected}
				shortcut={item.kind === 'microsoft'
					? item.expired
						? m.account_needs_relogin()
						: m.account_microsoft()
					: m.account_offline()}
				onclick={() => onSelectAccount(item.id)}
			>
				{item.playerName}
			</MenuItem>
		{:else}
			<div class="px-3 py-2 text-faint">{m.account_none()}</div>
		{/each}
		<MenuSeparator />
		<MenuItem checked={false} onclick={onManageAccounts}>{m.account_manage()}</MenuItem>
	</Menu>
</div>
