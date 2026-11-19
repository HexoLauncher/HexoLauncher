<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import { applyTheme, loadTheme } from '$lib/theme';
	import { applyDocumentLocale } from '$lib/i18n';

	let { children } = $props();

	function suppressNativeMenu(event: MouseEvent) {
		const target = event.target as HTMLElement | null;
		if (target?.closest('input, textarea, [contenteditable="true"], [data-native-menu]')) return;
		event.preventDefault();
	}

	onMount(() => {
		applyDocumentLocale();
		loadTheme();

		let unlisten: (() => void) | undefined;
		api
			.onThemeChanged((theme) => applyTheme(theme))
			.then((un) => (unlisten = un))
			.catch(() => {});

		return () => unlisten?.();
	});
</script>

<svelte:window oncontextmenu={suppressNativeMenu} />

<div class="app-backdrop" aria-hidden="true">
	<div class="image"></div>
	<div class="dim"></div>
</div>

{@render children()}
