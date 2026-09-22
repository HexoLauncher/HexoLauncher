<script lang="ts">
	import { Button, ColorField, Field, Group, Slider, Table, TableBox, TextInput } from '$lib/ui';
	import NavList from '$lib/ui/NavList.svelte';
	import Switch from '$lib/ui/Switch.svelte';
	import Dialog from '$lib/ui/Dialog.svelte';
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import * as api from '$lib/api';
	import type { AccountView, DeviceCode, JavaCandidate, LauncherConfig, Paths } from '$lib/api';
	import type { Page } from '$lib/ui';
	import {
		MODES,
		PRESETS,
		applyTheme,
		backgroundUrl,
		defaultTheme,
		parseHex,
		toHex,
		type ThemeMode,
		type ThemePreset
	} from '$lib/theme';
	import Icon from '$lib/ui/Icon.svelte';

	let {
		config,
		accounts,
		paths,
		initialPage = 'launcher',
		save,
		onAccounts,
		report
	}: {

		config: LauncherConfig;
		accounts: AccountView[];
		paths: Paths;
		initialPage?: string;

		save: (immediate?: boolean) => void;
		onAccounts: (accounts: AccountView[]) => void;
		report: (message: string) => void;
	} = $props();

	const pages: Page[] = [
		{ id: 'launcher', label: '啟動器', icon: 'settings' },
		{ id: 'theme', label: '外觀', icon: 'palette' },
		{ id: 'minecraft', label: '遊戲與啟動', icon: 'box' },
		{ id: 'java', label: 'Java', icon: 'coffee' },
		{ id: 'commands', label: '自訂指令', icon: 'terminal' },
		{ id: 'accounts', label: '帳號', icon: 'user' },
		{ id: 'apis', label: 'API', icon: 'key' }
	];

	// svelte-ignore state_referenced_locally
	let page = $state(initialPage);

	let bgPreview = $state('');
	let broadcastTimer: ReturnType<typeof setTimeout> | undefined;

	// 主題頁開著的時候，任何一個值被改到就立刻套到畫面上（順便把背景圖的預覽抓回來），
	// 所以這一頁不需要「套用」按鈕。
	$effect(() => {
		if (page !== 'theme') return;
		const theme = $state.snapshot(config.theme);
		applyTheme(theme).then(() => (bgPreview = backgroundUrl()));
	});

	/** 存檔，再通知其他視窗換主題。拖 slider 的時候會連續進來，所以廣播壓一下。 */
	function themeChanged(immediate = false) {
		save(immediate);
		clearTimeout(broadcastTimer);
		broadcastTimer = setTimeout(
			() => api.emitThemeChanged($state.snapshot(config.theme)).catch(() => {}),
			immediate ? 0 : 200
		);
	}

	function useMode(mode: ThemeMode) {
		config.theme.mode = mode;
		themeChanged(true);
	}

	function usePreset(preset: ThemePreset) {
		config.theme.preset = preset.id;
		config.theme.accent = preset.accent;
		themeChanged(true);
	}

	/** 色票跟手打的 hex 共用；打到一半不是合法顏色就先不要動。 */
	function setAccent(value: string) {
		const rgb = parseHex(value);
		if (!rgb) return;
		config.theme.accent = toHex(rgb);
		config.theme.preset = 'custom';
		themeChanged();
	}

	function resetColors() {
		const base = defaultTheme();
		config.theme.preset = base.preset;
		config.theme.accent = base.accent;
		config.theme.tint = base.tint;
		themeChanged(true);
		report('已還原預設強調色');
	}

	async function pickBackground() {
		let picked: string | null = null;
		try {
			const { open } = await import('@tauri-apps/api/dialog');
			const chosen = await open({
				filters: [{ name: '圖片', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp'] }]
			});
			picked = typeof chosen === 'string' ? chosen : null;
		} catch {
			report('這個環境沒有系統對話框，沒辦法選圖片');
			return;
		}
		if (!picked) return;

		try {
			config.theme.background = await api.setBackgroundImage(picked);
			// 不透明度停在 100% 的話，圖會被介面整片蓋掉，看起來就像沒設成功。
			if (config.theme.opacity >= 1) config.theme.opacity = defaultTheme().opacity;
			themeChanged(true);
			report('已更新背景圖片');
		} catch (e) {
			report(`設定背景失敗：${api.errorMessage(e)}`);
		}
	}

	async function clearBackground() {
		config.theme.background = '';
		bgPreview = '';
		themeChanged(true);
		await api.clearBackgroundImage().catch(() => {});
		report('已移除背景圖片');
	}

	const percent = (value: number) => `${Math.round(value * 100)}%`;

	let javaList = $state<JavaCandidate[]>([]);
	let javaLoaded = false;
	let javaBusy = $state('');

	let selectedAccountId = $state<string | null>(null);
	let login = $state<DeviceCode | null>(null);
	let loginError = $state('');
	let offlineDialog = $state<string | null>(null);

	$effect(() => {
		if (page === 'java' && !javaLoaded) {
			javaLoaded = true;
			detectJava();
		}
	});

	/** 32 位元的 Java 記憶體上限很低，在清單上標出來 */
	function is32bit(arch: string) {
		return !!arch && !['amd64', 'x86_64', 'aarch64', 'arm64'].includes(arch);
	}

	async function detectJava() {
		javaBusy = '正在尋找已安裝的 Java…';
		try {
			javaList = await api.detectJavaCandidates();
			report(javaList.length ? `找到 ${javaList.length} 個 Java` : '找不到系統安裝的 Java');
		} catch (e) {
			report(`偵測失敗：${api.errorMessage(e)}`);
		} finally {
			javaBusy = '';
		}
	}

	async function downloadJava(version: number) {
		javaBusy = `正在下載 Java ${version}…`;
		const unlisten = await api.onJavaProgress((p) => {
			if (p.version !== version || !javaBusy) return;
			javaBusy =
				p.message === 'Downloading Java' && p.total > 0
					? `正在下載 Java ${version}（${Math.min(100, Math.round((p.done / p.total) * 100))}%）`
					: p.message === 'Extracting Java'
						? `正在解壓縮 Java ${version}…`
						: `正在下載 Java ${version}…`;
		});
		try {
			const java = await api.installJava(paths.baseDir, version);
			report(`已下載 Java ${java.version}`);
			await detectJava();
		} catch (e) {
			report(`下載失敗：${api.errorMessage(e)}`);
		} finally {
			unlisten();
			javaBusy = '';
		}
	}

	async function testJava() {
		if (!config.java.path) {
			report('目前是自動偵測，沒有指定路徑可以測試');
			return;
		}
		try {
			const java = await api.testJava(config.java.path);
			report(`可以使用：Java ${java.version}`);
		} catch (e) {
			report(`測試失敗：${api.errorMessage(e)}`);
		}
	}


	async function browseFolder(current: string): Promise<string | null> {
		try {
			const { open } = await import('@tauri-apps/api/dialog');
			const picked = await open({ directory: true, defaultPath: current || undefined });
			return typeof picked === 'string' ? picked : null;
		} catch {
			report('這個環境沒有系統對話框，請直接輸入路徑');
			return null;
		}
	}

	async function pickBaseDir() {
		const picked = await browseFolder(config.baseDir || paths.baseDir);
		if (!picked) return;
		config.baseDir = picked;
		save(true);
		report(`資料目錄改為 ${picked}`);
	}

	async function pickJavaPath() {
		try {
			const { open } = await import('@tauri-apps/api/dialog');
			const picked = await open({
				filters: [{ name: 'Java', extensions: ['exe', ''] }],
				defaultPath: config.java.path || undefined
			});
			if (typeof picked === 'string') {
				config.java.path = picked;
				save(true);
			}
		} catch {
			report('這個環境沒有系統對話框，請直接輸入路徑');
		}
	}

	async function startLogin() {
		loginError = '';
		try {
			login = await api.msLoginStart();
			api.openExternal(login.verificationUri).catch(() => {});

			const updated = await api.msLoginPoll(login.sessionId);
			login = null;
			onAccounts(updated);
			report('Microsoft 帳號已加入');
		} catch (e) {
			loginError = api.errorMessage(e);
		}
	}

	async function cancelLogin() {
		if (login) await api.msLoginCancel(login.sessionId).catch(() => {});
		login = null;
	}

	async function accountAction(label: string, fn: () => Promise<AccountView[]>) {
		try {
			onAccounts(await fn());
			report(label);
		} catch (e) {
			report(`${label}失敗：${api.errorMessage(e)}`);
		}
	}

	async function addOffline() {
		const name = offlineDialog?.trim() ?? '';
		offlineDialog = null;
		if (!name) return;
		await accountAction('已新增離線帳號', () => api.addOfflineAccount(name));
	}
</script>

{#snippet slider(
	label: string,
	min: number,
	max: number,
	step: number,
	value: number,
	text: string,
	disabled: boolean,
	oninput: (value: number) => void
)}
	<Field {label}>
		<div class="flex items-center gap-3">
			<Slider
				ariaLabel={label}
				class="max-w-70"
				{min}
				{max}
				{step}
				{value}
				{disabled}
				onchange={oninput}
			/>
			<span class="w-12 shrink-0 text-muted">{text}</span>
		</div>
	</Field>
{/snippet}

{#snippet colorField(id: string, label: string, value: string, onpick: (hex: string) => void)}
	<Field {label} for={id}>
		<ColorField {id} {value} {onpick} />
	</Field>
{/snippet}

{#snippet pathField(id: string, label: string, value: string, onBrowse: () => void)}
	<Field {label} for={id}>
		<div class="flex items-center gap-1.75">
			<TextInput class="min-w-0 flex-1" {id} {value} readonly />
			<Button onclick={onBrowse}>瀏覽…</Button>
		</div>
	</Field>
{/snippet}


<div class="flex min-h-0 flex-1 bg-window">
	<NavList {pages} selected={page} onselect={(id) => (page = id)} width="172px" />


	<div
		class="mx-auto flex w-full min-w-0 max-w-205 flex-1 flex-col gap-1.5 overflow-y-auto px-5.5 pb-5.5 pt-2"
	>
		{#if page === 'launcher'}
			<Group title="資料夾">
				{@render pathField(
					'dir-base',
					'資料根目錄',
					config.baseDir || paths.baseDir,
					pickBaseDir
				)}
				<div class="flex gap-1.75 px-0.5 pt-1">
					<Button onclick={() => api.openLauncherFolder('base')}>開啟</Button>
					<Button onclick={() => api.openLauncherFolder('instances')}>
						開啟實例資料夾
					</Button>
					{#if config.baseDir}
						<Button onclick={() => { config.baseDir = ''; save(true); report('已改回預設資料目錄'); }}>
							還原預設
						</Button>
					{/if}
				</div>
			</Group>

			<Group title="啟動器設定">
				<Field label="設定檔位置" for="store-dir">
					<TextInput id="store-dir" value={paths.storeDir} readonly />
				</Field>
				<div class="px-0.5 pt-1">
					<Button onclick={() => api.openLauncherFolder('store')}>開啟</Button>
				</div>
			</Group>

			<Group title="使用者介面">
				<Switch
					label="顯示實例工具列"
					bind:checked={config.ui.instanceToolbar}
					onchange={() => save(true)}
				/>
				<Switch label="顯示狀態列" bind:checked={config.ui.statusBar} onchange={() => save(true)} />
				<Switch
					label="在實例圖示上顯示執行狀態"
					bind:checked={config.ui.runningBadge}
					onchange={() => save(true)}
				/>
			</Group>

		{:else if page === 'theme'}
			<Group title="主題">
				<div class="flex flex-wrap gap-1.75 px-0.5 pb-1">
					{#each MODES as item (item.id)}
						<button
							class="flex items-center gap-2 rounded-lg border px-3.5 py-2 transition-colors duration-150
								{config.theme.mode === item.id
								? 'border-brand-line bg-brand-soft text-brand-fg'
								: 'border-line text-muted hover:border-line-hi hover:text-ink'}"
							onclick={() => useMode(item.id)}
						>
							<Icon name={item.icon} size={15} />
							{item.label}
						</button>
					{/each}
				</div>
			</Group>

			<Group title="強調色">
				<div class="flex flex-wrap gap-1.75 px-0.5 pb-1">
					{#each PRESETS as preset (preset.id)}
						<button
							class="flex items-center gap-2 rounded-lg border px-3 py-2 transition-colors duration-150
								{config.theme.preset === preset.id
								? 'border-brand-line bg-brand-soft text-brand-fg'
								: 'border-line text-muted hover:border-line-hi hover:text-ink'}"
							onclick={() => usePreset(preset)}
						>
							<span class="size-4 shrink-0 rounded-full" style="background:{preset.accent}"></span>
							{preset.label}
						</button>
					{/each}
					{#if config.theme.preset === 'custom'}
						<span
							class="flex items-center gap-2 rounded-lg border border-brand-line bg-brand-soft px-3 py-2 text-brand-fg"
						>
							<span
								class="size-4 shrink-0 rounded-full"
								style="background:{config.theme.accent}"
							></span>
							自訂
						</span>
					{/if}
				</div>
				{@render colorField('t-accent', '自訂顏色', config.theme.accent, setAccent)}
				{@render slider(
					'介面染色',
					0,
					100,
					1,
					Math.round(config.theme.tint * 100),
					percent(config.theme.tint),
					false,
					(value) => {
						config.theme.tint = value / 100;
						themeChanged();
					}
				)}
				<div class="px-0.5 pt-2">
					<Button onclick={resetColors}>還原預設強調色</Button>
				</div>
			</Group>

			<Group title="背景圖片">
				<div class="flex gap-2.5">
					<div
						class="grid h-27 w-48 shrink-0 place-items-center overflow-hidden rounded-lg border border-line bg-deep"
					>
						{#if bgPreview}
							<img src={bgPreview} alt="背景預覽" class="size-full object-cover" />
						{:else}
							<span class="text-faint">沒有背景圖</span>
						{/if}
					</div>
					<div class="flex w-35 shrink-0 flex-col gap-1 [&>button]:justify-start">
						<Button onclick={pickBackground}>選擇圖片…</Button>
						<Button disabled={!config.theme.background} onclick={clearBackground}>
							移除
						</Button>
					</div>
				</div>

				<div class="pt-3">
					{@render slider(
						'介面不透明度',
						25,
						100,
						1,
						Math.round(config.theme.opacity * 100),
						percent(config.theme.opacity),
						!config.theme.background,
						(value) => {
							config.theme.opacity = value / 100;
							themeChanged();
						}
					)}
					{@render slider(
						'背景變暗',
						0,
						90,
						1,
						Math.round(config.theme.backgroundDim * 100),
						percent(config.theme.backgroundDim),
						!config.theme.background,
						(value) => {
							config.theme.backgroundDim = value / 100;
							themeChanged();
						}
					)}
					{@render slider(
						'背景模糊',
						0,
						40,
						1,
						config.theme.backgroundBlur,
						`${config.theme.backgroundBlur}px`,
						!config.theme.background,
						(value) => {
							config.theme.backgroundBlur = value;
							themeChanged();
						}
					)}
				</div>
			</Group>

		{:else if page === 'minecraft'}
			<Group title="啟動">
				<Switch
					label="啟動時顯示檔案檢查與下載視窗"
					bind:checked={config.game.showProgressWindow}
					onchange={() => save(true)}
				/>
				<Switch
					label="準備完成、遊戲起來之後自動關閉那個視窗"
					bind:checked={config.game.closeProgressWhenDone}
					onchange={() => save(true)}
				/>
			</Group>

			<Group title="遊戲日誌">
				<Field label="保留行數" for="history">
					<TextInput id="history" type="number" style="width:110px" bind:value={config.game.keepLogLines} oninput={() => save()} />
				</Field>
			</Group>

			<Group title="遊戲時間">
				<Switch
					label="記錄每個實例的遊玩時間"
					bind:checked={config.game.recordPlayTime}
					onchange={() => save(true)}
				/>
			</Group>

			<Group title="版本清單">
				<Switch
					label="預設顯示快照版本"
					bind:checked={config.ui.includeSnapshots}
					onchange={() => save(true)}
				/>
			</Group>

		{:else if page === 'java'}
			<Group title="記憶體">
				<Field label="最小配置" for="j-min">
					<div class="flex items-center gap-1.75">
						<TextInput id="j-min" type="number" style="width:110px" bind:value={config.game.minMemoryMb} oninput={() => save()} />
						<span class="text-muted">MiB</span>
					</div>
				</Field>
				<Field label="最大配置" for="j-max">
					<div class="flex items-center gap-1.75">
						<TextInput id="j-max" type="number" style="width:110px" bind:value={config.game.maxMemoryMb} oninput={() => save()} />
						<span class="text-muted">MiB</span>
					</div>
				</Field>
			</Group>

			<Group title="Java 執行階段">
				<div class="flex gap-2.25">
					<TableBox class="max-h-40">
						<Table>
							<thead>
								<tr>
									<th style="width:90px">版本</th>
									<th style="width:140px">廠商</th>
									<th>路徑</th>
								</tr>
							</thead>
							<tbody>
								{#each javaList as java (java.path)}
									<tr
										class:selected={config.java.path === java.path}
										onclick={() => {
											config.java.path = java.path;
											save(true);
										}}
									>
										<td title={java.fullVersion}>
											Java {java.version}{is32bit(java.arch) ? `（${java.arch}）` : ''}
										</td>
										<td class="truncate" title={java.vendor}>{java.vendor || '—'}</td>
										<td class="font-mono text-[11.5px]" title={java.path}>{java.path}</td>
									</tr>
								{/each}
							</tbody>
						</Table>
						{#if javaList.length === 0}
							<p class="px-3 py-4 text-center text-muted">
								{javaBusy || '沒有偵測到系統安裝的 Java。'}
							</p>
						{/if}
					</TableBox>
					<div class="flex w-35 shrink-0 flex-col gap-1 [&>button]:justify-start">
						<Button disabled={!!javaBusy} onclick={detectJava}>重新偵測</Button>
						<Button disabled={!!javaBusy} onclick={testJava}>測試</Button>
						<Button disabled={!!javaBusy} onclick={() => downloadJava(21)}>
							下載 Java 21
						</Button>
						<Button disabled={!!javaBusy} onclick={() => downloadJava(17)}>
							下載 Java 17
						</Button>
						<Button disabled={!!javaBusy} onclick={() => downloadJava(8)}>
							下載 Java 8
						</Button>
					</div>
				</div>

				<Field label="預設 Java" for="java-path">
					<div class="flex items-center gap-1.75">
						<TextInput class="min-w-0 flex-1" id="java-path" bind:value={config.java.path} oninput={() => save()} placeholder="（依實例需要的版本自動挑選）" />
						<Button onclick={pickJavaPath}>瀏覽…</Button>
						<Button disabled={!config.java.path} onclick={() => { config.java.path = ''; save(true); }}>
							自動挑選
						</Button>
					</div>
				</Field>
				{#if javaBusy}
					<p class="px-0.5 py-1 text-muted">{javaBusy}</p>
				{/if}
			</Group>

			<Group title="JVM 參數">
				<Field label="額外參數" for="jvm">
					<TextInput id="jvm" bind:value={config.game.jvmArgs} oninput={() => save()} placeholder="-XX:+UseG1GC -XX:MaxGCPauseMillis=50" />
				</Field>
			</Group>

		{:else if page === 'commands'}
			<Group title="自訂指令">
				<Field label="啟動前執行" for="c-pre">
					<TextInput id="c-pre" bind:value={config.commands.preLaunch} oninput={() => save()} placeholder="例如：backup.bat" />
				</Field>
				<Field label="結束後執行" for="c-post">
					<TextInput id="c-post" bind:value={config.commands.postExit} oninput={() => save()} />
				</Field>
			</Group>
			<p class="px-0.5 py-1 leading-[1.7] text-faint">
				可用變數：
				<code class="rounded-sm border border-line bg-deep px-1 py-px font-mono">$INST_ID</code>、
				<code class="rounded-sm border border-line bg-deep px-1 py-px font-mono">$INST_NAME</code>、
				<code class="rounded-sm border border-line bg-deep px-1 py-px font-mono">$INST_DIR</code>、
				<code class="rounded-sm border border-line bg-deep px-1 py-px font-mono">$INST_MC_DIR</code
				>、
				<code class="rounded-sm border border-line bg-deep px-1 py-px font-mono">$INST_JAVA</code>
			</p>

		{:else if page === 'accounts'}
			<div class="flex min-h-0 flex-1 gap-2.25">
				<TableBox>
					<Table>
						<thead>
							<tr>
								<th style="width:44px"></th>
								<th style="width:40%">使用者名稱</th>
								<th>類型</th>
								<th style="width:110px">狀態</th>
							</tr>
						</thead>
						<tbody>
							{#each accounts as item (item.id)}
								<tr
									class:selected={selectedAccountId === item.id}
									onclick={() => (selectedAccountId = item.id)}
								>
									<td><InstanceIcon name="steve" size={20} /></td>
									<td>{item.playerName}</td>
									<td class="text-muted">{item.kind === 'microsoft' ? 'Microsoft' : '離線'}</td>
									<td class="text-muted">
										{item.selected ? '使用中' : item.expired ? '需重新登入' : '已登入'}
									</td>
								</tr>
							{/each}
						</tbody>
					</Table>
					{#if accounts.length === 0}
						<p class="px-3 py-4 text-center text-muted">還沒有任何帳號。</p>
					{/if}
				</TableBox>
				<div class="flex w-35 shrink-0 flex-col gap-1 [&>button]:justify-start">
					<Button onclick={startLogin}>新增 Microsoft…</Button>
					<Button onclick={() => (offlineDialog = '')}>新增離線…</Button>
					<Button disabled={!selectedAccountId} onclick={() => selectedAccountId && accountAction('已設為使用中', () => api.selectAccount(selectedAccountId!))}>
						設為使用中
					</Button>
					<Button disabled={!selectedAccountId} onclick={() => selectedAccountId && accountAction('已更新權杖', () => api.refreshAccount(selectedAccountId!))}>
						重新整理
					</Button>
					<Button variant="destructive" disabled={!selectedAccountId} onclick={() => selectedAccountId && accountAction('已移除帳號', () => api.removeAccount(selectedAccountId!))}>
						移除
					</Button>
				</div>
			</div>

		{:else}
			<Group title="CurseForge">
				<Field label="API 金鑰" for="a-cf">
					<TextInput id="a-cf" type="password" bind:value={config.api.curseforgeApiKey} oninput={() => save()} placeholder="（不填就只查 Modrinth）" />
				</Field>
			</Group>
		{/if}
	</div>
</div>

{#if login}
	<Dialog title="Microsoft 登入" width="440px" onclose={cancelLogin}>
		<div class="flex flex-col gap-3 px-5 py-5">
			<p class="text-muted">在瀏覽器輸入這組代碼完成授權。</p>
			<div
				class="rounded-lg border border-line bg-deep px-4 py-3 text-center font-mono text-[22px] tracking-[4px]"
			>
				{login.userCode}
			</div>
			<Button onclick={() => login && api.openExternal(login.verificationUri)}>
				重新開啟 {login.verificationUri}
			</Button>
			<p class="flex items-center gap-2 text-faint">
				<span class="size-3.5 animate-spin rounded-full border-2 border-line border-t-brand"></span>
				等待授權中…
			</p>
		</div>
		{#snippet footer()}
			<Button onclick={cancelLogin}>取消</Button>
		{/snippet}
	</Dialog>
{/if}

{#if loginError}
	<Dialog title="登入失敗" width="420px" onclose={() => (loginError = '')}>
		<p class="px-5 py-5 leading-[1.8] text-danger">{loginError}</p>
		{#snippet footer()}
			<Button variant="primary" onclick={() => (loginError = '')}>關閉</Button>
		{/snippet}
	</Dialog>
{/if}

{#if offlineDialog !== null}
	<Dialog title="新增離線帳號" width="380px" onclose={() => (offlineDialog = null)}>
		<div class="px-5 py-5">
			<Field label="玩家名稱" for="offline-name">
				<TextInput id="offline-name" bind:value={offlineDialog} maxlength={16} />
			</Field>
			<p class="mt-2 text-faint">只能用英數字與底線。</p>
		</div>
		{#snippet footer()}
			<Button variant="primary" disabled={!offlineDialog?.trim()} onclick={addOffline}>
				新增
			</Button>
			<Button onclick={() => (offlineDialog = null)}>取消</Button>
		{/snippet}
	</Dialog>
{/if}
