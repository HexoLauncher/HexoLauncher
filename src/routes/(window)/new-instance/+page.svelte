<script lang="ts">
	import AddInstancePanel from '$lib/components/AddInstancePanel.svelte';
	import * as api from '$lib/api';
	import { closeSelf } from '$lib/windows';

	/**
	 * 這是獨立視窗，拿不到主視窗的記憶體狀態，所以自己跟後端要資料。
	 * 建好之後後端會發 instances-changed，主視窗自己會更新清單。
	 */
	let baseDir = $state('');
	let groups = $state<string[]>([]);
	let created = $state<string | null>(null);
	let error = $state('');

	async function load() {
		try {
			const paths = await api.resolvePaths();
			baseDir = paths.baseDir;
			const instances = await api.listInstances(baseDir);
			groups = instances
				.map((i) => i.group)
				.filter((g, i, all) => g !== '' && all.indexOf(g) === i)
				.sort();
		} catch (e) {
			error = api.errorMessage(e);
		}
	}

	load();
</script>

<svelte:head><title>新增實例 — Hexo Launcher</title></svelte:head>

{#if created}
	<div class="flex h-full flex-col items-center justify-center gap-3 text-muted">
		<h2 class="text-[17px] font-semibold text-ink">已建立「{created}」</h2>
		<p>實例已經出現在主視窗的清單裡。</p>
		<button class="qt-button primary" onclick={closeSelf}>關閉視窗</button>
	</div>
{:else if error}
	<div class="flex h-full flex-col items-center justify-center gap-3 text-muted">
		<h2 class="text-[17px] font-semibold text-ink">無法讀取啟動器設定</h2>
		<p>{error}</p>
		<button class="qt-button" onclick={load}>重試</button>
	</div>
{:else if baseDir}
	<AddInstancePanel
		{baseDir}
		{groups}
		onclose={closeSelf}
		oncreated={(instance) => (created = instance.name)}
	/>
{/if}
