<script lang="ts">
	import { page } from '$app/state';
	import InstanceEditor from '$lib/components/InstanceEditor.svelte';
	import * as api from '$lib/api';
	import type { InstanceView } from '$lib/api';
	import { closeSelf } from '$lib/windows';

	/**
	 * 這裡是獨立視窗，拿不到主視窗的記憶體狀態，所以自己依 id 跟後端要資料。
	 * 存檔之後後端會發 instances-changed，主視窗自己會更新。
	 */
	const id = $derived(page.params.id ?? '');

	let baseDir = $state('');
	let instance = $state<InstanceView | null>(null);
	let error = $state('');
	let loadedId = '';

	$effect(() => {
		if (!id || loadedId === id) return;
		loadedId = id;
		load(id);
	});

	async function load(instanceName: string) {
		error = '';
		try {
			const paths = await api.resolvePaths();
			baseDir = paths.baseDir;
			instance = await api.getInstance(baseDir, instanceName);
		} catch (e) {
			error = api.errorMessage(e);
		}
	}
</script>

<svelte:head>
	<title>{instance ? `${instance.name} — 編輯實例` : '編輯實例'}</title>
</svelte:head>

{#if instance}
	<InstanceEditor
		{baseDir}
		{instance}
		onclose={closeSelf}
		onchanged={(updated) => (instance = updated)}
	/>
{:else if error}
	<div class="flex h-full flex-col items-center justify-center gap-3.5 text-muted">
		<h2 class="text-[17px] font-semibold text-ink">找不到實例</h2>
		<p>{error}</p>
		<p class="text-faint">
			路由參數 <code class="rounded-sm bg-deep px-1.5 py-0.5 font-mono">{id || '（空的）'}</code>
		</p>
		<button class="qt-button primary" onclick={closeSelf}>關閉視窗</button>
	</div>
{:else}
	<div class="flex h-full items-center justify-center gap-3 text-muted">
		<span class="size-5 animate-spin rounded-full border-2 border-line border-t-brand"></span>
		正在讀取實例…
	</div>
{/if}
