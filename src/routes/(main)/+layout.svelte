<script lang="ts">
	import { onMount, type Snippet } from 'svelte';
	import { currentRoute, navigate } from '$lib/nav';
	import { page } from '$app/state';
	import MainToolbar from '$lib/components/MainToolbar.svelte';
	import StatusBar from '$lib/components/StatusBar.svelte';
	import * as api from '$lib/api';
	import {
		app,
		init,
		refreshAccounts,
		refreshInstances,
		refreshRunning,
		report,
		reportError,
		saveConfig,
		selectedAccount
	} from '$lib/state.svelte';
	import { openAppWindow, windowSpecs } from '$lib/windows';

	let { children }: { children: Snippet } = $props();

	onMount(() => {
		const unlisteners: Array<() => void> = [];
		const track = (p: Promise<() => void>) => p.then((un) => unlisteners.push(un));

		// 二級視窗（新增實例、編輯）改完東西之後，主視窗靠這些事件跟上。
		track(api.onInstancesChanged(() => refreshInstances()));
		track(api.onGameStart(() => refreshRunning()));
		track(
			api.onGameExit((exit) => {
				refreshRunning();
				const name = app.instances.find((i) => i.id === exit.instanceName)?.name ?? exit.instanceName;
				report(
					exit.exitCode === 0 || exit.exitCode === null
						? `${name} 已結束`
						: `${name} 異常結束（代碼 ${exit.exitCode}）`
				);
			})
		);

		init();
		return () => unlisteners.forEach((un) => un());
	});

	function shortcut(event: KeyboardEvent) {
		if (event.ctrlKey && event.key.toLowerCase() === 'n') {
			event.preventDefault();
			openAppWindow(windowSpecs.newInstance());
		} else if (event.ctrlKey && event.key === ',') {
			event.preventDefault();
			navigate('/settings');
		}
	}

	async function pickAccount(id: string) {
		try {
			app.accounts = await api.selectAccount(id);
			report(`已切換至帳號 ${selectedAccount()?.playerName ?? ''}`);
		} catch (e) {
			reportError('切換帳號失敗', e);
		}
	}

	async function openFolder(target: api.LauncherFolder) {
		try {
			await api.openLauncherFolder(target);
		} catch (e) {
			reportError('開啟資料夾失敗', e);
		}
	}
</script>

<svelte:window onkeydown={shortcut} />

<div class="flex h-screen flex-col bg-window">
	<MainToolbar
		accounts={app.accounts}
		view={app.config.ui}
		currentPath={currentRoute(page.url)}
		onNavigate={navigate}
		onAddInstance={() => openAppWindow(windowSpecs.newInstance())}
		onAbout={() => openAppWindow(windowSpecs.about())}
		onOpenFolder={openFolder}
		onSelectAccount={pickAccount}
		onToggleView={(key) => {
			app.config.ui[key] = !app.config.ui[key];
			saveConfig();
		}}
		onManageAccounts={() => {
			refreshAccounts();
			navigate('/settings/accounts');
		}}
	/>

	<div class="flex min-h-0 flex-1">
		{@render children()}
	</div>

	{#if app.config.ui.statusBar}
		<StatusBar
			message={app.status}
			baseDir={app.baseDir}
			instanceCount={app.instances.length}
			account={selectedAccount()?.playerName ?? '尚未登入'}
			onOpenBaseDir={() => openFolder('base')}
		/>
	{/if}
</div>
