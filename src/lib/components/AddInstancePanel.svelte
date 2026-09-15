<script lang="ts">
	import Icon from '$lib/Icon.svelte';
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import WindowShell from './WindowShell.svelte';
	import PageList from './PageList.svelte';
	import Menu from './Menu.svelte';
	import ModpackBrowser from './ModpackBrowser.svelte';
	import VersionPicker from './VersionPicker.svelte';
	import * as api from '$lib/api';
	import type {
		InstanceView,
		LoaderKind,
		ModpackInstallResult,
		ModpackPlatform,
		ModpackPreview,
		PackHit,
		PackTarget,
		RemoteVersion
	} from '$lib/api';
	import { loaderLabel } from '$lib/format';
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
		{ id: 'modrinth', label: 'Modrinth', icon: 'globe' },
		{ id: 'curseforge', label: 'CurseForge', icon: 'globe' },
		{ id: 'ftb', label: 'FTB', icon: 'globe' },
		{ id: 'atlauncher', label: 'ATLauncher', icon: 'globe' },
		{ id: 'modpack', label: '匯入模組包', icon: 'package' },
		{ id: 'import', label: '從壓縮檔匯入', icon: 'upload' }
	];

	const PLATFORMS: string[] = ['modrinth', 'curseforge', 'ftb', 'atlauncher'];

	const FORMAT_LABELS: Record<api.ModpackFormat, string> = {
		modrinth: 'Modrinth',
		curseforge: 'CurseForge',
		atlauncher: 'ATLauncher',
		ftb: 'FTB'
	};

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

	let packPath = $state('');
	let filePreview = $state<ModpackPreview | null>(null);

	let remoteHit = $state<PackHit | null>(null);
	let remoteVersion = $state<RemoteVersion | null>(null);
	let includeOptional = $state(true);


	let autoName = '';


	let result = $state<ModpackInstallResult | null>(null);

	const platform = $derived(PLATFORMS.includes(page) ? (page as ModpackPlatform) : null);

	const action = $derived.by(() => {
		if (platform) {
			return {
				label: '安裝',
				icon: 'download',
				ready: !!remoteHit && !!remoteVersion,
				run: () => {
					const hit = remoteHit;
					const version = remoteVersion;
					if (!hit || !version) return;
					installPack(
						hit.name,
						(target) =>
							api.installRemoteModpack(target, platform, hit.id, version.id, includeOptional),
						'正在下載模組包…'
					);
				}
			};
		}
		switch (page) {
			case 'custom':
				return { label: '確定', icon: 'plus', ready: !!name.trim() && !!mcVersion, run: submit };
			case 'modpack':
				return {
					label: '安裝',
					icon: 'download',
					ready: !!filePreview,
					run: () =>
						filePreview &&
						installPack(filePreview.name, (target) =>
							api.installModpackFile(target, packPath.trim())
						)
				};
			default:
				return { label: '匯入', icon: 'upload', ready: !!zipPath.trim(), run: importZip };
		}
	});

	function selectPage(id: string) {
		page = id;
		error = '';
		remoteHit = null;
		remoteVersion = null;
	}

	async function submit() {
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

	async function pickFile(filterName: string, extensions: string[]) {
		try {
			const { open } = await import('@tauri-apps/api/dialog');
			const picked = await open({ filters: [{ name: filterName, extensions }] });
			return typeof picked === 'string' ? picked : null;
		} catch {
			error = '這個環境沒有系統對話框，請直接貼上檔案路徑';
			return null;
		}
	}

	async function pickZip() {
		const picked = await pickFile('壓縮檔', ['zip']);
		if (picked) zipPath = picked;
	}

	async function importZip() {
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

	function suggestName(packName: string) {
		if (!name.trim() || name === autoName) name = packName;
		autoName = packName;
	}

	async function pickPack() {
		const picked = await pickFile('模組包', ['mrpack', 'zip']);
		if (!picked) return;
		packPath = picked;
		await inspectPack();
	}

	async function inspectPack() {
		filePreview = null;
		if (!packPath.trim()) return;
		error = '';
		busy = '正在讀取模組包…';
		try {
			filePreview = await api.inspectModpack(packPath.trim());
			suggestName(filePreview.name);
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			busy = '';
		}
	}

	async function installPack(
		packName: string,
		run: (target: PackTarget) => Promise<ModpackInstallResult>,
		initial = '正在準備安裝…'
	) {
		error = '';
		busy = initial;
		// 名稱一定明確送出，後端才會用跟 suggestInstanceId 一樣的名稱決定資料夾。
		const instanceName = name.trim() || packName;
		let unlisten: (() => void) | undefined;
		try {
			const id = await api.suggestInstanceId(baseDir, instanceName);
			unlisten = await api.onInstallProgress((p) => {
				if (p.instanceName === id) {
					busy = p.total > 1 ? `${p.message}（${p.done} / ${p.total}）` : p.message;
				}
			});

			const installed = await run({ baseDir, name: instanceName, icon, group });
			if (installed.manualDownloads.length || installed.skipped.length) {
				result = installed;
			} else {
				oncreated(installed.instance);
			}
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			unlisten?.();
			busy = '';
		}
	}

	function attempt(task: Promise<unknown>) {
		task.catch((e) => (error = api.errorMessage(e)));
	}
</script>

{#snippet identity(placeholder: string)}
	<div class="flex shrink-0 items-start gap-3">
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
				<input id="inst-name" class="qt-input" bind:value={name} {placeholder} />
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
{/snippet}

{#snippet packSummary(preview: ModpackPreview)}
	<dl
		class="grid shrink-0 grid-cols-[96px_1fr] gap-x-3 gap-y-1.5 rounded-lg border border-line bg-deep px-3.5 py-3"
	>
		<dt class="text-muted">模組包</dt>
		<dd class="min-w-0 truncate text-ink">
			{preview.name}{preview.version ? ` ${preview.version}` : ''}
		</dd>
		<dt class="text-muted">格式</dt>
		<dd>{FORMAT_LABELS[preview.format]}</dd>
		<dt class="text-muted">Minecraft</dt>
		<dd>{preview.mcVersion}</dd>
		<dt class="text-muted">模組載入器</dt>
		<dd>
			{preview.loader === 'vanilla'
				? '無'
				: `${loaderLabel(preview.loader)} ${preview.loaderVersion ?? '（最新版）'}`}
		</dd>
		<dt class="text-muted">要下載的檔案</dt>
		<dd>{preview.fileCount} 個</dd>
	</dl>
{/snippet}

<WindowShell>
	<div class="flex min-h-0 flex-1">
		{#if !result}
			<PageList {pages} selected={page} onselect={selectPage} width="162px" />
		{/if}

		<div class="flex min-w-0 flex-1 flex-col gap-3.5 overflow-y-auto px-4.5 py-4">
			{#if result}
				{@const done = result}
				<h2 class="text-[15px] font-semibold text-ink">
					已安裝「{done.instance.name}」，還有些檔案需要處理
				</h2>

				{#if done.manualDownloads.length}
					<p class="leading-[1.8] text-muted">
						這些檔案沒辦法自動下載（作者不允許第三方下載，或沒有設定 CurseForge API 金鑰）。請用瀏覽器下載，放到列出的位置，否則遊戲可能無法啟動。
					</p>
					<ul class="min-h-0 flex-1 overflow-y-auto rounded-lg border border-line bg-deep">
						{#each done.manualDownloads as file (file.dest)}
							<li class="flex items-center gap-3 border-b border-line px-3 py-2 last:border-b-0">
								<div class="min-w-0 flex-1">
									<div class="truncate text-ink">{file.name}</div>
									<div class="truncate text-faint">{file.dest}</div>
								</div>
								{#if file.website}
									{@const website = file.website}
									<button class="qt-button" onclick={() => attempt(api.openExternal(website))}>
										<Icon name="globe" size={13} />下載頁面
									</button>
								{/if}
								<button
									class="qt-button"
									onclick={() =>
										attempt(api.openInstanceFolder(baseDir, done.instance.id, file.folder))}
								>
									<Icon name="folder" size={13} />資料夾
								</button>
							</li>
						{/each}
					</ul>
				{/if}

				{#if done.skipped.length}
					<p class="leading-[1.8] text-muted">
						不支援而略過的項目：{done.skipped.join('、')}
					</p>
				{/if}
			{:else if platform}
				{#key platform}
					<ModpackBrowser
						{platform}
						bind:hit={remoteHit}
						bind:version={remoteVersion}
						bind:includeOptional
						disabled={!!busy}
						onchoose={(hit) => suggestName(hit.name)}
						onerror={(message) => (error = message)}
					/>
				{/key}
				{#if remoteHit}
					{@render identity(remoteHit.name)}
				{/if}
			{:else if page === 'custom'}
				{@render identity('例如：和朋友的生存日常')}

				<VersionPicker
					bind:mcVersion
					bind:loader
					bind:loaderVersion
					onError={(message) => (error = message)}
				/>
			{:else if page === 'modpack'}
				<div class="qt-field">
					<label for="pack-path">模組包</label>
					<div class="flex gap-1.75">
						<input
							id="pack-path"
							class="qt-input min-w-0 flex-1"
							bind:value={packPath}
							oninput={() => (filePreview = null)}
							onchange={inspectPack}
							placeholder="Modrinth 的 .mrpack，或 CurseForge 的 .zip"
						/>
						<button class="qt-button" disabled={!!busy} onclick={pickPack}>瀏覽…</button>
					</div>
				</div>

				{#if filePreview}
					{@render identity(filePreview.name)}
					{@render packSummary(filePreview)}
					{#if filePreview.format === 'curseforge'}
						<p class="px-0.5 leading-[1.8] text-faint">
							CurseForge 模組包會用「設定」裡的 API 金鑰下載檔案。作者不允許第三方下載的檔案，裝完後會列出來讓你自己下載。
						</p>
					{/if}
				{:else}
					<p class="px-0.5 leading-[1.8] text-faint">
						只會放入模組包的檔案；Minecraft 與模組載入器在第一次啟動時才下載。
					</p>
				{/if}
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
			<span class="mr-auto flex min-w-0 items-center gap-2 pl-1 text-muted">
				<span
					class="size-4 shrink-0 animate-spin rounded-full border-2 border-line border-t-brand"
				></span>
				<span class="truncate">{busy}</span>
			</span>
		{/if}
		{#if result}
			{@const done = result}
			<button class="qt-button primary" onclick={() => oncreated(done.instance)}>
				<Icon name="check" size={13} />完成
			</button>
		{:else}
			<button class="qt-button primary" disabled={!action.ready || !!busy} onclick={action.run}>
				<Icon name={action.icon} size={13} />{action.label}
			</button>
			<button class="qt-button" disabled={!!busy} onclick={onclose}>取消</button>
		{/if}
	{/snippet}
</WindowShell>
