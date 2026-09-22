<script lang="ts">
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import { Icon, Menu, MenuItem, MenuLabel, MenuSeparator, ToolButton } from '$lib/ui';
	import type { AccountView, LauncherFolder, UiConfig } from '$lib/api';

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
				aria-label="主選單"
				title="主選單"
			/>
		{/snippet}

		<MenuItem checked={false} shortcut="Ctrl+N" onclick={onAddInstance}>新增實例…</MenuItem>
		<MenuSeparator />
		<MenuLabel>資料夾</MenuLabel>
		<MenuItem checked={false} onclick={() => onOpenFolder('instances')}>實例資料夾</MenuItem>
		<MenuItem checked={false} onclick={() => onOpenFolder('base')}>資料根目錄</MenuItem>
		<MenuItem checked={false} onclick={() => onOpenFolder('store')}>啟動器設定資料夾</MenuItem>
		<MenuSeparator />
		<MenuLabel>檢視</MenuLabel>
		<MenuItem checked={onInstances} onclick={() => onNavigate('/')}>遊戲實例</MenuItem>
		<MenuItem checked={onSettings} onclick={() => onNavigate('/settings')}>設定</MenuItem>
		<MenuSeparator />
		<MenuItem checked={view.instanceToolbar} onclick={() => onToggleView('instanceToolbar')}>
			實例工具列
		</MenuItem>
		<MenuItem checked={view.statusBar} onclick={() => onToggleView('statusBar')}>狀態列</MenuItem>
		<MenuSeparator />
		<MenuItem checked={false} onclick={onAbout}>關於 Hexo Launcher</MenuItem>
	</Menu>

	<span class="mx-1.25 my-2.25 w-px self-stretch bg-line"></span>

	<ToolButton icon="grid" active={onInstances} aria-current={onInstances} onclick={() => onNavigate('/')}>
		{@render caption('遊戲實例')}
	</ToolButton>

	<ToolButton
		icon="settings"
		active={onSettings}
		aria-current={onSettings}
		onclick={() => onNavigate('/settings')}
	>
		{@render caption('設定')}
	</ToolButton>

	<span class="mx-1.25 my-2.25 w-px self-stretch bg-line"></span>

	<ToolButton icon="plus" onclick={onAddInstance}>
		{@render caption('新增實例')}
	</ToolButton>

	<Menu width="200px">
		{#snippet trigger(attrs)}
			<ToolButton {...attrs} icon="folder">
				<span class="flex items-center gap-1 text-[11.5px] font-medium">
					資料夾 <Icon name="caret" size={9} />
				</span>
			</ToolButton>
		{/snippet}

		<MenuItem icon="folder" onclick={() => onOpenFolder('instances')}>實例資料夾</MenuItem>
		<MenuItem icon="folder" onclick={() => onOpenFolder('base')}>資料根目錄</MenuItem>
		<MenuItem icon="coffee" onclick={() => onOpenFolder('java')}>下載的 Java</MenuItem>
		<MenuItem icon="settings" onclick={() => onOpenFolder('store')}>啟動器設定資料夾</MenuItem>
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
					{current?.playerName ?? '尚未登入'}
				</span>
				<Icon name="caret" size={9} />
			</ToolButton>
		{/snippet}

		<MenuLabel>帳號</MenuLabel>
		{#each accounts as item (item.id)}
			<MenuItem
				checked={item.selected}
				shortcut={item.kind === 'microsoft'
					? item.expired
						? '需重新登入'
						: 'Microsoft'
					: '離線'}
				onclick={() => onSelectAccount(item.id)}
			>
				{item.playerName}
			</MenuItem>
		{:else}
			<div class="px-3 py-2 text-faint">還沒有任何帳號</div>
		{/each}
		<MenuSeparator />
		<MenuItem checked={false} onclick={onManageAccounts}>管理帳號…</MenuItem>
	</Menu>
</div>
