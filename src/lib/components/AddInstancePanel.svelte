<script lang="ts">
	import Icon from '$lib/Icon.svelte';
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import WindowShell from './WindowShell.svelte';
	import PageList from './PageList.svelte';
	import Menu from './Menu.svelte';
	import VersionPicker from './VersionPicker.svelte';
	import * as api from '$lib/api';
	import type { InstanceView, LoaderKind } from '$lib/api';
	import { ICON_CHOICES } from '$lib/icons';
	import type { Page } from '$lib/ui';

	let {
		baseDir,
		groups,
		onclose,
		oncreated
	}: {
		baseDir: string;
		groups: string[];
		onclose: () => void;
		oncreated: (instance: InstanceView) => void;
	} = $props();

	const pages: Page[] = [
		{ id: 'custom', label: '自訂', icon: 'box' },
		{ id: 'import', label: '從壓縮檔匯入', icon: 'package' }
	];

	let page = $state('custom');
	let name = $state('');
	let icon = $state('grass');
	let group = $state('');
	let loader = $state<LoaderKind>('fabric');
	let mcVersion = $state('');
	let loaderVersion = $state('');
	let iconMenu = $state(false);

	let busy = $state('');
	let error = $state('');
	let zipPath = $state('');

	const ready = $derived(!!name.trim() && !!mcVersion && !busy);


	async function submit() {
		if (!ready) return;
		error = '';
		busy = '正在建立實例…';
		try {
			const created = await api.createInstance({
				baseDir,
				name: name.trim(),
				icon,
				group,
				mcVersion,
				loader,
				loaderVersion: loader === 'vanilla' ? null : loaderVersion || null
			});
			oncreated(created);
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			busy = '';
		}
	}

	async function pickZip() {
		try {
			const { open } = await import('@tauri-apps/api/dialog');
			const picked = await open({ filters: [{ name: '壓縮檔', extensions: ['zip'] }] });
			if (typeof picked === 'string') zipPath = picked;
		} catch {
			error = '這個環境沒有系統對話框，請直接貼上檔案路徑';
		}
	}

	async function importZip() {
		if (!zipPath.trim()) return;
		error = '';
		busy = '正在解壓縮…';
		try {
			oncreated(await api.importInstance(baseDir, zipPath.trim()));
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			busy = '';
		}
	}
</script>

<WindowShell>
	<div class="flex min-h-0 flex-1">
		<PageList {pages} selected={page} onselect={(id) => (page = id)} width="162px" />

		<div class="flex min-w-0 flex-1 flex-col gap-3.5 px-4.5 py-4">
			{#if page === 'custom'}
				<div class="flex items-start gap-3">
					<div class="relative">
						<button
							class="qt-button size-14.5 rounded-lg p-0"
							aria-label="選擇圖示"
							onclick={() => (iconMenu = !iconMenu)}
						>
							<InstanceIcon name={icon} size={34} />
						</button>
						<Menu open={iconMenu} onclose={() => (iconMenu = false)} width="228px">
							<div class="grid grid-cols-5 gap-0.75 p-0.75">
								{#each ICON_CHOICES as choice (choice)}
									<button
										class="grid place-items-center rounded-sm border border-transparent p-1.25 hover:bg-white/10 {icon ===
										choice
											? 'border-brand bg-brand-soft'
											: ''}"
										aria-label={choice}
										onclick={() => (icon = choice)}
									>
										<InstanceIcon name={choice} size={30} />
									</button>
								{/each}
							</div>
						</Menu>
					</div>

					<div class="min-w-0 flex-1 [&_.qt-field]:grid-cols-[48px_1fr]">
						<div class="qt-field">
							<label for="inst-name">名稱</label>
							<input
								id="inst-name"
								class="qt-input"
								bind:value={name}
								placeholder="例如：和朋友的生存日常"
							/>
						</div>
						<div class="qt-field">
							<label for="inst-group">群組</label>
							<input
								id="inst-group"
								class="qt-input"
								bind:value={group}
								list="group-suggestions"
								placeholder="（未分組）"
							/>
							<datalist id="group-suggestions">
								{#each groups as g (g)}
									<option value={g}></option>
								{/each}
							</datalist>
						</div>
					</div>
				</div>

				<VersionPicker
					bind:mcVersion
					bind:loader
					bind:loaderVersion
					onError={(message) => (error = message)}
				/>
			{:else}
				<div class="flex min-h-0 flex-1 flex-col gap-3">
					<div class="qt-field">
						<label for="zip-path">壓縮檔</label>
						<div class="flex gap-1.75">
							<input
								id="zip-path"
								class="qt-input min-w-0 flex-1"
								bind:value={zipPath}
								placeholder="選擇由「匯出實例」產生的 .zip"
							/>
							<button class="qt-button" onclick={pickZip}>瀏覽…</button>
						</div>
					</div>

					<p class="px-0.5 leading-[1.8] text-faint">
						也可以直接把整個實例資料夾複製到實例目錄底下。
					</p>

					<div class="flex gap-1.75">
						<button
							class="qt-button primary"
							disabled={!zipPath.trim() || !!busy}
							onclick={importZip}
						>
							匯入
						</button>
						<button class="qt-button" onclick={() => api.openLauncherFolder('instances')}>
							開啟實例資料夾
						</button>
					</div>
				</div>
			{/if}

			{#if error}
				<p class="shrink-0 rounded-lg bg-danger-soft px-3 py-2 leading-[1.7] text-danger">
					{error}
				</p>
			{/if}
		</div>
	</div>

	{#snippet footer()}
		{#if busy}
			<span class="mr-auto flex items-center gap-2 pl-1 text-muted">
				<span class="size-4 animate-spin rounded-full border-2 border-line border-t-brand"></span>
				{busy}
			</span>
		{/if}
		<button class="qt-button primary" disabled={!ready || page !== 'custom'} onclick={submit}>
			<Icon name="plus" size={13} />確定
		</button>
		<button class="qt-button" disabled={!!busy} onclick={onclose}>取消</button>
	{/snippet}
</WindowShell>
