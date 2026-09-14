<script lang="ts">
	import Icon from '$lib/Icon.svelte';
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import Menu from './Menu.svelte';
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

	let mainMenu = $state(false);
	let folderMenu = $state(false);
	let accountMenu = $state(false);
</script>

<div class="toolbar flex shrink-0 items-stretch gap-1 border-b border-line bg-window px-3 py-2">
	<div class="relative flex">
		<button
			class="tool-button w-9.5 justify-center"
			aria-label="主選單"
			title="主選單"
			onclick={() => (mainMenu = !mainMenu)}
		>
			<Icon name="menu" size={18} />
		</button>
		<Menu open={mainMenu} onclose={() => (mainMenu = false)} width="236px">
			<button class="qt-menu-item" onclick={onAddInstance}>
				<span class="check-slot"></span>新增實例…<span class="shortcut">Ctrl+N</span>
			</button>
			<hr />
			<div class="qt-menu-label">資料夾</div>
			<button class="qt-menu-item" onclick={() => onOpenFolder('instances')}>
				<span class="check-slot"></span>實例資料夾
			</button>
			<button class="qt-menu-item" onclick={() => onOpenFolder('base')}>
				<span class="check-slot"></span>資料根目錄
			</button>
			<button class="qt-menu-item" onclick={() => onOpenFolder('store')}>
				<span class="check-slot"></span>啟動器設定資料夾
			</button>
			<hr />
			<div class="qt-menu-label">檢視</div>
			<button class="qt-menu-item" onclick={() => onNavigate('/')}>
				<span class="check-slot">{onInstances ? '✓' : ''}</span>遊戲實例
			</button>
			<button class="qt-menu-item" onclick={() => onNavigate('/settings')}>
				<span class="check-slot">{onSettings ? '✓' : ''}</span>設定
			</button>
			<hr />
			<button class="qt-menu-item" onclick={() => onToggleView('instanceToolbar')}>
				<span class="check-slot">{view.instanceToolbar ? '✓' : ''}</span>實例工具列
			</button>
			<button class="qt-menu-item" onclick={() => onToggleView('statusBar')}>
				<span class="check-slot">{view.statusBar ? '✓' : ''}</span>狀態列
			</button>
			<hr />
			<button class="qt-menu-item" onclick={onAbout}>
				<span class="check-slot"></span>關於 Hexo Launcher
			</button>
		</Menu>
	</div>

	<span class="mx-1.25 my-2.25 w-px self-stretch bg-line"></span>


	<button
		class="tool-button under"
		class:active={onInstances}
		aria-current={onInstances}
		onclick={() => onNavigate('/')}
	>
		<Icon name="grid" size={22} />
		<span class="flex items-center gap-1 text-[11.5px] font-medium">遊戲實例</span>
	</button>

	<button
		class="tool-button under"
		class:active={onSettings}
		aria-current={onSettings}
		onclick={() => onNavigate('/settings')}
	>
		<Icon name="settings" size={22} />
		<span class="flex items-center gap-1 text-[11.5px] font-medium">設定</span>
	</button>

	<span class="mx-1.25 my-2.25 w-px self-stretch bg-line"></span>


	<button class="tool-button under" onclick={onAddInstance}>
		<Icon name="plus" size={22} />
		<span class="flex items-center gap-1 text-[11.5px] font-medium">新增實例</span>
	</button>

	<div class="relative flex">
		<button class="tool-button under" onclick={() => (folderMenu = !folderMenu)}>
			<Icon name="folder" size={22} />
			<span class="flex items-center gap-1 text-[11.5px] font-medium">
				資料夾 <Icon name="caret" size={9} />
			</span>
		</button>
		<Menu open={folderMenu} onclose={() => (folderMenu = false)} width="200px">
			<button class="qt-menu-item" onclick={() => onOpenFolder('instances')}>
				<Icon name="folder" size={14} />實例資料夾
			</button>
			<button class="qt-menu-item" onclick={() => onOpenFolder('base')}>
				<Icon name="folder" size={14} />資料根目錄
			</button>
			<button class="qt-menu-item" onclick={() => onOpenFolder('java')}>
				<Icon name="coffee" size={14} />下載的 Java
			</button>
			<button class="qt-menu-item" onclick={() => onOpenFolder('store')}>
				<Icon name="settings" size={14} />啟動器設定資料夾
			</button>
		</Menu>
	</div>

	<span class="flex-1"></span>

	<div class="relative flex">

		<button
			class="tool-button max-w-52.5 gap-2.25 bg-btn py-1 pl-1.25 pr-3.25
				not-disabled:hover:bg-btn-hi"
			onclick={() => (accountMenu = !accountMenu)}
		>
			<span class="grid place-items-center rounded-full bg-card p-1">
				<InstanceIcon name="steve" size={22} />
			</span>
			<span class="overflow-hidden text-ellipsis whitespace-nowrap font-medium text-ink">
				{current?.playerName ?? '尚未登入'}
			</span>
			<Icon name="caret" size={9} />
		</button>
		<Menu open={accountMenu} onclose={() => (accountMenu = false)} align="right" width="240px">
			<div class="qt-menu-label">帳號</div>
			{#each accounts as item (item.id)}
				<button class="qt-menu-item" onclick={() => onSelectAccount(item.id)}>
					<span class="check-slot">{item.selected ? '✓' : ''}</span>
					{item.playerName}
					<span class="shortcut">
						{item.kind === 'microsoft' ? (item.expired ? '需重新登入' : 'Microsoft') : '離線'}
					</span>
				</button>
			{:else}
				<div class="px-3 py-2 text-faint">還沒有任何帳號</div>
			{/each}
			<hr />
			<button class="qt-menu-item" onclick={onManageAccounts}>
				<span class="check-slot"></span>管理帳號…
			</button>
		</Menu>
	</div>
</div>
