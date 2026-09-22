<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import { applyTheme, loadTheme } from '$lib/theme';

	let { children } = $props();

	onMount(() => {
		// 每個視窗（主視窗、編輯實例、主控台…）都是各自的 webview，各自載一次主題。
		loadTheme();

		let unlisten: (() => void) | undefined;
		api
			.onThemeChanged((theme) => applyTheme(theme))
			.then((un) => (unlisten = un))
			.catch(() => {});

		return () => unlisten?.();
	});
</script>

<!-- 背景圖層。壓在所有內容底下，半透明的面板就是透到這裡為止。 -->
<div class="app-backdrop" aria-hidden="true">
	<div class="image"></div>
	<div class="dim"></div>
</div>

{@render children()}
