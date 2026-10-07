<script lang="ts">
	import { Button, Pill } from '$lib/ui';
	import { onDestroy } from 'svelte';
	import { page } from '$app/state';
	import Icon from '$lib/ui/Icon.svelte';
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import WindowShell from '$lib/components/WindowShell.svelte';
	import * as api from '$lib/api';
	import type { InstanceView, LauncherConfig } from '$lib/api';
	import { formatSize, loaderLabel } from '$lib/format';
	import { closeSelf } from '$lib/windows';
	import { m } from '$lib/paraglide/messages.js';


	const id = $derived(page.params.id ?? '');

	type StageId = 'read' | 'check' | 'download' | 'java' | 'launch';
	type StageState = 'pending' | 'active' | 'done' | 'skipped' | 'failed';

	const STAGES: { id: StageId; label: string; icon: string }[] = [
		{ id: 'read', label: m.launch_stage_read(), icon: 'box' },
		{ id: 'check', label: m.launch_stage_check(), icon: 'search' },
		{ id: 'download', label: m.launch_stage_download(), icon: 'package' },
		{ id: 'java', label: m.launch_stage_java(), icon: 'coffee' },
		{ id: 'launch', label: m.launch_stage_launch(), icon: 'play' }
	];


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
				return { label: m.launch_java_fetching({ version: p.version }), percent: null };
			case 'Downloading Java':
				return p.total > 0
					? {
							label: m.launch_java_downloading({
								version: p.version,
								done: formatSize(p.done),
								total: formatSize(p.total)
							}),
							percent: Math.min(100, Math.round((p.done / p.total) * 100))
						}
					: {
							label: m.launch_java_downloading_unknown({
								version: p.version,
								done: formatSize(p.done)
							}),
							percent: null
						};
			case 'Extracting Java':
				return { label: m.launch_java_extracting({ version: p.version }), percent: null };
			default:
				return { label: m.launch_java_preparing({ version: p.version }), percent: null };
		}
	}

	async function downloadJava(baseDir: string, version: number, stage: StageId) {
		if (!(await askDownloadJava(version))) {
			throw new Error(m.launch_java_cancelled({ version }));
		}
		javaDownload = { label: m.launch_java_preparing_download({ version }), percent: null };
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
		return !!e && typeof e === 'object' && (e as api.NolauncherCmdError).kind === 'javaNotFound';
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
			detail.read = m.instance_tooltip_version({
				version: instance.versionId,
				loader: loaderLabel(instance.loader)
			});

			stage = 'check';
			stageState.check = 'active';
			const needsDownload = !instance.healthy;
			stageState.check = 'done';
			detail.check = needsDownload
				? instance.installed
					? m.launch_detail_files_incomplete()
					: m.launch_detail_not_downloaded()
				: m.launch_detail_files_ready();

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
				detail.download = m.launch_detail_steps({ count: tasks.length });
			} else {
				stageState.download = 'skipped';
				detail.download = m.launch_detail_skipped();
			}

			stage = 'java';
			stageState.java = 'active';

			const check = await api.checkInstanceJava(baseDir, instanceName);
			if (!check.java) await downloadJava(baseDir, check.required, 'java');
			const info = await api.launchInstance({ baseDir, instanceName });

			stageState.java = 'done';
			detail.java = m.launch_detail_java({
				version: info.javaVersion,
				mode: info.online ? m.editor_online_mode() : m.editor_offline_mode(),
				player: info.playerName
			});
			stageState.launch = 'done';
			detail.launch = m.launch_detail_pid({ pid: info.pid });
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
	<title>{instance ? m.window_launch({ name: instance.name }) : m.window_launching()}</title>
</svelte:head>

{#snippet marker(phase: StageState, icon: string)}
	<span
		class="grid size-6 shrink-0 place-items-center rounded-full border transition-colors duration-150
			{phase === 'done'
			? 'border-brand-line bg-brand-soft text-brand-fg'
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
				<InstanceIcon name={instance?.icon} image={instance?.iconImage} size={40} />
			</span>
			<div class="min-w-0 flex-1">
				<h1 class="overflow-hidden text-ellipsis whitespace-nowrap text-[15px] font-semibold">
					{instance?.name ?? m.launch_loading()}
				</h1>
				<div class="mt-1.5 flex gap-1.25">
					<Pill accent>{instance?.versionId ?? m.common_dash()}</Pill>
					<Pill>{instance ? loaderLabel(instance.loader) : m.common_dash()}</Pill>
				</div>
			</div>
			{#if done}
				<Pill accent>{m.launch_started()}</Pill>
			{:else if failed}
				<Pill>{m.launch_failed()}</Pill>
			{/if}
		</header>


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
												<span class="text-brand-fg"><Icon name="check" size={10} /></span>
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
								<p class="leading-[1.7] text-ink">{m.launch_java_missing({ version: prompt.version })}</p>
								<p class="leading-[1.7] text-muted">
									{m.launch_java_ask({ version: prompt.version })}
								</p>
								<div class="mt-2 flex gap-1.75">
									<Button variant="primary" onclick={() => prompt.answer(true)}>
										<Icon name="download" size={13} />{m.launch_java_download({ version: prompt.version })}
									</Button>
									<Button onclick={() => prompt.answer(false)}>{m.common_cancel()}</Button>
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
			<Button variant="primary" onclick={() => run(id)}>{m.common_retry()}</Button>
			<Button onclick={closeSelf}>{m.common_close()}</Button>
		{:else if done}
			<Button variant="primary" onclick={closeSelf}>{m.common_close()}</Button>
		{:else}
			<Button onclick={closeSelf}>{m.launch_background()}</Button>
		{/if}
	{/snippet}
</WindowShell>
