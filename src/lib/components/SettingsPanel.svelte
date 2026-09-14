<script lang="ts">
	import PageList from './PageList.svelte';
	import Toggle from './Toggle.svelte';
	import Window from './Window.svelte';
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import * as api from '$lib/api';
	import type { AccountView, DeviceCode, JavaInfo, LauncherConfig, Paths } from '$lib/api';
	import type { Page } from '$lib/ui';

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
		{ id: 'minecraft', label: '遊戲與啟動', icon: 'box' },
		{ id: 'java', label: 'Java', icon: 'coffee' },
		{ id: 'commands', label: '自訂指令', icon: 'terminal' },
		{ id: 'accounts', label: '帳號', icon: 'user' },
		{ id: 'apis', label: 'API', icon: 'key' }
	];

	// svelte-ignore state_referenced_locally
	let page = $state(initialPage);

	let javaList = $state<JavaInfo[]>([]);
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
		try {
			const java = await api.installJava(paths.baseDir, version);
			report(`已下載 Java ${java.version}`);
			await detectJava();
		} catch (e) {
			report(`下載失敗：${api.errorMessage(e)}`);
		} finally {
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

{#snippet pathField(id: string, label: string, value: string, onBrowse: () => void)}
	<div class="qt-field">
		<label for={id}>{label}</label>
		<div class="flex items-center gap-1.75">
			<input {id} class="qt-input min-w-0 flex-1" {value} readonly />
			<button class="qt-button" onclick={onBrowse}>瀏覽…</button>
		</div>
	</div>
{/snippet}

<div class="flex min-h-0 flex-1">
	<PageList {pages} selected={page} onselect={(id) => (page = id)} width="172px" />


	<div
		class="mx-auto flex w-full min-w-0 max-w-205 flex-1 flex-col gap-1.5 overflow-y-auto px-5.5 pb-5.5 pt-2"
	>
		{#if page === 'launcher'}
			<div class="qt-group">
				<span class="qt-group-title">資料夾</span>
				{@render pathField(
					'dir-base',
					'資料根目錄',
					config.baseDir || paths.baseDir,
					pickBaseDir
				)}
				<div class="flex gap-1.75 px-0.5 pt-1">
					<button class="qt-button" onclick={() => api.openLauncherFolder('base')}>開啟</button>
					<button class="qt-button" onclick={() => api.openLauncherFolder('instances')}>
						開啟實例資料夾
					</button>
					{#if config.baseDir}
						<button
							class="qt-button"
							onclick={() => {
								config.baseDir = '';
								save(true);
								report('已改回預設資料目錄');
							}}
						>
							還原預設
						</button>
					{/if}
				</div>
			</div>

			<div class="qt-group">
				<span class="qt-group-title">啟動器設定</span>
				<div class="qt-field">
					<label for="store-dir">設定檔位置</label>
					<input id="store-dir" class="qt-input" value={paths.storeDir} readonly />
				</div>
				<div class="px-0.5 pt-1">
					<button class="qt-button" onclick={() => api.openLauncherFolder('store')}>開啟</button>
				</div>
			</div>

			<div class="qt-group">
				<span class="qt-group-title">使用者介面</span>
				<Toggle
					label="顯示實例工具列"
					bind:checked={config.ui.instanceToolbar}
					onchange={() => save(true)}
				/>
				<Toggle label="顯示狀態列" bind:checked={config.ui.statusBar} onchange={() => save(true)} />
				<Toggle
					label="在實例圖示上顯示執行狀態"
					bind:checked={config.ui.runningBadge}
					onchange={() => save(true)}
				/>
			</div>

		{:else if page === 'minecraft'}
			<div class="qt-group">
				<span class="qt-group-title">啟動</span>
				<Toggle
					label="啟動時顯示檔案檢查與下載視窗"
					bind:checked={config.game.showProgressWindow}
					onchange={() => save(true)}
				/>
				<Toggle
					label="準備完成、遊戲起來之後自動關閉那個視窗"
					bind:checked={config.game.closeProgressWhenDone}
					onchange={() => save(true)}
				/>
			</div>

			<div class="qt-group">
				<span class="qt-group-title">遊戲日誌</span>
				<div class="qt-field">
					<label for="history">保留行數</label>
					<input
						id="history"
						class="qt-input"
						type="number"
						style="width:110px"
						bind:value={config.game.keepLogLines}
						oninput={() => save()}
					/>
				</div>
			</div>

			<div class="qt-group">
				<span class="qt-group-title">遊戲時間</span>
				<Toggle
					label="記錄每個實例的遊玩時間"
					bind:checked={config.game.recordPlayTime}
					onchange={() => save(true)}
				/>
			</div>

			<div class="qt-group">
				<span class="qt-group-title">版本清單</span>
				<Toggle
					label="預設顯示快照版本"
					bind:checked={config.ui.includeSnapshots}
					onchange={() => save(true)}
				/>
			</div>

		{:else if page === 'java'}
			<div class="qt-group">
				<span class="qt-group-title">記憶體</span>
				<div class="qt-field">
					<label for="j-min">最小配置</label>
					<div class="flex items-center gap-1.75">
						<input
							id="j-min"
							class="qt-input"
							type="number"
							style="width:110px"
							bind:value={config.game.minMemoryMb}
							oninput={() => save()}
						/>
						<span class="text-muted">MiB</span>
					</div>
				</div>
				<div class="qt-field">
					<label for="j-max">最大配置</label>
					<div class="flex items-center gap-1.75">
						<input
							id="j-max"
							class="qt-input"
							type="number"
							style="width:110px"
							bind:value={config.game.maxMemoryMb}
							oninput={() => save()}
						/>
						<span class="text-muted">MiB</span>
					</div>
				</div>
			</div>

			<div class="qt-group">
				<span class="qt-group-title">Java 執行階段</span>
				<div class="flex gap-2.25">
					<div class="qt-table-wrap max-h-40">
						<table class="qt-table">
							<thead>
								<tr><th style="width:90px">版本</th><th>路徑</th></tr>
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
										<td>Java {java.version}</td>
										<td class="font-mono text-[11.5px]">{java.path}</td>
									</tr>
								{/each}
							</tbody>
						</table>
						{#if javaList.length === 0}
							<p class="px-3 py-4 text-center text-muted">
								{javaBusy || '沒有偵測到系統安裝的 Java。'}
							</p>
						{/if}
					</div>
					<div class="flex w-35 shrink-0 flex-col gap-1 [&>button]:justify-start">
						<button class="qt-button" disabled={!!javaBusy} onclick={detectJava}>重新偵測</button>
						<button class="qt-button" disabled={!!javaBusy} onclick={testJava}>測試</button>
						<button class="qt-button" disabled={!!javaBusy} onclick={() => downloadJava(21)}>
							下載 Java 21
						</button>
						<button class="qt-button" disabled={!!javaBusy} onclick={() => downloadJava(17)}>
							下載 Java 17
						</button>
						<button class="qt-button" disabled={!!javaBusy} onclick={() => downloadJava(8)}>
							下載 Java 8
						</button>
					</div>
				</div>

				<div class="qt-field">
					<label for="java-path">指定路徑</label>
					<div class="flex items-center gap-1.75">
						<input
							id="java-path"
							class="qt-input min-w-0 flex-1"
							bind:value={config.java.path}
							oninput={() => save()}
							placeholder="（自動偵測）"
						/>
						<button class="qt-button" onclick={pickJavaPath}>瀏覽…</button>
					</div>
				</div>
				<Toggle
					label="找不到需要的版本時自動下載"
					bind:checked={config.java.autoDownload}
					onchange={() => save(true)}
				/>
				{#if javaBusy}
					<p class="px-0.5 py-1 text-muted">{javaBusy}</p>
				{/if}
			</div>

			<div class="qt-group">
				<span class="qt-group-title">JVM 參數</span>
				<div class="qt-field">
					<label for="jvm">額外參數</label>
					<input
						id="jvm"
						class="qt-input"
						bind:value={config.game.jvmArgs}
						oninput={() => save()}
						placeholder="-XX:+UseG1GC -XX:MaxGCPauseMillis=50"
					/>
				</div>
			</div>

		{:else if page === 'commands'}
			<div class="qt-group">
				<span class="qt-group-title">自訂指令</span>
				<div class="qt-field">
					<label for="c-pre">啟動前執行</label>
					<input
						id="c-pre"
						class="qt-input"
						bind:value={config.commands.preLaunch}
						oninput={() => save()}
						placeholder="例如：backup.bat"
					/>
				</div>
				<div class="qt-field">
					<label for="c-post">結束後執行</label>
					<input
						id="c-post"
						class="qt-input"
						bind:value={config.commands.postExit}
						oninput={() => save()}
					/>
				</div>
			</div>
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
				<div class="qt-table-wrap">
					<table class="qt-table">
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
					</table>
					{#if accounts.length === 0}
						<p class="px-3 py-4 text-center text-muted">還沒有任何帳號。</p>
					{/if}
				</div>
				<div class="flex w-35 shrink-0 flex-col gap-1 [&>button]:justify-start">
					<button class="qt-button" onclick={startLogin}>新增 Microsoft…</button>
					<button class="qt-button" onclick={() => (offlineDialog = '')}>新增離線…</button>
					<button
						class="qt-button"
						disabled={!selectedAccountId}
						onclick={() =>
							selectedAccountId &&
							accountAction('已設為使用中', () => api.selectAccount(selectedAccountId!))}
					>
						設為使用中
					</button>
					<button
						class="qt-button"
						disabled={!selectedAccountId}
						onclick={() =>
							selectedAccountId &&
							accountAction('已更新權杖', () => api.refreshAccount(selectedAccountId!))}
					>
						重新整理
					</button>
					<button
						class="qt-button destructive"
						disabled={!selectedAccountId}
						onclick={() =>
							selectedAccountId &&
							accountAction('已移除帳號', () => api.removeAccount(selectedAccountId!))}
					>
						移除
					</button>
				</div>
			</div>

		{:else}
			<div class="qt-group">
				<span class="qt-group-title">CurseForge</span>
				<div class="qt-field">
					<label for="a-cf">API 金鑰</label>
					<input
						id="a-cf"
						class="qt-input"
						type="password"
						bind:value={config.api.curseforgeApiKey}
						oninput={() => save()}
						placeholder="（不填就只查 Modrinth）"
					/>
				</div>
			</div>
		{/if}
	</div>
</div>

{#if login}
	<Window title="Microsoft 登入" width="440px" onclose={cancelLogin}>
		<div class="flex flex-col gap-3 px-5 py-5">
			<p class="text-muted">在瀏覽器輸入這組代碼完成授權。</p>
			<div
				class="rounded-lg border border-line bg-deep px-4 py-3 text-center font-mono text-[22px] tracking-[4px]"
			>
				{login.userCode}
			</div>
			<button class="qt-button" onclick={() => login && api.openExternal(login.verificationUri)}>
				重新開啟 {login.verificationUri}
			</button>
			<p class="flex items-center gap-2 text-faint">
				<span class="size-3.5 animate-spin rounded-full border-2 border-line border-t-brand"></span>
				等待授權中…
			</p>
		</div>
		{#snippet footer()}
			<button class="qt-button" onclick={cancelLogin}>取消</button>
		{/snippet}
	</Window>
{/if}

{#if loginError}
	<Window title="登入失敗" width="420px" onclose={() => (loginError = '')}>
		<p class="px-5 py-5 leading-[1.8] text-danger">{loginError}</p>
		{#snippet footer()}
			<button class="qt-button primary" onclick={() => (loginError = '')}>關閉</button>
		{/snippet}
	</Window>
{/if}

{#if offlineDialog !== null}
	<Window title="新增離線帳號" width="380px" onclose={() => (offlineDialog = null)}>
		<div class="px-5 py-5">
			<div class="qt-field">
				<label for="offline-name">玩家名稱</label>
				<input id="offline-name" class="qt-input" bind:value={offlineDialog} maxlength={16} />
			</div>
			<p class="mt-2 text-faint">只能用英數字與底線。</p>
		</div>
		{#snippet footer()}
			<button class="qt-button primary" disabled={!offlineDialog?.trim()} onclick={addOffline}>
				新增
			</button>
			<button class="qt-button" onclick={() => (offlineDialog = null)}>取消</button>
		{/snippet}
	</Window>
{/if}
