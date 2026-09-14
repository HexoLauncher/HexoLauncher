<script lang="ts">
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import WindowShell from '$lib/components/WindowShell.svelte';
	import * as api from '$lib/api';
	import { closeSelf } from '$lib/windows';

	let paths = $state<api.Paths | null>(null);
	api.resolvePaths().then((p) => (paths = p));
</script>

<svelte:head><title>關於 Hexo Launcher</title></svelte:head>

<WindowShell>
	<div class="flex flex-1 flex-col items-center justify-center gap-2.25 px-7.5 py-6 text-center">
		<InstanceIcon name="grass" size={56} />
		<h1 class="mt-1.5 text-[20px] font-bold tracking-[-0.3px]">Hexo Launcher</h1>
		<p class="text-faint">版本 0.1.0</p>
		<p class="mt-2 leading-[1.9] text-muted">遊戲的下載、安裝與啟動由 hexomc-lib 負責。</p>

		{#if paths}
			<dl class="mt-3 w-full text-left text-[11.5px] text-faint">
				<div class="flex gap-2 py-0.5">
					<dt class="w-20 shrink-0">資料目錄</dt>
					<dd class="min-w-0 truncate font-mono">{paths.baseDir}</dd>
				</div>
				<div class="flex gap-2 py-0.5">
					<dt class="w-20 shrink-0">設定目錄</dt>
					<dd class="min-w-0 truncate font-mono">{paths.storeDir}</dd>
				</div>
			</dl>
		{/if}
	</div>

	{#snippet footer()}
		<button class="qt-button primary" onclick={closeSelf}>關閉</button>
	{/snippet}
</WindowShell>
