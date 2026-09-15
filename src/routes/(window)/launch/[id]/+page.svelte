<script lang="ts">
	import { onDestroy } from 'svelte';
	import { page } from '$app/state';
	import Icon from '$lib/Icon.svelte';
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import WindowShell from '$lib/components/WindowShell.svelte';
	import * as api from '$lib/api';
	import type { InstanceView, LauncherConfig } from '$lib/api';
	import { formatSize, loaderLabel } from '$lib/format';
	import { closeSelf } from '$lib/windows';

	/** 啟動流程圖：一步一步顯示檢查與下載的進度，遊戲輸出不在這裡。 */
	const id = $derived(page.params.id ?? '');

	type StageId = 'read' | 'check' | 'download' | 'java' | 'launch';
	type StageState = 'pending' | 'active' | 'done' | 'skipped' | 'failed';

	const STAGES: { id: StageId; label: string; icon: string }[] = [
		{ id: 'read', label: '讀取實例資訊', icon: 'box' },
		{ id: 'check', label: '檢查遊戲檔案', icon: 'search' },
		{ id: 'download', label: '下載遊戲檔案', icon: 'package' },
		{ id: 'java', label: '準備 Java 與啟動參數', icon: 'coffee' },
		{ id: 'launch', label: '啟動遊戲', icon: 'play' }
	];

	/** 下載清單的一列；同一個訊息會更新既有的那列，不會一直往下長 */
	type Task = { message: string; done: number; total: number; finished: boolean };

	let instance = $state<InstanceView | null>(null);
	let config = $state<LauncherConfig | null>(null);

	let stageState = $state<Record<StageId, StageState>>(reset());
	let detail = $state<Record<string, string>>({});
	let tasks = $state<Task[]>([]);
	let error = $state('');
	let done = $state(false);

	const unlisteners: Array<() => void> = [];
	let closeTimer: ReturnType<typeof setTimeout> | undefined;
	let started = false;


	const TOOL_JAVA_VERSION = 21;


	let javaPrompt = $state<{ version: number; answer: (download: boolean) => void } | null>(null);

	function askDownloadJava(version: number) {
		return new Promise<boolean>((resolve) => {
			javaPrompt = {
				version,
				answer: (download) => {
					javaPrompt = null;
					resolve(download);
				}
			};
		});
	}


	let javaDownload = $state<{ label: string; percent: number | null } | null>(null);

	function describeJavaProgress(p: api.JavaProgress) {
		switch (p.message) {
			case 'Fetching Java release':
				return { label: `取得 Java ${p.version} 的下載資訊…`, percent: null };
			case 'Downloading Java':
				return p.total > 0
					? {
							label: `下載 Java ${p.version}：${formatSize(p.done)} / ${formatSize(p.total)}`,
							percent: Math.min(100, Math.round((p.done / p.total) * 100))
						}
					: { label: `下載 Java ${p.version}：${formatSize(p.done)}`, percent: null };
			case 'Extracting Java':
				return { label: `解壓縮 Java ${p.version}…`, percent: null };
			default:
				return { label: `準備 Java ${p.version}…`, percent: null };
		}
	}

	async function downloadJava(baseDir: string, version: number, stage: StageId) {
		if (!(await askDownloadJava(version))) {
			throw new Error(`找不到 Java ${version}，已取消。可以到「設定」指定已安裝的 Java。`);
		}
		javaDownload = { label: `準備下載 Java ${version}…`, percent: null };
		const unlisten = await api.onJavaProgress((p) => {
			if (p.version === version) javaDownload = describeJavaProgress(p);
		});
		try {
			await api.installJava(baseDir, version);
		} finally {
			unlisten();
			javaDownload = null;
		}
		detail[stage] = '';
	}

	function isJavaMissing(e: unknown) {
		return !!e && typeof e === 'object' && (e as api.HexoCmdError).kind === 'javaNotFound';
	}

	const failed = $derived(Object.values(stageState).includes('failed'));
	const percent = $derived.by(() => {
		const current = tasks.find((t) => !t.finished);
		if (!current || current.total <= 0) return 0;
		return Math.min(100, Math.round((current.done / current.total) * 100));
	});

	function reset(): Record<StageId, StageState> {
		return { read: 'pending', check: 'pending', download: 'pending', java: 'pending', launch: 'pending' };
	}

	$effect(() => {
		// 路由參數要等 SvelteKit 準備好才有值，所以在 effect 裡起跑，而且只跑一次。
		if (!id || started) return;
		started = true;
		run(id);
	});

	function track(message: string, count: number, total: number) {
		const existing = tasks.find((t) => t.message === message);
		if (existing) {
			existing.done = count;
			existing.total = total;
			return;
		}
		// 新的階段開始，前面的就算做完了。
		for (const task of tasks) task.finished = true;
		tasks.push({ message, done: count, total, finished: false });
	}

	async function run(instanceName: string) {
		error = '';
		done = false;
		tasks = [];
		stageState = reset();
		detail = {};

		let stage: StageId = 'read';
		try {
			stageState.read = 'active';
			const [paths, launcherConfig] = await Promise.all([
				api.resolvePaths(),
				api.getLauncherConfig()
			]);
			config = launcherConfig;
			const baseDir = paths.baseDir;

			instance = await api.getInstance(baseDir, instanceName);
			stageState.read = 'done';
			detail.read = `${instance.versionId} · ${loaderLabel(instance.loader)}`;

			stage = 'check';
			stageState.check = 'active';
			const needsDownload = !instance.healthy;
			stageState.check = 'done';
			detail.check = needsDownload
				? instance.installed
					? '檔案不完整'
					: '尚未下載'
				: '檔案齊全';

			stage = 'download';
			if (needsDownload) {
				stageState.download = 'active';
				unlisteners.push(
					await api.onInstallProgress((p) => {
						if (p.instanceName !== instanceName) return;
						track(p.message, p.done, p.total);
					})
				);
				const repair = () => api.repairInstance(baseDir, instanceName);
				instance = await repair().catch(async (e) => {
					if (!isJavaMissing(e)) throw e;

					await downloadJava(baseDir, TOOL_JAVA_VERSION, 'download');
					return repair();
				});
				for (const task of tasks) task.finished = true;
				stageState.download = 'done';
				detail.download = `${tasks.length} 個步驟`;
			} else {
				stageState.download = 'skipped';
				detail.download = '略過';
			}

			stage = 'java';
			stageState.java = 'active';

			const check = await api.checkInstanceJava(baseDir, instanceName);
			if (!check.java) await downloadJava(baseDir, check.required, 'java');
			const info = await api.launchInstance({ baseDir, instanceName });

			stageState.java = 'done';
			detail.java = `Java ${info.javaVersion} · ${info.online ? '正版登入' : '離線模式'} · ${info.playerName}`;
			stageState.launch = 'done';
			detail.launch = `PID ${info.pid}`;
			done = true;

			if (config?.game.closeProgressWhenDone) {
				closeTimer = setTimeout(closeSelf, 1200);
			}
		} catch (e) {
			stageState[stage] = 'failed';
			error = api.errorMessage(e);
		}
	}

	onDestroy(() => {
		clearTimeout(closeTimer);
		unlisteners.forEach((un) => un());
	});
</script>

<svelte:head>
	<title>{instance ? `啟動 ${instance.name}` : '正在啟動'}</title>
</svelte:head>

{#snippet marker(phase: StageState, icon: string)}
	<span
		class="grid size-6 shrink-0 place-items-center rounded-full border transition-colors duration-150
			{phase === 'done'
			? 'border-brand-line bg-brand-soft text-brand'
			: phase === 'active'
				? 'border-brand bg-brand text-brand-ink'
				: phase === 'failed'
					? 'border-danger bg-danger-soft text-danger'
					: 'border-line bg-deep text-faint'}"
	>
		{#if phase === 'done'}
			<Icon name="check" size={12} />
		{:else if phase === 'active'}
			<span class="size-3 animate-spin rounded-full border-2 border-brand-ink/40 border-t-brand-ink"
			></span>
		{:else if phase === 'failed'}
			<Icon name="close" size={12} />
		{:else if phase === 'skipped'}
			<span class="h-px w-2.5 bg-current"></span>
		{:else}
			<Icon name={icon} size={12} />
		{/if}
	</span>
{/snippet}

<WindowShell>
	<div class="flex min-h-0 flex-1 flex-col gap-4 px-5.5 pb-2 pt-5">
		<header class="flex shrink-0 items-center gap-3.25">
			<span class="grid place-items-center rounded-lg bg-card p-1.75 shadow-sm">
				<InstanceIcon name={instance?.icon ?? 'grass'} size={40} />
			</span>
			<div class="min-w-0 flex-1">
				<h1 class="overflow-hidden text-ellipsis whitespace-nowrap text-[15px] font-semibold">
					{instance?.name ?? '正在讀取…'}
				</h1>
				<div class="mt-1.5 flex gap-1.25">
					<span class="pill accent">{instance?.versionId ?? '—'}</span>
					<span class="pill">{instance ? loaderLabel(instance.loader) : '—'}</span>
				</div>
			</div>
			{#if done}
				<span class="pill accent">已啟動</span>
			{:else if failed}
				<span class="pill">失敗</span>
			{/if}
		</header>

		<!-- 流程圖：左邊是節點與連接線，右邊是每一步的細節 -->
		<ol class="min-h-0 flex-1 list-none overflow-y-auto pr-1">
			{#each STAGES as stage, i (stage.id)}
				{@const current = stageState[stage.id]}
				<li class="flex gap-3">
					<div class="flex w-6 shrink-0 flex-col items-center">
						{@render marker(current, stage.icon)}
						{#if i < STAGES.length - 1}
							<span
								class="w-0.5 flex-1 rounded-full transition-colors duration-150
									{current === 'done' || current === 'skipped' ? 'bg-brand-line' : 'bg-line'}"
							></span>
						{/if}
					</div>

					<div class="min-w-0 flex-1 {i === STAGES.length - 1 ? 'pb-0' : 'pb-4'}">
						<div class="flex items-baseline justify-between gap-3">
							<span
								class="font-medium {current === 'pending'
									? 'text-faint'
									: current === 'failed'
										? 'text-danger'
										: current === 'active'
											? 'text-ink'
											: 'text-muted'}"
							>
								{stage.label}
							</span>
							<span class="shrink-0 tabular-nums text-faint">
								{current === 'active' && stage.id === 'download' ? `${percent}%` : (detail[stage.id] ?? '')}
							</span>
						</div>

						{#if stage.id === 'download' && tasks.length > 0}
							<div class="mt-2 rounded-lg border border-line bg-deep px-3 py-2">
								{#if current === 'active'}
									<div class="mb-2 h-1.5 overflow-hidden rounded-full bg-window">
										<span
											class="block h-full rounded-full bg-brand transition-[width] duration-100 ease-linear"
											style:width={`${percent}%`}
										></span>
									</div>
								{/if}
								{#each tasks as task (task.message)}
									<div
										class="flex items-center gap-2 py-0.75 {task.finished
											? 'text-muted'
											: 'text-ink'}"
									>
										<span class="grid size-3.5 shrink-0 place-items-center">
											{#if task.finished}
												<span class="text-brand"><Icon name="check" size={10} /></span>
											{:else}
												<span
													class="size-2.5 animate-spin rounded-full border-[1.5px] border-line border-t-brand"
												></span>
											{/if}
										</span>
										<span class="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap">
											{task.message}
										</span>
										{#if task.total > 1}
											<span class="shrink-0 tabular-nums text-faint">
												{task.done.toLocaleString()} / {task.total.toLocaleString()}
											</span>
										{/if}
									</div>
								{/each}
							</div>
						{/if}

						{#if current === 'active' && javaPrompt}
							{@const prompt = javaPrompt}
							<div class="mt-2 rounded-lg border border-line bg-deep px-3 py-2.5">
								<p class="leading-[1.7] text-ink">找不到 Java {prompt.version}。</p>
								<p class="leading-[1.7] text-muted">
									要從 Adoptium 下載 Temurin {prompt.version} 嗎？也可以取消，到「設定」指定已安裝的 Java。
								</p>
								<div class="mt-2 flex gap-1.75">
									<button class="qt-button primary" onclick={() => prompt.answer(true)}>
										<Icon name="download" size={13} />下載 Java {prompt.version}
									</button>
									<button class="qt-button" onclick={() => prompt.answer(false)}>取消</button>
								</div>
							</div>
						{/if}

						{#if current === 'active' && javaDownload}
							<div class="mt-2 rounded-lg border border-line bg-deep px-3 py-2">
								{#if javaDownload.percent !== null}
									<div class="mb-2 h-1.5 overflow-hidden rounded-full bg-window">
										<span
											class="block h-full rounded-full bg-brand transition-[width] duration-100 ease-linear"
											style:width={`${javaDownload.percent}%`}
										></span>
									</div>
								{/if}
								<div class="flex items-center gap-2 text-ink">
									<span
										class="size-2.5 shrink-0 animate-spin rounded-full border-[1.5px] border-line border-t-brand"
									></span>
									<span class="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap">
										{javaDownload.label}
									</span>
									{#if javaDownload.percent !== null}
										<span class="shrink-0 tabular-nums text-faint">{javaDownload.percent}%</span>
									{/if}
								</div>
							</div>
						{/if}

						{#if current === 'failed' && error}
							<p class="mt-2 rounded-lg bg-danger-soft px-3 py-2 leading-[1.7] text-danger">
								{error}
							</p>
						{/if}
					</div>
				</li>
			{/each}
		</ol>
	</div>

	{#snippet footer()}
		{#if failed}
			<button class="qt-button primary" onclick={() => run(id)}>重試</button>
			<button class="qt-button" onclick={closeSelf}>關閉</button>
		{:else if done}
			<button class="qt-button primary" onclick={closeSelf}>關閉</button>
		{:else}
			<button class="qt-button" onclick={closeSelf}>在背景繼續</button>
		{/if}
	{/snippet}
</WindowShell>
