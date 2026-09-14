<script lang="ts">
	import SettingsPanel from '$lib/components/SettingsPanel.svelte';
	import { app, report, saveConfig, saveConfigSoon } from '$lib/state.svelte';
	import { page } from '$app/state';


	const initialPage = $derived(page.params.section ?? 'launcher');

	const paths = $derived({
		baseDir: app.baseDir,
		defaultBaseDir: app.defaultBaseDir,
		storeDir: app.storeDir
	});
</script>

<svelte:head><title>設定 — Hexo Launcher</title></svelte:head>

{#key initialPage}
	<SettingsPanel
		config={app.config}
		accounts={app.accounts}
		{paths}
		{initialPage}
		save={(immediate) => (immediate ? saveConfig() : saveConfigSoon())}
		onAccounts={(accounts) => (app.accounts = accounts)}
		{report}
	/>
{/key}
