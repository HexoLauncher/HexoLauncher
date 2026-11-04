<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import { applyTheme, loadTheme } from '$lib/theme';
	import { applyDocumentLocale } from '$lib/i18n';
	import { getLocale } from '$lib/paraglide/runtime.js';

	let { children } = $props();

	onMount(() => {
		applyDocumentLocale();
		api.setUiLocale(getLocale()).catch(() => {});
		loadTheme();

		let unlisten: (() => void) | undefined;
		api
			.onThemeChanged((theme) => applyTheme(theme))
			.then((un) => (unlisten = un))
			.catch(() => {});

		return () => unlisten?.();
	});
</script>

<div class="app-backdrop" aria-hidden="true">
	<div class="image"></div>
	<div class="dim"></div>
</div>

{@render children()}
