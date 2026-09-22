<script lang="ts">
	import { Button, CheckRow, Field, Menu, Select, Table, TableBox, TextInput } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import WindowShell from './WindowShell.svelte';
	import NavList from '$lib/ui/NavList.svelte';
	import ModpackBrowser from './ModpackBrowser.svelte';
	import VersionPicker from './VersionPicker.svelte';
	import * as api from '$lib/api';
	import type {
		ForeignImportResult,
		ForeignInstance,
		ForeignLauncher,
		ImportMode,
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
	import { SvelteSet } from 'svelte/reactivity';
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
		{ id: 'launcher', label: '從其他啟動器', icon: 'shortcut' },
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

	/** 從別的啟動器匯入：偵測到的啟動器、目前選的來源、它底下的實例。 */
	let launchers = $state<ForeignLauncher[]>([]);
	let launcherPath = $state('');
	let foreign = $state<ForeignInstance[]>([]);
	const picked = new SvelteSet<string>();
	let importMode = $state<ImportMode>('copy');
	let foreignResult = $state<ForeignImportResult | null>(null);
	let scanned = false;

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
			case 'launcher':
				return {
					label: importMode === 'link' ? '建立連結' : '複製匯入',
					icon: importMode === 'link' ? 'shortcut' : 'copy',
					ready: picked.size > 0,
					run: importForeign
				};
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
		if (id === 'launcher' && !scanned) {
			scanned = true;
			scanLaunchers();
		}
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

	async function scanLaunchers() {
		busy = '正在尋找其他啟動器…';
		try {
			launchers = await api.detectForeignLaunchers();
			const first = launchers[0];
			if (first) await selectLauncher(first.path);
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			busy = '';
		}
	}

	async function selectLauncher(path: string) {
		launcherPath = path;
		picked.clear();
		foreign = [];
		if (!path.trim()) return;

		error = '';
		busy = '正在讀取實例清單…';
		try {
			foreign = await api.listForeignInstances(path.trim());
			if (foreign.length === 0) error = '這個啟動器裡沒有實例。';
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			busy = '';
		}
	}

	async function pickLauncherDir() {
		try {
			const { open } = await import('@tauri-apps/api/dialog');
			const dir = await open({ directory: true });
			if (typeof dir === 'string') await selectLauncher(dir);
		} catch {
			error = '這個環境沒有系統對話框，請直接貼上資料夾路徑';
		}
	}

	function toggle(path: string) {
		if (!picked.delete(path)) picked.add(path);
	}

	function toggleAll() {
		const all = picked.size === foreign.length;
		picked.clear();
		if (!all) for (const item of foreign) picked.add(item.path);
	}

	async function importForeign() {
		error = '';
		busy = importMode === 'link' ? '正在建立連結…' : '正在複製…';

		let unlisten: (() => void) | undefined;
		try {
			unlisten = await api.onInstallProgress((p) => {
				busy = p.total > 1 ? `${p.message}（${p.done} / ${p.total}）` : p.message;
			});

			foreignResult = await api.importForeignInstances({
				baseDir,
				paths: foreign.filter((i) => picked.has(i.path)).map((i) => i.path),
				mode: importMode,
				group
			});
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			unlisten?.();
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
		<Menu width="228px">
			{#snippet trigger(attrs)}
				<Button {...attrs} class="size-14.5 rounded-lg p-0" aria-label="選擇圖示">
					<InstanceIcon name={icon} size={34} />
				</Button>
			{/snippet}
			<div class="grid grid-cols-5 gap-0.75 p-0.75">
				{#each ICON_CHOICES as choice (choice)}
					<button
						class="grid place-items-center rounded-sm border border-transparent p-1.25 hover:bg-hover {icon ===
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

		<div class="min-w-0 flex-1 [--field-label:48px]">
			<Field label="名稱" for="inst-name">
				<TextInput id="inst-name" bind:value={name} {placeholder} />
			</Field>
			<Field label="群組" for="inst-group">
				<TextInput id="inst-group" bind:value={group} list="group-suggestions" placeholder="（未分組）" />
				<datalist id="group-suggestions">
					{#each groups as g (g)}
						<option value={g}></option>
					{/each}
				</datalist>
			</Field>
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
		{#if !result && !foreignResult}
			<NavList {pages} selected={page} onselect={selectPage} width="162px" />
		{/if}

		<div class="flex min-w-0 flex-1 flex-col gap-3.5 overflow-y-auto px-4.5 py-4">
			{#if foreignResult}
				{@const done = foreignResult}
				<h2 class="text-[15px] font-semibold text-ink">
					{done.imported.length > 0 ? `已匯入 ${done.imported.length} 個實例` : '沒有匯入任何實例'}
				</h2>

				{#if done.imported.length}
					<ul class="rounded-lg border border-line bg-deep">
						{#each done.imported as item (item.id)}
							<li class="flex items-center gap-3 border-b border-line px-3 py-2 last:border-b-0">
								<InstanceIcon name={item.icon} size={22} />
								<span class="min-w-0 flex-1 truncate text-ink">{item.name}</span>
								<span class="text-faint">{item.versionId || '未知版本'}</span>
							</li>
						{/each}
					</ul>
					<p class="leading-[1.8] text-muted">
						Minecraft 本體與模組載入器不會跟著搬過來，第一次啟動時才下載。
					</p>
				{/if}

				{#if done.failed.length}
					<p class="leading-[1.8] text-danger">這些實例沒有匯入成功：</p>
					<ul class="rounded-lg border border-line bg-deep">
						{#each done.failed as item (item.path)}
							<li class="border-b border-line px-3 py-2 last:border-b-0">
								<div class="truncate text-ink">{item.name}</div>
								<div class="text-faint">{item.message}</div>
							</li>
						{/each}
					</ul>
				{/if}
			{:else if result}
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
									<Button onclick={() => attempt(api.openExternal(website))}>
										<Icon name="globe" size={13} />下載頁面
									</Button>
								{/if}
								<Button onclick={() => attempt(api.openInstanceFolder(baseDir, done.instance.id, file.folder))}>
									<Icon name="folder" size={13} />資料夾
								</Button>
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
				<Field label="模組包" for="pack-path">
					<div class="flex gap-1.75">
						<TextInput class="min-w-0 flex-1" id="pack-path" bind:value={packPath} oninput={() => (filePreview = null)} onchange={inspectPack} placeholder="Modrinth 的 .mrpack，或 CurseForge 的 .zip" />
						<Button disabled={!!busy} onclick={pickPack}>瀏覽…</Button>
					</div>
				</Field>

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
			{:else if page === 'launcher'}
				<div class="flex min-h-0 flex-1 flex-col gap-3">
					<Field label="來源" for="foreign-src">
						<div class="flex gap-1.75">
							<Select class="min-w-0 flex-1" id="foreign-src" disabled={!!busy} value={launcherPath} onchange={(e) => selectLauncher(e.currentTarget.value)}>
								{#if launchers.length === 0 && !launcherPath}
									<option value="">（沒有偵測到 Prism Launcher / PolyMC / MultiMC）</option>
								{/if}
								{#each launchers as item (item.path)}
									<option value={item.path}>
										{item.name}（{item.instanceCount} 個實例）— {item.path}
									</option>
								{/each}
								{#if launcherPath && !launchers.some((l) => l.path === launcherPath)}
									<option value={launcherPath}>{launcherPath}</option>
								{/if}
							</Select>
							<Button disabled={!!busy} onclick={pickLauncherDir}>瀏覽…</Button>
						</div>
					</Field>

					<TableBox>
						<Table>
							<thead>
								<tr>
									<th style="width:34px">
										<input
											type="checkbox"
											aria-label="全選"
											disabled={foreign.length === 0}
											checked={foreign.length > 0 && picked.size === foreign.length}
											onchange={toggleAll}
										/>
									</th>
									<th>名稱</th>
									<th style="width:92px">版本</th>
									<th style="width:128px">模組載入器</th>
									<th style="width:64px">模組</th>
								</tr>
							</thead>
							<tbody>
								{#each foreign as item (item.path)}
									<tr>
										<td>
											<input
												type="checkbox"
												aria-label={item.name}
												checked={picked.has(item.path)}
												onchange={() => toggle(item.path)}
											/>
										</td>
										<td class="truncate">{item.name}</td>
										<td class="text-muted">{item.mcVersion || '—'}</td>
										<td class="text-muted">
											{#if item.unsupportedLoader}
												<span class="text-danger">{item.unsupportedLoader}（不支援）</span>
											{:else if item.loader === 'vanilla'}
												無
											{:else}
												{loaderLabel(item.loader)}
											{/if}
										</td>
										<td class="text-muted">{item.modCount}</td>
									</tr>
								{/each}
							</tbody>
						</Table>
						{#if foreign.length === 0}
							<p class="px-3 py-4 text-center text-muted">
								選一個啟動器的資料目錄，或它的 instances 資料夾。
							</p>
						{/if}
					</TableBox>

					<Field label="群組" for="foreign-group">
						<TextInput id="foreign-group" bind:value={group} list="group-suggestions" placeholder="（未分組）" />
						<datalist id="group-suggestions">
							{#each groups as g (g)}
								<option value={g}></option>
							{/each}
						</datalist>
					</Field>

					<div class="flex shrink-0 flex-col">
						<CheckRow class="items-start">
							<input type="radio" class="mt-[3px]" value="copy" bind:group={importMode} />
							<span class="min-w-0">
								<span class="text-ink">複製一份</span>
								<span class="block leading-[1.7] text-faint">
									把遊戲目錄整份複製過來，兩邊各玩各的，但硬碟會多放一份。
								</span>
							</span>
						</CheckRow>
						<CheckRow class="items-start">
							<input type="radio" class="mt-[3px]" value="link" bind:group={importMode} />
							<span class="min-w-0">
								<span class="text-ink">建立連結</span>
								<span class="block leading-[1.7] text-faint">
									直接指向原本的遊戲目錄，存檔與模組兩邊共用，不佔額外空間。不要兩個啟動器同時開同一個實例。
								</span>
							</span>
						</CheckRow>
					</div>
				</div>
			{:else}
				<div class="flex min-h-0 flex-1 flex-col gap-3">
					<Field label="壓縮檔" for="zip-path">
						<div class="flex gap-1.75">
							<TextInput class="min-w-0 flex-1" id="zip-path" bind:value={zipPath} placeholder="選擇由「匯出實例」產生的 .zip" />
							<Button onclick={pickZip}>瀏覽…</Button>
						</div>
					</Field>

					<p class="px-0.5 leading-[1.8] text-faint">
						也可以直接把整個實例資料夾複製到實例目錄底下。
					</p>

					<div class="flex gap-1.75">
						<Button onclick={() => api.openLauncherFolder('instances')}>
							開啟實例資料夾
						</Button>
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
		{#if foreignResult}
			{@const first = foreignResult.imported[0]}
			<Button variant="primary" onclick={() => (first ? oncreated(first) : onclose())}>
				<Icon name="check" size={13} />完成
			</Button>
		{:else if result}
			{@const done = result}
			<Button variant="primary" onclick={() => oncreated(done.instance)}>
				<Icon name="check" size={13} />完成
			</Button>
		{:else}
			<Button variant="primary" disabled={!action.ready || !!busy} onclick={action.run}>
				<Icon name={action.icon} size={13} />{action.label}
			</Button>
			<Button disabled={!!busy} onclick={onclose}>取消</Button>
		{/if}
	{/snippet}
</WindowShell>
