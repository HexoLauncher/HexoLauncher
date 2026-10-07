<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import type { AccountView, DeviceCode, InstanceView, JavaInfo, LoaderKind } from '$lib/api';

	/**
	 * 直接打後端指令的測試台，不套介面樣式。
	 * 正式的操作都在主視窗，這頁只是拿來確認每個 command 的回傳值。
	 */
	let logs = $state<string[]>([]);

	function log(...parts: unknown[]) {
		const text = parts
			.map((p) => (typeof p === 'string' ? p : JSON.stringify(p, null, 2)))
			.join(' ');
		logs = [`[${new Date().toLocaleTimeString()}] ${text}`, ...logs].slice(0, 300);
	}

	/** 統一包一層：把錯誤印出來而不是丟到 console 裡消失 */
	async function run<T>(label: string, fn: () => Promise<T>): Promise<T | undefined> {
		log(`▶ ${label}`);
		try {
			const result = await fn();
			log(`✔ ${label}`, result === undefined ? '' : result);
			return result;
		} catch (e) {
			log(`✘ ${label}`, e);
			return undefined;
		}
	}

	let baseDir = $state('');
	let instances = $state<InstanceView[]>([]);
	let selected = $state('');
	let accounts = $state<AccountView[]>([]);
	let device = $state<DeviceCode | null>(null);
	let java = $state<JavaInfo | null>(null);
	let javaVersionInput = $state(21);
	let running = $state<api.RunningGame[]>([]);
	let progress = $state('');

	let mcVersion = $state('');
	let loader = $state<LoaderKind>('vanilla');
	let loaderVersion = $state('');
	let newName = $state('測試實例');
	let offlineName = $state('Player');

	// 遊戲輸出：量很大，只留最近這麼多行
	const MAX_GAME_LINES = 500;
	let gameLines = $state<{ kind: string; line: string }[]>([]);
	let gameLogEl = $state<HTMLPreElement | null>(null);

	onMount(() => {
		const unlisteners: Array<() => void> = [];
		const track = (p: Promise<() => void>) => p.then((un) => unlisteners.push(un));

		track(
			api.onInstallProgress((p) => {
				progress = `${p.instanceName} ${p.done}/${p.total} ${p.message}`;
			})
		);
		track(
			api.onGameExit((e) => {
				log('遊戲結束', e);
				refreshRunning();
			})
		);
		track(
			api.onGameOutput((out) => {
				gameLines.push(out);
				if (gameLines.length > MAX_GAME_LINES) gameLines = gameLines.slice(-MAX_GAME_LINES);
				queueMicrotask(() => gameLogEl?.scrollTo(0, gameLogEl.scrollHeight));
			})
		);
		track(api.onInstancesChanged(() => refreshInstances()));

		run('resolve_paths', () => api.resolvePaths()).then((paths) => {
			if (!paths) return;
			baseDir = paths.baseDir;
			refreshInstances();
			refreshAccounts();
		});

		return () => unlisteners.forEach((un) => un());
	});

	async function refreshInstances() {
		const list = await run('list_instances', () => api.listInstances(baseDir));
		if (!list) return;
		instances = list;
		if (!selected && list.length > 0) selected = list[0].id;
	}

	async function refreshAccounts() {
		const list = await run('list_accounts', () => api.listAccounts());
		if (list) accounts = list;
	}

	async function refreshRunning() {
		const list = await run('running_instances', () => api.runningInstances());
		if (list) running = list;
	}

	async function loadLoaderVersions() {
		loaderVersion = '';
		if (loader === 'vanilla' || !mcVersion) return;
		const list = await run('list_loader_versions', () => api.listLoaderVersions(loader, mcVersion));
		if (list) loaderVersion = list[0] ?? '';
	}

	async function install() {
		progress = '';
		const instanceName = await run('suggest_instance_id', () =>
			api.suggestInstanceId(baseDir, newName)
		);
		if (!instanceName) return;

		const created = await run('install_instance', () =>
			api.installInstance({
				baseDir,
				instanceName,
				name: newName,
				mcVersion,
				loader,
				loaderVersion: loader === 'vanilla' ? null : loaderVersion || null
			})
		);
		if (created) selected = created.id;
	}

	async function msLogin() {
		const code = await run('ms_login_start', () => api.msLoginStart());
		if (!code) return;
		device = code;
		const list = await run('ms_login_poll', () => api.msLoginPoll(code.sessionId));
		device = null;
		if (list) accounts = list;
	}
</script>

<h1>Nolauncher 測試台</h1>

<h2>1. 路徑</h2>
<input size="60" bind:value={baseDir} />
<button onclick={() => run('resolve_paths', () => api.resolvePaths())}>resolve_paths</button>
<button onclick={() => run('prepare_base_dir', () => api.prepareBaseDir(baseDir))}>建立目錄</button>
<button onclick={() => run('get_launcher_config', () => api.getLauncherConfig())}>讀設定</button>
<button onclick={() => run('open_launcher_folder', () => api.openLauncherFolder('store'))}>
	開設定資料夾
</button>

<h2>2. 安裝</h2>
名稱 <input bind:value={newName} />
MC 版本 <input bind:value={mcVersion} placeholder="1.21.8" onchange={loadLoaderVersions} />
<select bind:value={loader} onchange={loadLoaderVersions}>
	<option value="vanilla">vanilla</option>
	<option value="fabric">fabric</option>
	<option value="forge">forge</option>
	<option value="neoforge">neoforge</option>
</select>
loader 版本 <input bind:value={loaderVersion} disabled={loader === 'vanilla'} />
<button onclick={() => run('list_versions', () => api.listVersions(true))}>版本清單</button>
<button onclick={install} disabled={!baseDir || !mcVersion}>安裝</button>
<p>進度: {progress || '—'}</p>

<h2>3. 實例</h2>
<button onclick={refreshInstances}>重新整理</button>
<select bind:value={selected}>
	{#each instances as i (i.id)}
		<option value={i.id}>
			{i.id} — {i.name} / {i.versionId} / {i.loader} / {i.healthy ? 'ok' : '需修復'}
		</option>
	{/each}
</select>
<button disabled={!selected} onclick={() => run('get_instance', () => api.getInstance(baseDir, selected))}>
	詳細
</button>
<button disabled={!selected} onclick={() => run('repair_instance', () => api.repairInstance(baseDir, selected))}>
	修復
</button>
<button
	disabled={!selected}
	onclick={() => run('open_instance_folder', () => api.openInstanceFolder(baseDir, selected))}
>
	開資料夾
</button>
<button
	disabled={!selected}
	onclick={() => run('duplicate_instance', () => api.duplicateInstance(baseDir, selected, `${selected}-copy`))}
>
	複製
</button>
<button
	disabled={!selected}
	onclick={() => run('delete_instance', () => api.deleteInstance(baseDir, selected))}
>
	刪除
</button>

<h2>4. Java</h2>
<input type="number" bind:value={javaVersionInput} style="width:4em" />
<button onclick={() => run('detect_java', () => api.detectJava(javaVersionInput)).then((j) => (java = j ?? null))}>
	偵測
</button>
<button onclick={() => run('detect_java_candidates', () => api.detectJavaCandidates())}>列出全部</button>
<button onclick={() => run('install_java', () => api.installJava(baseDir, javaVersionInput)).then((j) => j && (java = j))}>
	下載
</button>
<button onclick={() => run('ensure_java', () => api.ensureJava(baseDir, javaVersionInput)).then((j) => j && (java = j))}>
	確保可用
</button>
<p>目前: {java ? `${java.path} (v${java.version})` : '—'}</p>

<h2>5. 帳號</h2>
離線名稱 <input bind:value={offlineName} />
<button onclick={() => run('add_offline_account', () => api.addOfflineAccount(offlineName)).then((a) => a && (accounts = a))}>
	新增離線
</button>
<button onclick={msLogin}>Microsoft 登入</button>
<button onclick={refreshAccounts}>重新整理</button>
{#if device}
	<p>
		到 <b>{device.verificationUri}</b> 輸入代碼 <b>{device.userCode}</b>
		<button onclick={() => run('open_external', () => api.openExternal(device!.verificationUri))}>
			用瀏覽器開
		</button>
		<button onclick={() => run('ms_login_cancel', () => api.msLoginCancel(device!.sessionId)).then(() => (device = null))}>
			取消
		</button>
	</p>
{/if}
<ul>
	{#each accounts as a (a.id)}
		<li>
			{a.selected ? '● ' : '○ '}{a.playerName} ({a.kind}{a.expired ? ' / 已過期' : ''})
			<button onclick={() => run('select_account', () => api.selectAccount(a.id)).then((x) => x && (accounts = x))}>
				使用
			</button>
			<button onclick={() => run('remove_account', () => api.removeAccount(a.id)).then((x) => x && (accounts = x))}>
				移除
			</button>
		</li>
	{/each}
</ul>

<h2>6. 啟動</h2>
<button
	disabled={!selected}
	onclick={() =>
		run('launch_instance', () => api.launchInstance({ baseDir, instanceName: selected })).then(refreshRunning)}
>
	啟動 {selected || ''}
</button>
<button disabled={!selected} onclick={() => run('stop_instance', () => api.stopInstance(selected)).then(refreshRunning)}>
	停止
</button>
<button onclick={refreshRunning}>執行中清單</button>
<p>執行中: {running.map((r) => `${r.instanceName} (pid ${r.pid})`).join(', ') || '—'}</p>

<h4>遊戲輸出 ({gameLines.length} 行)</h4>
<button onclick={() => (gameLines = [])}>清空</button>
<pre
	bind:this={gameLogEl}
	style="white-space:pre-wrap;max-height:20em;overflow:auto;border:1px solid #ccc;font-size:12px">{#each gameLines as l, i (i)}<div
		style={l.kind === 'stderr' ? 'color:#c00' : ''}
	>{l.line}</div>{/each}</pre>

<h2>7. 內容</h2>
<button disabled={!selected} onclick={() => run('list_mods', () => api.listMods(baseDir, selected))}>模組</button>
<button disabled={!selected} onclick={() => run('check_mod_updates', () => api.checkModUpdates({ baseDir, instanceName: selected }))}>
	模組更新
</button>
<button disabled={!selected} onclick={() => run('list_resource_packs', () => api.listResourcePacks(baseDir, selected))}>
	資源包
</button>
<button disabled={!selected} onclick={() => run('list_shader_packs', () => api.listShaderPacks(baseDir, selected))}>
	光影包
</button>
<button disabled={!selected} onclick={() => run('list_worlds', () => api.listWorlds(baseDir, selected))}>世界</button>
<button disabled={!selected} onclick={() => run('list_screenshots', () => api.listScreenshots(baseDir, selected))}>
	截圖
</button>
<button disabled={!selected} onclick={() => run('list_instance_logs', () => api.listInstanceLogs(baseDir, selected))}>
	日誌清單
</button>

<h2>紀錄</h2>
<button onclick={() => (logs = [])}>清空</button>
<pre style="white-space:pre-wrap;font-size:12px">{logs.join('\n')}</pre>
