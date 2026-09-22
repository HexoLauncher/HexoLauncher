<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * 二級視窗共用的外框。
	 * 這些路由是被 openAppWindow() 載進另一個真正的視窗，
	 * 所以不掛主視窗的工具列與狀態列。
	 */
	let { children }: { children: Snippet } = $props();
</script>

<div class="app-shell h-screen">
	{@render children()}
</div>

