<script lang="ts">
	import Icon from '$lib/Icon.svelte';
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import WindowShell from './WindowShell.svelte';
	import PageList from './PageList.svelte';
	import Toggle from './Toggle.svelte';
	import Window from './Window.svelte';
	import VersionPicker from './VersionPicker.svelte';
	import ContentBrowser from './ContentBrowser.svelte';
	import * as api from '$lib/api';
	import type {
		ContentKind,
		FileEntry,
		GameLogLine,
		InstanceSettings,
		InstanceView,
		JavaInfo,
		LoaderKind,
		ModEntry,
		ModUpdateEntry,
		PackEntry,
		RunningGame,
		ServerEntry,
		WorldEntry
	} from '$lib/api';
	import { ICON_CHOICES } from '$lib/icons';
	import { formatDuration, formatSize, formatTime, loaderLabel } from '$lib/format';
	import { onMount } from 'svelte';
	import type { Page } from '$lib/ui';

	let {
		baseDir,
		instance,
		onclose,
		onchanged
	}: {
		baseDir: string;
		instance: InstanceView;
		onclose: () => void;
		/** 存檔或重新安裝之後回報新的內容 */
		onchanged: (instance: InstanceView) => void;
	} = $props();

	const pages: Page[] = [
		{ id: 'general', label: '一般', icon: 'box' },
		{ id: 'console', label: '遊戲日誌', icon: 'terminal' },
		{ id: 'version', label: '版本', icon: 'package' },
		{ id: 'mods', label: '模組', icon: 'package' },
		{ id: 'resourcepacks', label: '資源包', icon: 'image' },
		{ id: 'shaderpacks', label: '光影包', icon: 'sun' },
		{ id: 'worlds', label: '世界', icon: 'globe' },
		{ id: 'servers', label: '伺服器', icon: 'server' },
		{ id: 'screenshots', label: '螢幕截圖', icon: 'image' },
		{ id: 'notes', label: '記事', icon: 'notes' },
		{ id: 'logs', label: '日誌檔', icon: 'list' },
		{ id: 'settings', label: '設定', icon: 'settings' }
	];

	const BROWSE_TITLES: Record<ContentKind, string> = {
		mod: '下載模組',
		resourcepack: '下載資源包',
		shader: '下載光影包'
	};

	const BROWSE_PAGES: Record<ContentKind, string> = {
		mod: 'mods',
		resourcepack: 'resourcepacks',
		shader: 'shaderpacks'
	};

	let page = $state('general');
	let status = $state('');
	let busy = $state('');


	// svelte-ignore state_referenced_locally
	let form = $state({
		name: instance.name,
		icon: instance.icon,
		group: instance.group,
		notes: instance.notes,
		javaPath: instance.javaPath,
		settings: { ...instance.settings } as InstanceSettings
	});

	let saved = $state(snapshot());
	const dirty = $derived(snapshot() !== saved);
	let iconMenu = $state(false);

	function snapshot() {
		return JSON.stringify({
			name: form.name,
			icon: form.icon,
			group: form.group,
			notes: form.notes,
			javaPath: form.javaPath,
			settings: form.settings
		});
	}


	let mods = $state<ModEntry[]>([]);
	let modUpdates = $state<ModUpdateEntry[]>([]);
	let resourcePacks = $state<PackEntry[]>([]);
	let shaderPacks = $state<PackEntry[]>([]);
	let worlds = $state<WorldEntry[]>([]);
	let servers = $state<ServerEntry[]>([]);
	let screenshots = $state<FileEntry[]>([]);
	let logFiles = $state<FileEntry[]>([]);
	let logFile = $state('');
	let logText = $state('');
	let javaCandidates = $state<JavaInfo[]>([]);

	const loaded: string[] = [];


	let output = $state<GameLogLine[]>([]);
	let running = $state<RunningGame | null>(null);
	let outputEl = $state<HTMLDivElement | null>(null);
	let autoScroll = $state(true);
	let onlyErrors = $state(false);

	let selectedMod = $state<string | null>(null);
	let wrapLines = $state(true);
	let colorLog = $state(true);
	let logSearch = $state('');

	let versionDialog = $state<{
		mcVersion: string;
		loader: LoaderKind;
		loaderVersion: string;
	} | null>(null);


	let browse = $state<ContentKind | null>(null);

	const title = $derived(pages.find((p) => p.id === page)?.label ?? '');

	function report(message: string) {
		status = message;
		setTimeout(() => {
			if (status === message) status = '';
		}, 5000);
	}

	function fail(prefix: string, e: unknown) {
		status = `${prefix}：${api.errorMessage(e)}`;
	}

	async function load(target: string, force = false) {
		if (loaded.includes(target) && !force) return;
		loaded.push(target);
		try {
			switch (target) {
				case 'mods':
					mods = await api.listMods(baseDir, instance.id);
					break;
				case 'resourcepacks':
					resourcePacks = await api.listResourcePacks(baseDir, instance.id);
					break;
				case 'shaderpacks':
					shaderPacks = await api.listShaderPacks(baseDir, instance.id);
					break;
				case 'worlds':
					worlds = await api.listWorlds(baseDir, instance.id);
					break;
				case 'servers':
					servers = await api.listServers(baseDir, instance.id);
					break;
				case 'screenshots':
					screenshots = await api.listScreenshots(baseDir, instance.id);
					break;
				case 'logs':
					logFiles = await api.listInstanceLogs(baseDir, instance.id);
					logFile = logFiles.find((f) => f.name === 'latest.log')?.name ?? logFiles[0]?.name ?? '';
					await openLog(logFile);
					break;
				case 'settings':
					javaCandidates = await api.detectJavaCandidates();
					break;
			}
		} catch (e) {
			loaded.splice(loaded.indexOf(target), 1);
			fail('讀取失敗', e);
		}
	}

	$effect(() => {
		load(page);
	});

	onMount(() => {
		const unlisteners: Array<() => void> = [];
		const track = (p: Promise<() => void>) => p.then((un) => unlisteners.push(un));


		track(
			api.onGameOutput((out) => {
				if (out.instanceName !== instance.id) return;
				appendOutput([out]);
			})
		);

		track(
			api.onGameStart((info) => {
				if (info.instanceName !== instance.id) return;
				output = [];
				running = info;
				loadOutput();
			})
		);
		track(api.onGameExit(() => refreshRunning()));

		loadOutput();
		refreshRunning();

		return () => unlisteners.forEach((un) => un());
	});

	function appendOutput(lines: GameLogLine[]) {
		const seen: Record<number, true> = {};
		for (const line of output) seen[line.seq] = true;

		const merged = [...output, ...lines.filter((line) => !seen[line.seq])];
		merged.sort((a, b) => a.seq - b.seq);

		const keep = 5000;
		output = merged.length > keep ? merged.slice(-keep) : merged;

		if (autoScroll && outputEl) {
			queueMicrotask(() => outputEl?.scrollTo(0, outputEl.scrollHeight));
		}
	}

	async function loadOutput() {
		try {
			appendOutput(await api.gameLog(instance.id));
		} catch (e) {
			fail('讀取遊戲輸出失敗', e);
		}
	}

	async function refreshRunning() {
		try {
			const list = await api.runningInstances();
			running = list.find((r) => r.instanceName === instance.id) ?? null;
		} catch (e) {
			fail('讀取執行狀態失敗', e);
		}
	}

	async function stopGame() {
		try {
			await api.stopInstance(instance.id);
			report('已要求遊戲結束');
		} catch (e) {
			fail('終止失敗', e);
		}
	}

	function copyOutput() {
		navigator.clipboard?.writeText(output.map((line) => line.line).join('\n'));
	}

	async function clearOutput() {
		try {
			await api.clearGameLog(instance.id);
			output = [];
		} catch (e) {
			fail('清空失敗', e);
		}
	}

	const visibleOutput = $derived(
		onlyErrors ? output.filter((line) => line.kind === 'stderr') : output
	);

	async function openLog(name: string) {
		logFile = name;
		logText = '';
		if (!name) return;
		try {
			logText = await api.readInstanceLog(baseDir, instance.id, name);
		} catch (e) {
			fail('讀取日誌失敗', e);
		}
	}

	const logLines = $derived(
		logText.split('\n').map((text) => ({
			text,
			level:
				text.includes('/ERROR]') || text.includes('/FATAL]')
					? 'error'
					: text.includes('/WARN]')
						? 'warn'
						: 'info'
		}))
	);

	const visibleLogLines = $derived.by(() => {
		const q = logSearch.trim().toLowerCase();
		if (!q) return logLines;
		return logLines.filter((line) => line.text.toLowerCase().includes(q));
	});

	async function save() {
		try {
			const updated = await api.saveInstanceMeta(baseDir, instance.id, {
				name: form.name,
				icon: form.icon,
				group: form.group,
				notes: form.notes,
				javaPath: form.javaPath,

				settings: $state.snapshot(form.settings)
			});
			saved = snapshot();
			onchanged(updated);
			report('已儲存');
		} catch (e) {
			fail('儲存失敗', e);
		}
	}

	async function saveAndClose() {
		if (dirty) await save();
		onclose();
	}

	async function openFolder(target: api.InstanceFolder) {
		try {
			await api.openInstanceFolder(baseDir, instance.id, target);
		} catch (e) {
			fail('開啟資料夾失敗', e);
		}
	}


	async function toggleMod(mod: ModEntry) {
		try {
			await api.setModEnabled(baseDir, instance.id, mod.fileName, !mod.enabled);
			await load('mods', true);
		} catch (e) {
			fail('切換模組失敗', e);
		}
	}

	async function removeMod(fileName: string) {
		try {
			await api.deleteMod(baseDir, instance.id, fileName);
			await load('mods', true);
			report(`已移除 ${fileName}`);
		} catch (e) {
			fail('移除模組失敗', e);
		}
	}

	async function checkUpdates() {
		busy = '正在比對 Modrinth / CurseForge…';
		try {
			modUpdates = await api.checkModUpdates({ baseDir, instanceName: instance.id });
			report(modUpdates.length ? `有 ${modUpdates.length} 個模組可以更新` : '所有模組都是最新的');
		} catch (e) {
			fail('檢查更新失敗', e);
		} finally {
			busy = '';
		}
	}

	async function applyUpdate(update: ModUpdateEntry) {
		busy = `正在更新 ${update.name}…`;
		try {
			await api.applyModUpdate(baseDir, instance.id, update.id);
			modUpdates = modUpdates.filter((u) => u.id !== update.id);
			await load('mods', true);
			report(`${update.name} 已更新至 ${update.newVersion}`);
		} catch (e) {
			fail('更新失敗', e);
		} finally {
			busy = '';
		}
	}


	async function reinstall(request?: { mcVersion: string; loader: LoaderKind; loaderVersion: string }) {
		versionDialog = null;
		busy = request ? '正在安裝新版本…' : '正在修復遊戲檔案…';

		let unlisten: (() => void) | undefined;
		try {
			unlisten = await api.onInstallProgress((p) => {
				if (p.instanceName === instance.id) {
					busy = p.total > 1 ? `${p.message}（${p.done} / ${p.total}）` : p.message;
				}
			});

			const updated = request
				? await api.installInstance({
						baseDir,
						instanceName: instance.id,
						mcVersion: request.mcVersion,
						loader: request.loader,
						loaderVersion: request.loader === 'vanilla' ? null : request.loaderVersion || null
					})
				: await api.repairInstance(baseDir, instance.id);

			onchanged(updated);
			report('已完成');
		} catch (e) {
			fail('安裝失敗', e);
		} finally {
			unlisten?.();
			busy = '';
		}
	}

	const versionRows = $derived([
		{ name: 'Minecraft', version: instance.versionId || '未知', note: '必要' },
		...(instance.loader !== 'vanilla'
			? [
					{
						name: `${loaderLabel(instance.loader)} Loader`,
						version: instance.loaderVersion || '（未記錄）',
						note: '模組載入器'
					}
				]
			: []),
		{ name: 'Java', version: `需要 ${instance.javaVersion} 以上`, note: '執行環境' },
		{ name: '資產索引', version: instance.assetsId || '—', note: '共用資源' },
		{ name: '主類別', version: instance.mainClass || '—', note: '啟動進入點' }
	]);
</script>

{#snippet sideButtons(items: { label: string; disabled?: boolean; action: () => void }[])}
	<div class="flex w-34.5 shrink-0 flex-col gap-1 [&>button]:justify-start">
		{#each items as item (item.label)}
			<button class="qt-button" disabled={item.disabled} onclick={item.action}>
				{item.label}
			</button>
		{/each}
	</div>
{/snippet}

<WindowShell>
	<div class="flex min-h-0 flex-1">
		<PageList {pages} selected={page} onselect={(id) => (page = id)} />

		<div class="flex min-w-0 flex-1 flex-col">
			<div
				class="flex shrink-0 items-center gap-2.75 border-b border-line px-4.5 pb-3.25 pt-3.75 text-brand"
			>
				<Icon name={pages.find((p) => p.id === page)?.icon ?? 'box'} size={19} />
				<h3>{title}</h3>
				{#if running}
					<span class="ml-auto text-[12px] text-brand">執行中 · PID {running.pid}</span>
				{:else if !instance.healthy}
					<span class="ml-auto text-[12px] text-warn">遊戲檔案不完整，請到「版本」修復</span>
				{/if}
			</div>

			<div class="flex min-h-0 flex-1 flex-col gap-2.5 px-4.5 pb-4.5 pt-3.5">
				{#if page === 'general'}
					<div class="flex items-start gap-3">
						<div class="relative">
							<button
								class="qt-button size-14.5 rounded-lg p-0"
								aria-label="選擇圖示"
								onclick={() => (iconMenu = !iconMenu)}
							>
								<InstanceIcon name={form.icon} size={34} />
							</button>
							{#if iconMenu}
								<button
									type="button"
									class="qt-menu-scrim"
									aria-label="關閉選單"
									onclick={() => (iconMenu = false)}
								></button>
								<div class="qt-menu" style:min-width="228px">
									<div class="grid grid-cols-5 gap-0.75 p-0.75">
										{#each ICON_CHOICES as choice (choice)}
											<button
												class="grid place-items-center rounded-sm border border-transparent p-1.25 hover:bg-white/10 {form.icon ===
												choice
													? 'border-brand bg-brand-soft'
													: ''}"
												aria-label={choice}
												onclick={() => {
													form.icon = choice;
													iconMenu = false;
												}}
											>
												<InstanceIcon name={choice} size={30} />
											</button>
										{/each}
									</div>
								</div>
							{/if}
						</div>

						<div class="min-w-0 flex-1 [&_.qt-field]:grid-cols-[64px_1fr]">
							<div class="qt-field">
								<label for="edit-name">名稱</label>
								<input
									id="edit-name"
									class="qt-input"
									bind:value={form.name}
								/>
							</div>
							<div class="qt-field">
								<label for="edit-group">群組</label>
								<input
									id="edit-group"
									class="qt-input"
									bind:value={form.group}
									placeholder="（未分組）"
								/>
							</div>
						</div>
					</div>

					<div class="qt-group">
						<span class="qt-group-title">狀態</span>
						<div class="qt-field"><label for="s1">資料夾名稱</label><span id="s1">{instance.id}</span></div>
						<div class="qt-field">
							<label for="s2">位置</label>
							<span id="s2" class="overflow-hidden text-ellipsis font-mono text-[11.5px] text-muted"
								>{instance.dir}</span
							>
						</div>
						<div class="qt-field">
							<label for="s3">上次遊玩</label><span id="s3">{formatTime(instance.lastPlayed)}</span>
						</div>
						<div class="qt-field">
							<label for="s4">總時數</label>
							<span id="s4">
								{formatDuration(instance.playTimeSeconds)}（啟動 {instance.launchCount} 次）
							</span>
						</div>
						<div class="qt-field">
							<label for="s5">模組</label><span id="s5">{instance.modCount} 個</span>
						</div>
					</div>

				{:else if page === 'console'}
					<div class="flex shrink-0 items-center gap-2.5">
						{#if running}
							<span class="pill accent">執行中 · PID {running.pid}</span>
							<span class="text-muted">
								{running.online ? '正版登入' : '離線模式'} · {running.playerName} ·
								{formatTime(running.startedAt)} 起
							</span>
						{:else}
							<span class="pill">未執行</span>
						{/if}
						<span class="flex-1"></span>
						<button class="qt-button destructive" disabled={!running} onclick={stopGame}>
							終止遊戲
						</button>
					</div>

					<div
						bind:this={outputEl}
						class="min-h-0 flex-1 overflow-auto rounded-lg border border-line bg-deep px-3.5 py-3
							font-mono text-[11.5px] leading-[1.6] text-[#cfd6da]"
					>
						{#each visibleOutput as line (line.seq)}
							<div
								class={wrapLines ? 'whitespace-pre-wrap' : 'whitespace-pre'}
								class:text-danger={colorLog && line.kind === 'stderr'}
							>{line.line}</div>
						{:else}
							<p class="py-6 text-center font-sans text-faint">目前沒有輸出。</p>
						{/each}
					</div>

					<div class="flex shrink-0 items-center gap-4.5">
						<label class="qt-check compact"
							><input type="checkbox" bind:checked={autoScroll} />自動捲動</label
						>
						<label class="qt-check compact"
							><input type="checkbox" bind:checked={onlyErrors} />只看 stderr</label
						>
						<label class="qt-check compact"
							><input type="checkbox" bind:checked={wrapLines} />自動換行</label
						>
						<span class="flex-1"></span>
						<span class="text-faint">{visibleOutput.length} / {output.length} 行</span>
						<button
							class="qt-button"
							disabled={output.length === 0}
							onclick={copyOutput}
						>
							複製
						</button>
						<button class="qt-button" disabled={!!running || output.length === 0} onclick={clearOutput}>
							清空
						</button>
					</div>

				{:else if page === 'version'}
					<div class="flex min-h-0 flex-1 gap-2.25">
						<div class="qt-table-wrap">
							<table class="qt-table">
								<thead>
									<tr><th style="width:34%">名稱</th><th>版本</th><th style="width:110px">說明</th></tr>
								</thead>
								<tbody>
									{#each versionRows as row (row.name)}
										<tr>
											<td>{row.name}</td>
											<td class="font-mono text-[11.5px]">{row.version}</td>
											<td class="text-muted">{row.note}</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
						{@render sideButtons([
							{
								label: '變更版本…',
								action: () =>
									(versionDialog = {
										mcVersion: instance.versionId,
										loader: instance.loader,
										loaderVersion: instance.loaderVersion
									})
							},
							{ label: '修復／重新安裝', action: () => reinstall() },
							{ label: '實例資料夾', action: () => openFolder('instance') }
						])}
					</div>
				{:else if page === 'mods'}
					{#if modUpdates.length > 0}
						<div class="shrink-0 rounded-lg border border-line bg-deep p-2.5">
							<p class="mb-2 font-medium">可以更新的模組</p>
							<div class="flex flex-col gap-1">
								{#each modUpdates as update (update.id)}
									<div class="flex items-center gap-2.5">
										<span class="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap">
											{update.name}
											<span class="text-faint">
												{update.currentVersion} → {update.newVersion}（{update.source}）
											</span>
										</span>
										<button class="qt-button" onclick={() => applyUpdate(update)}>更新</button>
									</div>
								{/each}
							</div>
						</div>
					{/if}

					<div class="flex min-h-0 flex-1 gap-2.25">
						<div class="qt-table-wrap">
							<table class="qt-table">
								<thead>
									<tr>
										<th style="width:28px"></th>
										<th style="width:34%">名稱</th>
										<th>版本</th>
										<th>檔案</th>
										<th style="width:80px">載入器</th>
									</tr>
								</thead>
								<tbody>
									{#each mods as mod (mod.fileName)}
										<tr
											class:selected={selectedMod === mod.fileName}
											onclick={() => (selectedMod = mod.fileName)}
										>
											<td>
												<input
													type="checkbox"
													checked={mod.enabled}
													aria-label={`啟用 ${mod.name}`}
													onclick={(e) => {
														e.stopPropagation();
														toggleMod(mod);
													}}
												/>
											</td>
											<td class:line-through={!mod.enabled} class:text-faint={!mod.enabled}
												>{mod.name}</td
											>
											<td class="text-muted">{mod.version || '—'}</td>
											<td class="text-muted">{mod.fileName}</td>
											<td class="text-muted">{mod.loader}</td>
										</tr>
									{/each}
								</tbody>
							</table>
							{#if mods.length === 0}
								<p class="px-3 py-4 text-center text-muted">這個實例還沒有任何模組。</p>
							{/if}
						</div>
						{@render sideButtons([
							{ label: '下載模組…', action: () => (browse = 'mod') },
							{ label: '檢查更新', disabled: mods.length === 0, action: checkUpdates },
							{
								label: '移除',
								disabled: !selectedMod,
								action: () => selectedMod && removeMod(selectedMod)
							},
							{ label: '重新載入', action: () => load('mods', true) },
							{ label: '模組資料夾', action: () => openFolder('mods') }
						])}
					</div>

				{:else if page === 'resourcepacks' || page === 'shaderpacks'}
					{@const packs = page === 'resourcepacks' ? resourcePacks : shaderPacks}
					<div class="flex min-h-0 flex-1 gap-2.25">
						<div class="qt-table-wrap">
							<table class="qt-table">
								<thead>
									<tr><th style="width:45%">名稱</th><th>類型</th><th>位置</th></tr>
								</thead>
								<tbody>
									{#each packs as pack (pack.path)}
										<tr>
											<td>{pack.name}</td>
											<td class="text-muted">{pack.isFolder ? '資料夾' : '壓縮檔'}</td>
											<td class="overflow-hidden text-ellipsis font-mono text-[11px] text-muted"
												>{pack.path}</td
											>
										</tr>
									{/each}
								</tbody>
							</table>
							{#if packs.length === 0}
								<p class="px-3 py-4 text-center text-muted">
									{page === 'resourcepacks' ? '沒有資源包。' : '沒有光影包。'}
								</p>
							{/if}
						</div>
						{@render sideButtons([
							{
								label: page === 'resourcepacks' ? '下載資源包…' : '下載光影包…',
								action: () => (browse = page === 'resourcepacks' ? 'resourcepack' : 'shader')
							},
							{ label: '重新載入', action: () => load(page, true) },
							{
								label: '開啟資料夾',
								action: () => openFolder(page === 'resourcepacks' ? 'resourcepacks' : 'shaderpacks')
							}
						])}
					</div>

				{:else if page === 'worlds'}
					<div class="flex min-h-0 flex-1 gap-2.25">
						<div class="qt-table-wrap">
							<table class="qt-table">
								<thead>
									<tr><th style="width:44px"></th><th style="width:45%">名稱</th><th>資料夾</th></tr>
								</thead>
								<tbody>
									{#each worlds as world (world.path)}
										<tr>
											<td>
												{#if world.icon}
													<img src={world.icon} alt="" class="size-6 rounded-sm" />
												{/if}
											</td>
											<td>{world.name}</td>
											<td class="overflow-hidden text-ellipsis font-mono text-[11px] text-muted"
												>{world.path}</td
											>
										</tr>
									{/each}
								</tbody>
							</table>
							{#if worlds.length === 0}
								<p class="px-3 py-4 text-center text-muted">還沒有任何世界存檔。</p>
							{/if}
						</div>
						{@render sideButtons([
							{ label: '重新載入', action: () => load('worlds', true) },
							{ label: '開啟資料夾', action: () => openFolder('saves') }
						])}
					</div>

				{:else if page === 'servers'}
					<div class="flex min-h-0 flex-1 gap-2.25">
						<div class="qt-table-wrap">
							<table class="qt-table">
								<thead>
									<tr>
										<th style="width:44px"></th>
										<th style="width:40%">名稱</th>
										<th>位址</th>
										<th style="width:90px">材質包</th>
									</tr>
								</thead>
								<tbody>
									{#each servers as server (server.address + server.name)}
										<tr>
											<td>
												{#if server.icon}
													<img src={server.icon} alt="" class="size-6 rounded-sm" />
												{/if}
											</td>
											<td>{server.name}</td>
											<td class="font-mono text-[11.5px] text-muted">{server.address}</td>
											<td class="text-muted">{server.acceptTextures ? '接受' : '詢問'}</td>
										</tr>
									{/each}
								</tbody>
							</table>
							{#if servers.length === 0}
								<p class="px-3 py-4 text-center text-muted">
									還沒有加過伺服器。
								</p>
							{/if}
						</div>
						{@render sideButtons([
							{ label: '重新載入', action: () => load('servers', true) },
							{ label: '開啟資料夾', action: () => openFolder('minecraft') }
						])}
					</div>
				{:else if page === 'screenshots'}
					<div class="flex min-h-0 flex-1 gap-2.25">
						<div class="qt-table-wrap">
							<table class="qt-table">
								<thead>
									<tr><th style="width:55%">檔名</th><th>大小</th><th>拍攝時間</th></tr>
								</thead>
								<tbody>
									{#each screenshots as shot (shot.path)}
										<tr>
											<td>{shot.name}</td>
											<td class="text-muted">{formatSize(shot.size)}</td>
											<td class="text-muted">{formatTime(shot.modified)}</td>
										</tr>
									{/each}
								</tbody>
							</table>
							{#if screenshots.length === 0}
								<p class="px-3 py-4 text-center text-muted">還沒有螢幕截圖。</p>
							{/if}
						</div>
						{@render sideButtons([
							{ label: '重新載入', action: () => load('screenshots', true) },
							{ label: '開啟資料夾', action: () => openFolder('screenshots') }
						])}
					</div>

				{:else if page === 'notes'}
					<textarea
						class="qt-textarea min-h-0 flex-1 p-2.5"
						bind:value={form.notes}
						placeholder="為這個實例寫點筆記…"
						aria-label="實例記事"
					></textarea>
				{:else if page === 'logs'}
					<div class="flex shrink-0 items-center gap-2.5">
						<select
							class="qt-select"
							value={logFile}
							onchange={(e) => openLog(e.currentTarget.value)}
							aria-label="日誌檔案"
						>
							{#each logFiles as file (file.name)}
								<option value={file.name}>{file.name}（{formatSize(file.size)}）</option>
							{:else}
								<option value="">沒有日誌檔</option>
							{/each}
						</select>
						<label class="qt-search bg-deep">
							<Icon name="search" size={14} />
							<input placeholder="搜尋日誌…" bind:value={logSearch} aria-label="搜尋日誌" />
							{#if logSearch}
								<span class="whitespace-nowrap text-faint">{visibleLogLines.length} 行</span>
							{/if}
						</label>
						<span class="flex-1"></span>
						<button class="qt-button" onclick={() => load('logs', true)}>重新載入</button>
						<button class="qt-button" onclick={() => openFolder('logs')}>資料夾</button>
					</div>

					<div
						class="min-h-0 flex-1 overflow-auto rounded-lg border border-line bg-deep px-3.5 py-3
							font-mono text-[11.5px] leading-[1.6] text-[#cfd6da]"
					>
						{#each visibleLogLines as line, i (i)}
							<div
								class={wrapLines ? 'whitespace-pre-wrap' : 'whitespace-pre'}
								class:text-warn={colorLog && line.level === 'warn'}
								class:text-danger={colorLog && line.level === 'error'}
							>{line.text}</div>
						{:else}
							<p class="py-6 text-center font-sans text-faint">
								{logFile ? '這個檔案是空的。' : '這個實例還沒有日誌。'}
							</p>
						{/each}
					</div>

					<div class="flex shrink-0 items-center gap-4.5">
						<label class="qt-check compact"
							><input type="checkbox" bind:checked={wrapLines} />自動換行</label
						>
						<label class="qt-check compact"
							><input type="checkbox" bind:checked={colorLog} />彩色輸出</label
						>
					</div>

				{:else}
					<div class="min-h-0 flex-1 overflow-y-auto pr-1">
						<div class="qt-group">
							<span class="qt-group-title">Java</span>
							<div class="qt-field">
								<label for="java-path">Java 執行檔</label>
								<div class="flex items-center gap-1.75">
									<select
										class="qt-select min-w-0 flex-1"
										bind:value={form.javaPath}
									>
										<option value="">跟隨啟動器設定</option>
										{#each javaCandidates as java (java.path)}
											<option value={java.path}>Java {java.version} — {java.path}</option>
										{/each}
										{#if form.javaPath && !javaCandidates.some((j) => j.path === form.javaPath)}
											<option value={form.javaPath}>{form.javaPath}</option>
										{/if}
									</select>
									<button class="qt-button" onclick={() => load('settings', true)}>重新偵測</button>
								</div>
							</div>
							<p class="px-0.5 py-1 text-faint">需要 Java {instance.javaVersion} 以上</p>
						</div>

						<div class="qt-group" class:checkable-off={!form.settings.overrideMemory}>
							<span class="qt-group-title">
								記憶體
								<span class="ml-auto text-[11.5px] font-normal text-muted">
									<Toggle
										label="覆寫"
										bind:checked={form.settings.overrideMemory}
									/>
								</span>
							</span>
							<div class="qt-field">
								<label for="mem-min">最小記憶體</label>
								<div class="flex items-center gap-1.75">
									<input
										id="mem-min"
										class="qt-input"
										type="number"
										style="width:100px"
										bind:value={form.settings.minMemoryMb}
									/>
									<span class="text-muted">MiB</span>
								</div>
							</div>
							<div class="qt-field">
								<label for="mem-max">最大記憶體</label>
								<div class="flex items-center gap-1.75">
									<input
										id="mem-max"
										class="qt-input"
										type="number"
										style="width:100px"
										bind:value={form.settings.maxMemoryMb}
									/>
									<span class="text-muted">MiB</span>
								</div>
							</div>
						</div>

						<div class="qt-group" class:checkable-off={!form.settings.overrideJvmArgs}>
							<span class="qt-group-title">
								JVM 參數
								<span class="ml-auto text-[11.5px] font-normal text-muted">
									<Toggle
										label="覆寫"
										bind:checked={form.settings.overrideJvmArgs}
									/>
								</span>
							</span>
							<div class="qt-field">
								<label for="jvm-args">參數</label>
								<input
									id="jvm-args"
									class="qt-input"
									bind:value={form.settings.jvmArgs}
									placeholder="-XX:+UseG1GC"
								/>
							</div>
						</div>

						<div class="qt-group" class:checkable-off={!form.settings.overrideCommands}>
							<span class="qt-group-title">
								自訂指令
								<span class="ml-auto text-[11.5px] font-normal text-muted">
									<Toggle
										label="覆寫"
										bind:checked={form.settings.overrideCommands}
									/>
								</span>
							</span>
							<div class="qt-field">
								<label for="pre">啟動前執行</label>
								<input
									id="pre"
									class="qt-input"
									bind:value={form.settings.preLaunch}
									placeholder="例如：backup.bat"
								/>
							</div>
							<div class="qt-field">
								<label for="post">結束後執行</label>
								<input
									id="post"
									class="qt-input"
									bind:value={form.settings.postExit}
								/>
							</div>
						</div>
					</div>
				{/if}
			</div>
		</div>
	</div>

	{#snippet footer()}
		{#if status}
			<span class="mr-auto pl-1 text-muted" role="status">{status}</span>
		{/if}
		<button class="qt-button" disabled={!dirty} onclick={save}>儲存</button>
		<button class="qt-button primary" onclick={saveAndClose}>關閉</button>
	{/snippet}
</WindowShell>

{#if versionDialog}
	<Window title="變更版本" width="720px" height="560px" onclose={() => (versionDialog = null)}>
		<div class="flex min-h-0 flex-1 flex-col gap-3 px-4.5 py-4">
			<p class="shrink-0 text-muted">世界與模組留在原地，但模組不一定相容新版本。</p>
			<VersionPicker
				bind:mcVersion={versionDialog.mcVersion}
				bind:loader={versionDialog.loader}
				bind:loaderVersion={versionDialog.loaderVersion}
				onError={(message) => (status = message)}
			/>
		</div>
		{#snippet footer()}
			<button
				class="qt-button primary"
				disabled={!versionDialog?.mcVersion}
				onclick={() => versionDialog && reinstall({ ...versionDialog })}
			>
				安裝
			</button>
			<button class="qt-button" onclick={() => (versionDialog = null)}>取消</button>
		{/snippet}
	</Window>
{/if}

{#if browse}
	{@const kind = browse}
	<Window title={BROWSE_TITLES[kind]} width="940px" height="660px" onclose={() => (browse = null)}>
		<div class="flex min-h-0 flex-1 flex-col px-4.5 py-4">
			<ContentBrowser
				{baseDir}
				{instance}
				{kind}
				oninstalled={(fileName) => {
					load(BROWSE_PAGES[kind], true);
					report(`已加入 ${fileName}`);
				}}
			/>
		</div>
	</Window>
{/if}

{#if busy}
	<div class="fixed inset-0 z-100 grid place-items-center bg-[#07090b99] backdrop-blur-[3px]">
		<div
			class="flex items-center gap-3 rounded-xl border border-line bg-window px-6 py-5 shadow-lg"
			role="status"
		>
			<span class="size-5 animate-spin rounded-full border-2 border-line border-t-brand"></span>
			{busy}
		</div>
	</div>
{/if}
