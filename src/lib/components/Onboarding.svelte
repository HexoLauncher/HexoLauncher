<script lang="ts">
	import { Button, Select, Table, TableBox, TextInput } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import PlayerHead from '$lib/PlayerHead.svelte';
	import Confetti from './Confetti.svelte';
	import * as api from '$lib/api';
	import type {
		DeviceCode,
		DownloadSource,
		ForeignImportResult,
		ForeignInstance,
		ForeignLauncher,
		ImportMode,
		JavaCandidate
	} from '$lib/api';
	import { app, refreshInstances, saveConfig, selectedAccount } from '$lib/state.svelte';
	import { openAppWindow, windowSpecs } from '$lib/windows';
	import { loaderLabel } from '$lib/format';
	import { LOCALE_OPTIONS, storedLocale, useLocale } from '$lib/i18n';
	import type { Locale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { SvelteSet } from 'svelte/reactivity';
	import { onMount } from 'svelte';

	const STEP_KEY = 'nolauncher.onboarding.step';

	const steps = [
		{ id: 'language', label: m.onboarding_step_language(), icon: 'globe' },
		{ id: 'account', label: m.onboarding_step_account(), icon: 'user' },
		{ id: 'java', label: m.onboarding_step_java(), icon: 'coffee' },
		{ id: 'download', label: m.onboarding_step_download(), icon: 'download' },
		{ id: 'import', label: m.onboarding_step_import(), icon: 'upload' },
		{ id: 'done', label: m.onboarding_step_done(), icon: 'party-popper' }
	] as const;

	let step = $state(restoreStep());
	const current = $derived(steps[step].id);
	const locale = storedLocale();
	let error = $state('');

	function restoreStep() {
		try {
			const saved = Number(sessionStorage.getItem(STEP_KEY));
			return Number.isInteger(saved) && saved > 0 && saved < steps.length ? saved : 0;
		} catch {
			return 0;
		}
	}

	function go(next: number) {
		error = '';
		step = Math.max(0, Math.min(steps.length - 1, next));
		try {
			sessionStorage.setItem(STEP_KEY, String(step));
		} catch {
			return;
		}
	}

	$effect(() => {
		if (current === 'java' && !javaScanned) {
			javaScanned = true;
			scanJava();
		}
		if (current === 'import' && !launchersScanned) {
			launchersScanned = true;
			scanLaunchers();
		}
	});

	function pickLocale(value: Locale | '') {
		if (value !== locale) useLocale(value);
	}

	let login = $state<DeviceCode | null>(null);
	let offlineName = $state<string | null>(null);

	async function startLogin() {
		error = '';
		try {
			login = await api.msLoginStart();
			api.openExternal(login.verificationUri).catch(() => {});
			app.accounts = await api.msLoginPoll(login.sessionId);
			login = null;
		} catch (e) {
			if (login) error = api.errorMessage(e);
			login = null;
		}
	}

	async function cancelLogin() {
		const session = login;
		login = null;
		if (session) await api.msLoginCancel(session.sessionId).catch(() => {});
	}

	async function addOffline() {
		const name = offlineName?.trim() ?? '';
		if (!name) return;
		error = '';
		try {
			app.accounts = await api.addOfflineAccount(name);
			offlineName = null;
		} catch (e) {
			error = api.errorMessage(e);
		}
	}

	let javaScanned = false;
	let javaBusy = $state(false);
	let javaList = $state<JavaCandidate[]>([]);

	async function scanJava() {
		javaBusy = true;
		try {
			javaList = await api.detectJavaCandidates();
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			javaBusy = false;
		}
	}

	onMount(() => {
		const unlisten = api.onJavaInstalled(() => {
			if (javaScanned) scanJava();
		});
		return () => {
			unlisten.then((stop) => stop());
		};
	});

	function useJava(path: string) {
		app.config.java.path = path;
		saveConfig();
	}

	const downloadSources: { id: DownloadSource; icon: string; label: string; desc: string }[] = [
		{
			id: 'official',
			icon: 'globe',
			label: m.settings_download_official(),
			desc: m.onboarding_download_official_desc()
		},
		{
			id: 'bmclapi',
			icon: 'server',
			label: m.settings_download_bmclapi(),
			desc: m.onboarding_download_bmclapi_desc()
		},
		{
			id: 'custom',
			icon: 'wrench',
			label: m.settings_download_custom(),
			desc: m.onboarding_download_custom_desc()
		}
	];

	function useDownload(source: DownloadSource) {
		app.config.download.source = source;
		saveConfig();
	}

	let launchersScanned = false;
	let launchers = $state<ForeignLauncher[]>([]);
	let launcherPath = $state('');
	let foreign = $state<ForeignInstance[]>([]);
	const picked = new SvelteSet<string>();
	let importMode = $state<ImportMode>('copy');
	let importBusy = $state('');
	let imported = $state<ForeignImportResult | null>(null);

	async function scanLaunchers() {
		importBusy = m.busy_scanning_launchers();
		try {
			launchers = await api.detectForeignLaunchers();
			if (launchers[0]) await selectLauncher(launchers[0].path);
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			importBusy = '';
		}
	}

	async function selectLauncher(path: string) {
		launcherPath = path;
		picked.clear();
		foreign = [];
		imported = null;
		if (!path.trim()) return;
		error = '';
		importBusy = m.busy_loading_instances();
		try {
			foreign = await api.listForeignInstances(path.trim());
			for (const item of foreign) if (!item.unsupportedLoader) picked.add(item.path);
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			importBusy = '';
		}
	}

	async function browseLauncher() {
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const dir = await open({ directory: true });
			if (typeof dir === 'string') await selectLauncher(dir);
		} catch {
			error = m.common_no_dialog_paste_folder();
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

	async function importSelected() {
		error = '';
		importBusy = importMode === 'link' ? m.busy_linking() : m.busy_copying();
		let unlisten: (() => void) | undefined;
		try {
			unlisten = await api.onInstallProgress((p) => {
				importBusy =
					p.total > 1
						? m.common_progress_count({ message: p.message, done: p.done, total: p.total })
						: p.message;
			});
			const paths = foreign.filter((i) => picked.has(i.path)).map((i) => i.path);
			imported = await api.importForeignInstances({
				baseDir: app.baseDir,
				paths,
				mode: importMode
			});
			const done = new Set(paths);
			for (const failure of imported.failed) done.delete(failure.path);
			foreign = foreign.filter((i) => !done.has(i.path));
			for (const path of done) picked.delete(path);
			await refreshInstances();
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			unlisten?.();
			importBusy = '';
		}
	}

	const busy = $derived(!!login || !!importBusy);

	const required = $derived(
		current === 'account'
			? app.accounts.length === 0
				? m.onboarding_account_required()
				: ''
			: current === 'java'
				? javaBusy || javaList.length === 0
					? m.onboarding_java_required()
					: ''
				: ''
	);
	const skippable = $derived(!['language', 'account', 'java'].includes(current));

	const javaSummary = $derived.by(() => {
		const path = app.config.java.path;
		if (!path) return m.settings_java_auto();
		const found = javaList.find((j) => j.path === path);
		return found ? `Java ${found.version}` : path;
	});

	const downloadSummary = $derived(
		downloadSources.find((s) => s.id === app.config.download.source)?.label ?? ''
	);

	async function finish() {
		await cancelLogin();
		app.config.onboarded = true;
		await saveConfig();
		try {
			sessionStorage.removeItem(STEP_KEY);
		} catch {
			return;
		}
	}
</script>

{#snippet choice(
	selected: boolean,
	icon: string,
	title: string,
	desc: string,
	onclick: () => void,
	mono = false
)}
	<button
		class="flex w-full items-center gap-3 rounded-lg border px-3.5 py-3 text-left transition-colors duration-150
			{selected ? 'border-brand-line bg-brand-soft' : 'border-line hover:border-line-hi hover:bg-hover'}"
		aria-pressed={selected}
		{onclick}
	>
		<span
			class="grid size-8.5 shrink-0 place-items-center rounded-md transition-colors duration-150
				{selected ? 'bg-brand text-brand-ink' : 'bg-btn text-muted'}"
		>
			<Icon name={icon} size={16} />
		</span>
		<span class="min-w-0 flex-1">
			<span class="block font-semibold text-ink">{title}</span>
			{#if desc}
				<span
					class="block truncate leading-[1.6] text-faint {mono ? 'font-mono text-[11.5px]' : ''}"
					title={desc}
				>
					{desc}
				</span>
			{/if}
		</span>
		{#if selected}
			<Icon name="check" size={16} class="shrink-0 text-brand-fg" />
		{/if}
	</button>
{/snippet}

{#snippet heading(title: string)}
	<h2 class="mb-5 text-[20px] font-semibold text-ink">{title}</h2>
{/snippet}

<div class="fixed inset-0 z-40 grid animate-fade place-items-center bg-scrim backdrop-blur-[6px]">
	<div
		class="relative flex h-[min(600px,calc(100vh-48px))] w-[min(900px,calc(100vw-48px))] animate-rise
			overflow-hidden rounded-xl border border-line bg-window shadow-lg"
		role="dialog"
		aria-modal="true"
		aria-label={m.onboarding_title()}
	>
		<aside class="flex w-52 shrink-0 flex-col border-r border-line bg-deep px-4 py-5">
			<div class="mb-6 flex items-center gap-2.5 px-1.5">
				<span class="grid size-8 place-items-center rounded-lg bg-brand text-brand-ink">
					<Icon name="sparkles" size={17} />
				</span>
				<span class="min-w-0">
					<span class="block font-semibold text-ink">{m.app_name()}</span>
					<span class="block text-[11.5px] text-faint">{m.onboarding_title()}</span>
				</span>
			</div>

			<ol class="flex flex-col gap-0.5">
				{#each steps as item, i (item.id)}
					{@const done = i < step || current === 'done'}
					{@const active = i === step}
					<li>
						<button
							class="flex w-full items-center gap-2.5 rounded-md px-2 py-1.75 text-left transition-colors duration-150
								{active ? 'bg-brand-soft text-ink' : done ? 'text-muted hover:text-ink' : 'text-faint'}"
							disabled={busy || i > step || current === 'done'}
							onclick={() => go(i)}
						>
							<span
								class="grid size-5.5 shrink-0 place-items-center rounded-full border text-[11px] font-semibold transition-colors duration-200
									{active
									? 'border-brand bg-brand text-brand-ink'
									: done
										? 'border-brand-line bg-brand-soft text-brand-fg'
										: 'border-line'}"
							>
								{#if done && !active}
									<Icon name="check" size={12} />
								{:else}
									{i + 1}
								{/if}
							</span>
							{item.label}
						</button>
					</li>
				{/each}
			</ol>

		</aside>

		<section class="flex min-w-0 flex-1 flex-col">
			<div class="h-0.75 shrink-0 bg-line">
				<div
					class="h-full bg-brand transition-[width] duration-500 ease-out"
					style:width="{(step / (steps.length - 1)) * 100}%"
				></div>
			</div>

			<div class="flex min-h-0 flex-1 flex-col overflow-y-auto px-8 pt-7 pb-5">
				{#key step}
					<div class="flex min-h-0 flex-1 animate-rise flex-col">
						{#if current === 'language'}
							{@render heading(m.onboarding_welcome_title())}
							<p class="mb-2 font-semibold text-ink">{m.onboarding_language_label()}</p>
							<div class="grid max-w-130 grid-cols-2 gap-2">
								{@render choice(locale === '', 'monitor', m.settings_language_system(), '', () =>
									pickLocale('')
								)}
								{#each LOCALE_OPTIONS as option (option.id)}
									{@render choice(locale === option.id, 'globe', option.label, option.id, () =>
										pickLocale(option.id)
									)}
								{/each}
							</div>
							<p class="mt-3 text-faint">{m.onboarding_language_reload()}</p>
						{:else if current === 'account'}
							{@render heading(m.onboarding_account_title())}
							<div class="flex max-w-130 flex-col gap-2">
								{@render choice(
									!!login,
									'login',
									m.settings_add_microsoft(),
									m.onboarding_account_microsoft_desc(),
									() => (login ? cancelLogin() : startLogin())
								)}
								{#if login}
									<div class="animate-rise rounded-lg border border-line bg-deep px-4 py-3.5">
										<p class="text-muted">{m.login_desc()}</p>
										<div class="my-2.5 text-center font-mono text-[24px] tracking-[5px] text-ink">
											{login.userCode}
										</div>
										<div class="flex items-center gap-2">
											<span class="flex items-center gap-2 text-faint">
												<span
													class="size-3.5 animate-spin rounded-full border-2 border-line border-t-brand"
												></span>
												{m.login_waiting()}
											</span>
											<Button
												class="ml-auto"
												icon="shortcut"
												onclick={() => login && api.openExternal(login.verificationUri)}
											>
												{m.common_open()}
											</Button>
											<Button onclick={cancelLogin}>{m.common_cancel()}</Button>
										</div>
									</div>
								{/if}

								{@render choice(
									offlineName !== null,
									'user',
									m.settings_add_offline(),
									m.onboarding_account_offline_desc(),
									() => (offlineName = offlineName === null ? '' : null)
								)}
								{#if offlineName !== null}
									<form
										class="flex animate-rise items-center gap-2 rounded-lg border border-line bg-deep px-4 py-3"
										onsubmit={(e) => {
											e.preventDefault();
											addOffline();
										}}
									>
										<TextInput
											class="min-w-0 flex-1"
											aria-label={m.offline_player_name()}
											placeholder={m.offline_player_name()}
											maxlength={16}
											bind:value={offlineName}
										/>
										<Button type="submit" variant="primary" disabled={!offlineName.trim()}>
											{m.offline_add()}
										</Button>
									</form>
									<p class="-mt-1 px-1 text-faint">{m.offline_name_hint()}</p>
								{/if}

								{#if app.accounts.length}
									<p class="mt-3 font-semibold text-ink">{m.onboarding_account_added()}</p>
									{#each app.accounts as account (account.id)}
										<div
											class="flex animate-rise items-center gap-2.5 rounded-lg border border-line px-3.5 py-2"
										>
											<PlayerHead {account} size={22} />
											<span class="text-ink">{account.playerName}</span>
											<span class="text-faint">
												{account.kind === 'microsoft' ? m.account_microsoft() : m.account_offline()}
											</span>
											{#if account.selected}
												<Icon name="check" size={15} class="ml-auto text-brand-fg" />
											{/if}
										</div>
									{/each}
								{/if}
							</div>
						{:else if current === 'java'}
							{@render heading(m.onboarding_java_title())}
							<div class="flex max-w-130 flex-col gap-2">
								{@render choice(
									!app.config.java.path,
									'sparkles',
									m.onboarding_java_auto(),
									m.onboarding_java_auto_desc(),
									() => useJava('')
								)}
								<div class="mt-3 flex items-center gap-2">
									<p class="font-semibold text-ink">{m.onboarding_java_found()}</p>
									<Button class="ml-auto" icon="refresh" disabled={javaBusy} onclick={scanJava}>
										{m.settings_java_redetect()}
									</Button>
									<Button icon="download" onclick={() => openAppWindow(windowSpecs.javaDownload())}>
										{m.settings_java_download_open()}
									</Button>
								</div>
								{#if javaBusy}
									<p class="flex items-center gap-2 px-1 py-2 text-faint">
										<span
											class="size-3.5 animate-spin rounded-full border-2 border-line border-t-brand"
										></span>
										{m.onboarding_java_searching()}
									</p>
								{:else if javaList.length === 0}
									<p class="px-1 py-2 leading-[1.7] text-faint">{m.onboarding_java_none()}</p>
								{:else}
									{#each javaList as java (java.path)}
										{@render choice(
											app.config.java.path === java.path,
											'coffee',
											`Java ${java.version}${java.vendor ? ` · ${java.vendor}` : ''}`,
											java.path,
											() => useJava(java.path),
											true
										)}
									{/each}
								{/if}
							</div>
						{:else if current === 'download'}
							{@render heading(m.onboarding_download_title())}
							<div class="flex max-w-130 flex-col gap-2">
								{#each downloadSources as source (source.id)}
									{@render choice(
										app.config.download.source === source.id,
										source.icon,
										source.label,
										source.desc,
										() => useDownload(source.id)
									)}
								{/each}
								{#if app.config.download.source === 'custom'}
									<TextInput
										class="animate-rise"
										aria-label={m.settings_download_custom_url()}
										placeholder="https://mirror.example.com"
										spellcheck={false}
										bind:value={app.config.download.customUrl}
										onchange={() => saveConfig()}
									/>
								{/if}
								<p class="mt-2 leading-[1.7] text-faint">{m.onboarding_download_fallback()}</p>
							</div>
						{:else if current === 'import'}
							{@render heading(m.onboarding_import_title())}
							<div class="flex min-h-0 flex-1 flex-col gap-3">
								<div class="flex gap-1.75">
									<Select
										class="min-w-0 flex-1"
										aria-label={m.add_source()}
										disabled={!!importBusy}
										value={launcherPath}
										onchange={(e) => selectLauncher(e.currentTarget.value)}
									>
										{#if launchers.length === 0 && !launcherPath}
											<option value="">{m.add_launcher_none()}</option>
										{/if}
										{#each launchers as item (item.path)}
											<option value={item.path}>
												{m.add_launcher_option({
													name: item.name,
													count: item.instanceCount,
													path: item.path
												})}
											</option>
										{/each}
										{#if launcherPath && !launchers.some((l) => l.path === launcherPath)}
											<option value={launcherPath}>{launcherPath}</option>
										{/if}
									</Select>
									<Button disabled={!!importBusy} onclick={browseLauncher}>{m.common_browse()}</Button>
								</div>

								{#if foreign.length > 0}
									<TableBox class="min-h-30">
										<Table>
											<thead>
												<tr>
													<th style="width:34px">
														<input
															type="checkbox"
															aria-label={m.add_select_all()}
															checked={picked.size === foreign.length}
															onchange={toggleAll}
														/>
													</th>
													<th>{m.common_name()}</th>
													<th style="width:92px">{m.common_version()}</th>
													<th style="width:128px">{m.common_loader()}</th>
												</tr>
											</thead>
											<tbody>
												{#each foreign as item (item.path)}
													<tr onclick={() => toggle(item.path)}>
														<td>
															<input
																type="checkbox"
																aria-label={item.name}
																checked={picked.has(item.path)}
																onclick={(e) => e.stopPropagation()}
																onchange={() => toggle(item.path)}
															/>
														</td>
														<td class="truncate">{item.name}</td>
														<td class="text-muted">{item.mcVersion || m.common_dash()}</td>
														<td class="text-muted">
															{#if item.unsupportedLoader}
																<span class="text-danger">
																	{m.add_loader_unsupported({ loader: item.unsupportedLoader })}
																</span>
															{:else if item.loader === 'vanilla'}
																{m.common_none()}
															{:else}
																{loaderLabel(item.loader)}
															{/if}
														</td>
													</tr>
												{/each}
											</tbody>
										</Table>
									</TableBox>

									<div class="flex flex-wrap items-center gap-x-5 gap-y-1.5">
										<label class="flex items-center gap-2" title={m.add_mode_copy_desc()}>
											<input type="radio" value="copy" bind:group={importMode} />
											{m.add_mode_copy()}
										</label>
										<label class="flex items-center gap-2" title={m.add_mode_link_desc()}>
											<input type="radio" value="link" bind:group={importMode} />
											{m.add_mode_link()}
										</label>
										<Button
											class="ml-auto"
											variant="primary"
											icon={importMode === 'link' ? 'shortcut' : 'copy'}
											disabled={picked.size === 0 || !!importBusy}
											onclick={importSelected}
										>
											{m.onboarding_import_action({ count: picked.size })}
										</Button>
									</div>
									<p class="-mt-1 leading-[1.6] text-faint">
										{importMode === 'link' ? m.add_mode_link_desc() : m.add_mode_copy_desc()}
									</p>
								{:else if importBusy}
									<p class="flex items-center gap-2 px-1 py-2 text-faint">
										<span
											class="size-3.5 animate-spin rounded-full border-2 border-line border-t-brand"
										></span>
										{importBusy}
									</p>
								{:else if !imported}
									<p class="px-1 py-2 leading-[1.7] text-faint">
										{launcherPath ? m.add_launcher_empty() : m.onboarding_import_none()}
									</p>
								{/if}

								{#if importBusy && foreign.length > 0}
									<p class="flex items-center gap-2 text-faint">
										<span
											class="size-3.5 animate-spin rounded-full border-2 border-line border-t-brand"
										></span>
										{importBusy}
									</p>
								{/if}

								{#if imported}
									<div class="animate-rise rounded-lg border border-line bg-deep px-4 py-3">
										{#if imported.imported.length}
											<p class="flex items-center gap-2 font-semibold text-ink">
												<Icon name="check" size={15} class="text-brand-fg" />
												{m.onboarding_import_done({ count: imported.imported.length })}
											</p>
										{/if}
										{#each imported.failed as failure (failure.path)}
											<p class="mt-1 text-danger">
												{m.onboarding_import_failed({ name: failure.name, message: failure.message })}
											</p>
										{/each}
									</div>
								{/if}
							</div>
						{:else}
							<div class="flex flex-1 flex-col items-center justify-center text-center">
								<span class="celebrate-badge mb-5 grid size-20 place-items-center rounded-full bg-brand text-brand-ink">
									<Icon name="party-popper" size={38} />
								</span>
								<h2 class="celebrate-text text-[24px] font-semibold text-ink">
									{m.onboarding_done_title()}
								</h2>
								<p class="celebrate-text mt-2 text-muted" style:animation-delay="120ms">
									{m.onboarding_done_desc()}
								</p>

								<dl
									class="celebrate-text mt-7 grid w-full max-w-110 grid-cols-2 gap-2 text-left"
									style:animation-delay="240ms"
								>
									{#each [{ icon: 'user', label: m.onboarding_summary_account(), value: selectedAccount()?.playerName ?? m.onboarding_summary_none() }, { icon: 'coffee', label: m.onboarding_summary_java(), value: javaSummary }, { icon: 'download', label: m.onboarding_summary_download(), value: downloadSummary }, { icon: 'box', label: m.onboarding_summary_instances(), value: m.onboarding_summary_instance_count({ count: app.instances.length }) }] as row (row.label)}
										<div class="flex items-center gap-2.5 rounded-lg border border-line bg-deep px-3 py-2.5">
											<Icon name={row.icon} size={16} class="shrink-0 text-brand-fg" />
											<div class="min-w-0">
												<dt class="text-[11.5px] text-faint">{row.label}</dt>
												<dd class="truncate text-ink" title={row.value}>{row.value}</dd>
											</div>
										</div>
									{/each}
								</dl>
							</div>
						{/if}

						{#if error}
							<p class="mt-3 leading-[1.7] text-danger">{error}</p>
						{/if}
					</div>
				{/key}
			</div>

			<footer class="flex shrink-0 items-center gap-2 border-t border-line px-6 py-3.5">
				{#if required}
					<span class="flex items-center gap-1.5 text-warn">
						<Icon name="info" size={14} />
						{required}
					</span>
				{:else}
					<span class="text-faint">
						{m.onboarding_step_count({ current: step + 1, total: steps.length })}
					</span>
				{/if}
				<div class="ml-auto flex gap-2">
					{#if current === 'done'}
						<Button variant="primary" icon="play" onclick={finish}>{m.onboarding_finish()}</Button>
					{:else}
						<Button disabled={step === 0 || busy} onclick={() => go(step - 1)}>
							{m.onboarding_back()}
						</Button>
						{#if skippable}
							<Button disabled={busy} onclick={() => go(step + 1)}>{m.onboarding_skip()}</Button>
						{/if}
						<Button variant="primary" disabled={busy || !!required} onclick={() => go(step + 1)}>
							{m.onboarding_next()}
						</Button>
					{/if}
				</div>
			</footer>
		</section>

		{#if current === 'done'}
			<Confetti />
		{/if}
	</div>
</div>

<style>
	.celebrate-badge {
		animation: celebrate-pop 0.7s cubic-bezier(0.34, 1.56, 0.64, 1) both;
		box-shadow: 0 0 0 0 var(--color-brand-soft-hi);
	}

	.celebrate-text {
		animation: celebrate-rise 0.5s cubic-bezier(0.4, 0, 0.2, 1) 0.25s both;
	}

	@keyframes celebrate-pop {
		0% {
			transform: scale(0) rotate(-30deg);
			opacity: 0;
		}
		60% {
			transform: scale(1.15) rotate(8deg);
			opacity: 1;
			box-shadow: 0 0 0 0 var(--color-brand-soft-hi);
		}
		100% {
			transform: scale(1) rotate(0);
			box-shadow: 0 0 0 18px transparent;
		}
	}

	@keyframes celebrate-rise {
		from {
			opacity: 0;
			transform: translateY(10px);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.celebrate-badge,
		.celebrate-text {
			animation: none;
		}
	}
</style>
