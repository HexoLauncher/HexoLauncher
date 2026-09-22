<script lang="ts">
	import { Button } from '$lib/ui';
	import AddInstancePanel from '$lib/components/AddInstancePanel.svelte';
	import * as api from '$lib/api';
	import { closeSelf } from '$lib/windows';


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
		<Button variant="primary" onclick={closeSelf}>關閉視窗</Button>
	</div>
{:else if error}
	<div class="flex h-full flex-col items-center justify-center gap-3 text-muted">
		<h2 class="text-[17px] font-semibold text-ink">無法讀取啟動器設定</h2>
		<p>{error}</p>
		<Button onclick={load}>重試</Button>
	</div>
{:else if baseDir}
	<AddInstancePanel
		{baseDir}
		{groups}
		onclose={closeSelf}
		oncreated={(instance) => (created = instance.name)}
	/>
{/if}
