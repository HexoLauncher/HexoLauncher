<script lang="ts">
	import { Button, Pill } from '$lib/ui';
	import { onDestroy, onMount } from 'svelte';
	import { page } from '$app/state';
	import Icon from '$lib/ui/Icon.svelte';
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import WindowShell from '$lib/components/WindowShell.svelte';
	import * as api from '$lib/api';
	import type { CrashAnalysis, CrashCause, InstanceView } from '$lib/api';
	import { closeSelf, openAppWindow, windowSpecs } from '$lib/windows';
	import { m } from '$lib/paraglide/messages.js';

	const id = $derived(page.params.id ?? '');

	let instance = $state<InstanceView | null>(null);
	let crash = $state<CrashAnalysis | null>(null);
	let loading = $state(true);
	let error = $state('');
	let copied = $state(false);
	let copyTimer: ReturnType<typeof setTimeout> | undefined;
	let unlisten: (() => void) | undefined;

	async function load() {
		loading = true;
		error = '';
		try {
			const paths = await api.resolvePaths();
			[instance, crash] = await Promise.all([
				api.getInstance(paths.baseDir, id).catch(() => null),
				api.crashAnalysis(id)
			]);
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		load();
		api
			.onGameExit((exit) => {
				if (exit.instanceName === id && exit.crashed) load();
			})
			.then((un) => (unlisten = un));
	});

	onDestroy(() => {
		unlisten?.();
		clearTimeout(copyTimer);
	});

	type Explained = { title: string; desc: string; details: string[] };

	function explain(cause: CrashCause): Explained {
		switch (cause.kind) {
			case 'javaTooOld':
				return {
					title: m.crash_cause_java_too_old(),
					desc:
						cause.requiredJava !== null
							? m.crash_cause_java_too_old_desc({ version: cause.requiredJava })
							: m.crash_cause_java_too_old_desc_unknown(),
					details: []
				};
			case 'javaTooNew':
				return { title: m.crash_cause_java_too_new(), desc: m.crash_cause_java_too_new_desc(), details: [] };
			case 'outOfMemory':
				return { title: m.crash_cause_out_of_memory(), desc: m.crash_cause_out_of_memory_desc(), details: [] };
			case 'heapTooLarge':
				return { title: m.crash_cause_heap_too_large(), desc: m.crash_cause_heap_too_large_desc(), details: [] };
			case 'graphicsDriver':
				return { title: m.crash_cause_graphics_driver(), desc: m.crash_cause_graphics_driver_desc(), details: [] };
			case 'missingDependencies':
				return {
					title: m.crash_cause_missing_dependencies(),
					desc: m.crash_cause_missing_dependencies_desc(),
					details: cause.details
				};
			case 'incompatibleMods':
				return {
					title: m.crash_cause_incompatible_mods(),
					desc: m.crash_cause_incompatible_mods_desc(),
					details: cause.details
				};
			case 'duplicateMods':
				return { title: m.crash_cause_duplicate_mods(), desc: m.crash_cause_duplicate_mods_desc(), details: cause.details };
			case 'mixinFailure':
				return { title: m.crash_cause_mixin_failure(), desc: m.crash_cause_mixin_failure_desc(), details: cause.mods };
			case 'corruptedModFile':
				return {
					title: m.crash_cause_corrupted_mod_file(),
					desc: m.crash_cause_corrupted_mod_file_desc(),
					details: cause.files
				};
			case 'corruptedConfig':
				return {
					title: m.crash_cause_corrupted_config(),
					desc: m.crash_cause_corrupted_config_desc(),
					details: cause.files
				};
			case 'suspectedMods':
				return { title: m.crash_cause_suspected_mods(), desc: m.crash_cause_suspected_mods_desc(), details: cause.mods };
			case 'openJ9':
				return { title: m.crash_cause_open_j9(), desc: m.crash_cause_open_j9_desc(), details: [] };
			case 'nativeCrash':
				return {
					title: m.crash_cause_native_crash(),
					desc: m.crash_cause_native_crash_desc(),
					details: cause.frame ? [cause.frame] : []
				};
		}
	}

	const causes = $derived(crash?.causes.map(explain) ?? []);
	const name = $derived(instance?.name ?? id);

	async function copyLog() {
		if (!crash) return;
		try {
			await navigator.clipboard.writeText(crash.logTail.join('\n'));
			copied = true;
			clearTimeout(copyTimer);
			copyTimer = setTimeout(() => (copied = false), 1500);
		} catch (e) {
			error = api.errorMessage(e);
		}
	}

	async function openFile(kind: 'crashReport' | 'jvmReport') {
		try {
			await api.openCrashFile(id, kind);
		} catch (e) {
			error = api.errorMessage(e);
		}
	}
</script>

<svelte:head>
	<title>{m.window_crash({ name })}</title>
</svelte:head>

<WindowShell>
	<div class="flex min-h-0 flex-1 flex-col gap-4 px-5.5 pb-2 pt-5">
		<header class="flex shrink-0 items-center gap-3.25">
			<span class="grid place-items-center rounded-lg bg-card p-1.75 shadow-sm">
				<InstanceIcon name={instance?.icon} image={instance?.iconImage} size={40} />
			</span>
			<div class="min-w-0 flex-1">
				<h1 class="overflow-hidden text-ellipsis whitespace-nowrap text-[15px] font-semibold">
					{m.crash_title()}
				</h1>
				<div class="mt-1.5 flex gap-1.25">
					<Pill accent>{name}</Pill>
					{#if crash?.exitCode !== null && crash?.exitCode !== undefined}
						<Pill>{m.crash_exit_code({ code: crash.exitCode })}</Pill>
					{/if}
				</div>
			</div>
		</header>

		<div class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto pr-1">
			{#if loading}
				<p class="text-muted">{m.crash_loading()}</p>
			{:else if !crash}
				<p class="text-muted">{m.crash_none_recorded()}</p>
			{:else}
				<section class="flex flex-col gap-2">
					<h2 class="font-medium text-muted">{m.crash_causes()}</h2>
					{#each causes as cause, i (i)}
						<div class="rounded-lg border border-line bg-deep px-3.5 py-2.5">
							<p class="flex items-center gap-2 font-medium text-ink">
								<span class="text-danger"><Icon name="info" size={14} /></span>
								{cause.title}
							</p>
							<p class="mt-1 leading-[1.7] text-muted">{cause.desc}</p>
							{#if cause.details.length > 0}
								<ul class="mt-1.5 flex list-none flex-col gap-0.5">
									{#each cause.details as detail (detail)}
										<li class="break-all font-mono text-[11.5px] text-ink">{detail}</li>
									{/each}
								</ul>
							{/if}
						</div>
					{:else}
						<div class="rounded-lg border border-line bg-deep px-3.5 py-2.5">
							<p class="font-medium text-ink">{m.crash_unknown_title()}</p>
							<p class="mt-1 leading-[1.7] text-muted">{m.crash_unknown_desc()}</p>
						</div>
					{/each}
				</section>

				{#if crash.logTail.length > 0}
					<section class="flex min-h-40 flex-1 flex-col gap-2">
						<h2 class="font-medium text-muted">{m.crash_log({ count: crash.logTail.length })}</h2>
						<pre
							class="min-h-0 flex-1 overflow-auto rounded-lg border border-line bg-deep px-3 py-2 font-mono text-[11.5px] leading-[1.6] text-muted">{crash.logTail.join('\n')}</pre>
					</section>
				{/if}
			{/if}

			{#if error}
				<p class="rounded-lg bg-danger-soft px-3 py-2 leading-[1.7] text-danger">{error}</p>
			{/if}
		</div>
	</div>

	{#snippet footer()}
		{#if crash?.crashReport}
			<Button onclick={() => openFile('crashReport')}>{m.crash_open_report()}</Button>
		{/if}
		{#if crash?.jvmReport}
			<Button onclick={() => openFile('jvmReport')}>{m.crash_open_jvm_report()}</Button>
		{/if}
		{#if crash && crash.logTail.length > 0}
			<Button onclick={copyLog}>{copied ? m.crash_copied() : m.crash_copy_log()}</Button>
		{/if}
		{#if instance}
			<Button onclick={() => instance && openAppWindow(windowSpecs.instance(instance.id, instance.name))}>
				{m.crash_edit_instance()}
			</Button>
		{/if}
		<Button variant="primary" onclick={closeSelf}>{m.common_close()}</Button>
	{/snippet}
</WindowShell>
