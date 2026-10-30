<script lang="ts">
	import { onMount, type Snippet } from 'svelte';
	import { currentRoute, navigate } from '$lib/nav';
	import { page } from '$app/state';
	import MainToolbar from '$lib/components/MainToolbar.svelte';
	import StatusBar from '$lib/components/StatusBar.svelte';
	import UpdateDialog from '$lib/components/UpdateDialog.svelte';
	import { check, type Update } from '@tauri-apps/plugin-updater';
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
	import { m } from '$lib/paraglide/messages.js';

	let { children }: { children: Snippet } = $props();

	let update = $state<Update | null>(null);

	onMount(() => {
		const unlisteners: Array<() => void> = [];
		const track = (p: Promise<() => void>) => p.then((un) => unlisteners.push(un));

		track(api.onInstancesChanged(() => refreshInstances()));
		track(api.onGameStart(() => refreshRunning()));
		track(
			api.onGameExit((exit) => {
				refreshRunning();
				const name = app.instances.find((i) => i.id === exit.instanceName)?.name ?? exit.instanceName;
				report(
					exit.crashed
						? m.status_game_crashed({ name, code: exit.exitCode ?? '?' })
						: m.status_game_exited({ name })
				);
				if (exit.crashed) openAppWindow(windowSpecs.crash(exit.instanceName, name));
			})
		);

		init();
		check()
			.then((found) => (update = found))
			.catch((e) => console.warn('update check failed', e));
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
			report(m.status_account_switched({ name: selectedAccount()?.playerName ?? '' }));
		} catch (e) {
			reportError(m.error_switch_account(), e);
		}
	}

	async function openFolder(target: api.LauncherFolder) {
		try {
			await api.openLauncherFolder(target);
		} catch (e) {
			reportError(m.error_open_folder(), e);
		}
	}
</script>

<svelte:window onkeydown={shortcut} />

<div class="app-shell flex h-screen flex-col">
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
			instanceCount={app.instances.length}
			account={selectedAccount()?.playerName ?? m.account_not_signed_in()}
		/>
	{/if}
</div>

{#if update}
	<UpdateDialog {update} onclose={() => (update = null)} />
{/if}
