<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import type {
		Account,
		DeviceCode,
		InstanceSummary,
		JavaInfo,
		LoaderKind,
		ModEntry,
		PackEntry,
		VersionSummary,
		WorldEntry
	} from '$lib/api';

	// ---------- log ----------
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

	// ---------- 狀態 ----------
	let baseDir = $state('');

	let versions = $state<VersionSummary[]>([]);
	let includeSnapshots = $state(false);
	let mcVersion = $state('');
	let loader = $state<LoaderKind>('vanilla');
	let loaderVersions = $state<string[]>([]);
	let loaderVersion = $state('');
	let instanceName = $state('');

	let instances = $state<InstanceSummary[]>([]);
	let selected = $state('');

	let java = $state<JavaInfo | null>(null);
	let javaVersionInput = $state(21);

	let playerName = $state('Player');
	let account = $state<Account | null>(null);
	let device = $state<DeviceCode | null>(null);

	let running = $state<string[]>([]);
	let progress = $state('');

	let mods = $state<ModEntry[]>([]);
	let resourcePacks = $state<PackEntry[]>([]);
	let shaderPacks = $state<PackEntry[]>([]);
	let worlds = $state<WorldEntry[]>([]);

	onMount(() => {
		const unlisteners: Array<() => void> = [];

		api.onInstallProgress((p) => {
			progress = `${p.instanceName} ${p.done}/${p.total} ${p.message}`;
		}).then((un) => unlisteners.push(un));

		api.onGameExit((e) => {
			log('遊戲結束', e);
			refreshRunning();
		}).then((un) => unlisteners.push(un));

		// 進來先抓預設目錄 + 版本列表
		api.defaultBaseDir().then(
			(info) => {
				baseDir = info.path;
				log('預設 baseDir', info);
				refreshInstances();
			},
			(e) => log('✘ defaultBaseDir', e)
		);
		loadVersions();

		return () => unlisteners.forEach((un) => un());
	});

	// ---------- 動作 ----------
	async function refreshInstances() {
		const list = await run('list_instances', () => api.listInstances(baseDir));
		if (list) {
			instances = list;
			if (!selected && list.length > 0) selected = list[0].name;
		}
	}

	async function refreshRunning() {
		const list = await run('running_instances', () => api.runningInstances());
		if (list) running = list;
	}

	async function loadVersions() {
		const list = await run('list_versions', () => api.listVersions(includeSnapshots));
		if (!list) return;
		versions = list.versions;
		// 現在選的版本如果不在新清單裡（例如關掉快照），退回最新正式版
		if (!versions.some((v) => v.id === mcVersion)) {
			mcVersion = list.latestRelease || versions[0]?.id || '';
		}
		await loadLoaderVersions();
	}

	/** loader 或 mc 版本一變就重抓；vanilla 沒有 loader 版本 */
	async function loadLoaderVersions() {
		loaderVersions = [];
		loaderVersion = '';
		if (loader === 'vanilla' || !mcVersion) return;
		const list = await run('list_loader_versions', () =>
			api.listLoaderVersions(loader, mcVersion)
		);
		if (list) {
			loaderVersions = list;
			loaderVersion = list[0] ?? '';
		}
	}

	async function install() {
		progress = '';
		const name = instanceName || `${mcVersion}-${loader}`;
		const result = await run('install_instance', () =>
			api.installInstance({
				baseDir,
				mcVersion,
				instanceName: name,
				loader,
				loaderVersion: loader === 'vanilla' ? null : loaderVersion || null
			})
		);
		if (result) {
			selected = result.name;
			refreshInstances();
		}
	}

	async function launch() {
		const info = await run('launch_instance', () =>
			api.launchInstance({
				baseDir,
				instanceName: selected,
				account: account
					? {
							playerName: account.playerName,
							accessToken: account.accessToken,
							uuid: account.uuid,
							xuid: account.xuid
						}
					: { playerName },
				autoDownloadJava: true
			})
		);
		if (info) refreshRunning();
	}

	async function msLogin() {
		const code = await run('ms_login_start', () => api.msLoginStart());
		if (!code) return;
		device = code;
		const acc = await run('ms_login_poll', () => api.msLoginPoll(code.sessionId));
		device = null;
		if (acc) {
			account = acc;
			playerName = acc.playerName;
		}
	}
</script>

<h1>HexoLauncher 測試台</h1>

<h2>1. 目錄</h2>
<input size="60" bind:value={baseDir} />
<button onclick={() => run('default_base_dir', () => api.defaultBaseDir()).then((i) => i && (baseDir = i.path))}>
	預設目錄
</button>
<button onclick={() => run('prepare_base_dir', () => api.prepareBaseDir(baseDir))}>建立目錄</button>

<h2>2. 版本 / 安裝</h2>
<label><input type="checkbox" bind:checked={includeSnapshots} onchange={loadVersions} /> 含快照</label>
<button onclick={loadVersions}>重新取得版本列表</button>
<br />
MC 版本 ({versions.length}):
<select bind:value={mcVersion} onchange={loadLoaderVersions}>
	{#each versions as v (v.id)}
		<option value={v.id}>{v.id} ({v.versionType})</option>
	{/each}
</select>
Loader:
<select bind:value={loader} onchange={loadLoaderVersions}>
	<option value="vanilla">vanilla</option>
	<option value="fabric">fabric</option>
	<option value="forge">forge</option>
	<option value="neoforge">neoforge</option>
</select>
Loader 版本 ({loaderVersions.length}):
<select bind:value={loaderVersion} disabled={loader === 'vanilla'}>
	{#each loaderVersions as lv (lv)}
		<option value={lv}>{lv}</option>
	{/each}
</select>
<button onclick={loadLoaderVersions} disabled={loader === 'vanilla' || !mcVersion}>重抓</button>
<br />
instance 名稱:
<input bind:value={instanceName} placeholder={`${mcVersion || '?'}-${loader}`} />
<button onclick={install} disabled={!baseDir || !mcVersion}>安裝</button>
<p>進度: {progress || '—'}</p>

<h2>3. Instance</h2>
<button onclick={refreshInstances}>重新整理</button>
<select bind:value={selected}>
	{#each instances as i (i.name)}
		<option value={i.name}>{i.name} — {i.versionId} / {i.loader} / java {i.javaVersion}</option>
	{/each}
</select>
<button onclick={() => run('get_instance', () => api.getInstance(baseDir, selected))} disabled={!selected}>
	詳細
</button>
<button onclick={() => run('open_instance_folder', () => api.openInstanceFolder(baseDir, selected))} disabled={!selected}>
	開資料夾
</button>
<button
	onclick={() => run('delete_instance', () => api.deleteInstance(baseDir, selected)).then(refreshInstances)}
	disabled={!selected}
>
	刪除
</button>

<h2>4. Java</h2>
<input type="number" bind:value={javaVersionInput} style="width:4em" />
<button onclick={() => run('detect_java', () => api.detectJava(javaVersionInput)).then((j) => (java = j ?? null))}>
	偵測
</button>
<button onclick={() => run('install_java', () => api.installJava(baseDir, javaVersionInput)).then((j) => j && (java = j))}>
	下載
</button>
<button onclick={() => run('ensure_java', () => api.ensureJava(baseDir, javaVersionInput)).then((j) => j && (java = j))}>
	確保可用
</button>
<p>目前: {java ? `${java.path} (v${java.version})` : '—'}</p>

<h2>5. 帳號</h2>
離線名稱: <input bind:value={playerName} />
<button onclick={msLogin}>Microsoft 登入</button>
{#if account}
	<button onclick={() => run('ms_login_refresh', () => api.msLoginRefresh(account!.refreshToken)).then((a) => a && (account = a))}>
		刷新 token
	</button>
	<button onclick={() => (account = null)}>登出</button>
{/if}
{#if device}
	<p>
		到 <a href={device.verificationUri} target="_blank" rel="noreferrer">{device.verificationUri}</a>
		輸入代碼 <b>{device.userCode}</b>
		<button onclick={() => run('open_external', () => api.openExternal(device!.verificationUri))}>用瀏覽器開</button>
		<button onclick={() => run('ms_login_cancel', () => api.msLoginCancel(device!.sessionId)).then(() => (device = null))}>
			取消
		</button>
	</p>
{/if}
<p>帳號: {account ? `${account.playerName} (${account.uuid})` : `離線 ${playerName}`}</p>

<h2>6. 啟動</h2>
<button onclick={launch} disabled={!selected}>啟動 {selected || ''}</button>
<button onclick={() => run('stop_instance', () => api.stopInstance(selected)).then(refreshRunning)} disabled={!selected}>
	停止
</button>
<button onclick={refreshRunning}>執行中清單</button>
<p>執行中: {running.join(', ') || '—'}</p>

<h2>7. 模組</h2>
<button onclick={() => run('list_mods', () => api.listMods(baseDir, selected)).then((m) => m && (mods = m))} disabled={!selected}>
	列出 ({mods.length})
</button>
<button onclick={() => run('check_mod_updates', () => api.checkModUpdates({ baseDir, instanceName: selected }))} disabled={!selected}>
	檢查更新
</button>
<ul>
	{#each mods as m (m.filePath)}
		<li>
			{m.name} {m.version} [{m.loader}] — {m.fileName}
			<button onclick={() => run('delete_mod', () => api.deleteMod(baseDir, selected, m.fileName))}>刪除</button>
		</li>
	{/each}
</ul>

<h2>8. 資源包 / 光影 / 存檔</h2>
<button onclick={() => run('list_resource_packs', () => api.listResourcePacks(baseDir, selected)).then((p) => p && (resourcePacks = p))} disabled={!selected}>
	資源包 ({resourcePacks.length})
</button>
<button onclick={() => run('list_shader_packs', () => api.listShaderPacks(baseDir, selected)).then((p) => p && (shaderPacks = p))} disabled={!selected}>
	光影 ({shaderPacks.length})
</button>
<button onclick={() => run('list_worlds', () => api.listWorlds(baseDir, selected)).then((w) => w && (worlds = w))} disabled={!selected}>
	存檔 ({worlds.length})
</button>
<ul>
	{#each [...resourcePacks, ...shaderPacks] as p (p.path)}
		<li>{p.name} {p.isFolder ? '(資料夾)' : ''}</li>
	{/each}
	{#each worlds as w (w.path)}
		<li>
			{#if w.icon}<img src={w.icon} alt="" width="16" height="16" />{/if}
			{w.name}
		</li>
	{/each}
</ul>

<h2>Log</h2>
<button onclick={() => (logs = [])}>清空</button>
<pre style="white-space:pre-wrap;max-height:20em;overflow:auto;border:1px solid #ccc">{logs.join('\n')}</pre>
