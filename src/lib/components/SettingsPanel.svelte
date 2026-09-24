<script lang="ts">
	import {
		Button,
		ColorField,
		Field,
		Group,
		Select,
		Slider,
		Table,
		TableBox,
		TextInput
	} from '$lib/ui';
	import NavList from '$lib/ui/NavList.svelte';
	import Switch from '$lib/ui/Switch.svelte';
	import Dialog from '$lib/ui/Dialog.svelte';
	import PlayerHead from '$lib/PlayerHead.svelte';
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
	import { m } from '$lib/paraglide/messages.js';
	import { LOCALE_OPTIONS, storedLocale, useLocale } from '$lib/i18n';
	import type { Locale } from '$lib/paraglide/runtime.js';

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
		{ id: 'launcher', label: m.settings_page_launcher(), icon: 'settings' },
		{ id: 'theme', label: m.settings_page_theme(), icon: 'palette' },
		{ id: 'minecraft', label: m.settings_page_minecraft(), icon: 'box' },
		{ id: 'java', label: 'Java', icon: 'coffee' },
		{ id: 'commands', label: m.settings_page_commands(), icon: 'terminal' },
		{ id: 'accounts', label: m.settings_page_accounts(), icon: 'user' },
		{ id: 'apis', label: m.settings_page_apis(), icon: 'key' }
	];

	// svelte-ignore state_referenced_locally
	let page = $state(initialPage);

	const locale = storedLocale();

	let bgPreview = $state('');
	let broadcastTimer: ReturnType<typeof setTimeout> | undefined;

	$effect(() => {
		if (page !== 'theme') return;
		const theme = $state.snapshot(config.theme);
		applyTheme(theme).then(() => (bgPreview = backgroundUrl()));
	});

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
		report(m.settings_status_accent_reset());
	}

	async function pickBackground() {
		let picked: string | null = null;
		try {
			const { open } = await import('@tauri-apps/api/dialog');
			const chosen = await open({
				filters: [{ name: m.file_filter_image(), extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp'] }]
			});
			picked = typeof chosen === 'string' ? chosen : null;
		} catch {
			report(m.settings_status_no_dialog_image());
			return;
		}
		if (!picked) return;

		try {
			config.theme.background = await api.setBackgroundImage(picked);
			if (config.theme.opacity >= 1) config.theme.opacity = defaultTheme().opacity;
			themeChanged(true);
			report(m.settings_status_background_updated());
		} catch (e) {
			report(m.settings_status_background_failed({ message: api.errorMessage(e) }));
		}
	}

	async function clearBackground() {
		config.theme.background = '';
		bgPreview = '';
		themeChanged(true);
		await api.clearBackgroundImage().catch(() => {});
		report(m.settings_status_background_removed());
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

	function is32bit(arch: string) {
		return !!arch && !['amd64', 'x86_64', 'aarch64', 'arm64'].includes(arch);
	}

	async function detectJava() {
		javaBusy = m.settings_status_java_searching();
		try {
			javaList = await api.detectJavaCandidates();
			report(
				javaList.length
					? m.settings_status_java_found({ count: javaList.length })
					: m.settings_status_java_not_found()
			);
		} catch (e) {
			report(m.settings_status_detect_failed({ message: api.errorMessage(e) }));
		} finally {
			javaBusy = '';
		}
	}

	async function downloadJava(version: number) {
		javaBusy = m.settings_status_java_downloading({ version });
		const unlisten = await api.onJavaProgress((p) => {
			if (p.version !== version || !javaBusy) return;
			javaBusy =
				p.message === 'Downloading Java' && p.total > 0
					? m.settings_status_java_downloading_percent({
							version,
							percent: Math.min(100, Math.round((p.done / p.total) * 100))
						})
					: p.message === 'Extracting Java'
						? m.settings_status_java_extracting({ version })
						: m.settings_status_java_downloading({ version });
		});
		try {
			const java = await api.installJava(paths.baseDir, version);
			report(m.settings_status_java_downloaded({ version: java.version }));
			await detectJava();
		} catch (e) {
			report(m.settings_status_download_failed({ message: api.errorMessage(e) }));
		} finally {
			unlisten();
			javaBusy = '';
		}
	}

	async function testJava() {
		if (!config.java.path) {
			report(m.settings_status_java_auto_no_test());
			return;
		}
		try {
			const java = await api.testJava(config.java.path);
			report(m.settings_status_java_usable({ version: java.version }));
		} catch (e) {
			report(m.settings_status_test_failed({ message: api.errorMessage(e) }));
		}
	}


	async function browseFolder(current: string): Promise<string | null> {
		try {
			const { open } = await import('@tauri-apps/api/dialog');
			const picked = await open({ directory: true, defaultPath: current || undefined });
			return typeof picked === 'string' ? picked : null;
		} catch {
			report(m.common_no_dialog_type_path());
			return null;
		}
	}

	async function pickBaseDir() {
		const picked = await browseFolder(config.baseDir || paths.baseDir);
		if (!picked) return;
		config.baseDir = picked;
		save(true);
		report(m.settings_status_base_dir_changed({ path: picked }));
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
			report(m.common_no_dialog_type_path());
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
			report(m.settings_status_microsoft_added());
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
			report(m.settings_status_action_failed({ action: label, message: api.errorMessage(e) }));
		}
	}

	async function addOffline() {
		const name = offlineDialog?.trim() ?? '';
		offlineDialog = null;
		if (!name) return;
		await accountAction(m.settings_status_offline_added(), () => api.addOfflineAccount(name));
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
			<Button onclick={onBrowse}>{m.common_browse()}</Button>
		</div>
	</Field>
{/snippet}


<div class="flex min-h-0 flex-1 bg-window">
	<NavList {pages} selected={page} onselect={(id) => (page = id)} width="172px" />


	<div
		class="mx-auto flex w-full min-w-0 max-w-205 flex-1 flex-col gap-1.5 overflow-y-auto px-5.5 pb-5.5 pt-2"
	>
		{#if page === 'launcher'}
			<Group title={m.settings_folders()}>
				{@render pathField(
					'dir-base',
					m.settings_base_dir(),
					config.baseDir || paths.baseDir,
					pickBaseDir
				)}
				<div class="flex gap-1.75 px-0.5 pt-1">
					<Button onclick={() => api.openLauncherFolder('base')}>{m.common_open()}</Button>
					<Button onclick={() => api.openLauncherFolder('instances')}>
						{m.folder_open_instances()}
					</Button>
					{#if config.baseDir}
						<Button onclick={() => { config.baseDir = ''; save(true); report(m.settings_status_base_dir_reset()); }}>
							{m.settings_restore_default()}
						</Button>
					{/if}
				</div>
			</Group>

			<Group title={m.settings_launcher_store()}>
				<Field label={m.settings_store_path()} for="store-dir">
					<TextInput id="store-dir" value={paths.storeDir} readonly />
				</Field>
				<div class="px-0.5 pt-1">
					<Button onclick={() => api.openLauncherFolder('store')}>{m.common_open()}</Button>
				</div>
			</Group>

			<Group title={m.settings_ui()}>
				<Field label={m.settings_language()} for="ui-locale">
					<div class="flex items-center gap-2.5">
						<Select
							id="ui-locale"
							style="width:180px"
							value={locale}
							onchange={(e) => useLocale(e.currentTarget.value as Locale | '')}
						>
							<option value="">{m.settings_language_system()}</option>
							{#each LOCALE_OPTIONS as option (option.id)}
								<option value={option.id}>{option.label}</option>
							{/each}
						</Select>
						<span class="text-faint">{m.settings_language_desc()}</span>
					</div>
				</Field>
				<Switch
					label={m.settings_show_instance_toolbar()}
					bind:checked={config.ui.instanceToolbar}
					onchange={() => save(true)}
				/>
				<Switch label={m.settings_show_status_bar()} bind:checked={config.ui.statusBar} onchange={() => save(true)} />
				<Switch
					label={m.settings_show_running_badge()}
					bind:checked={config.ui.runningBadge}
					onchange={() => save(true)}
				/>
			</Group>

		{:else if page === 'theme'}
			<Group title={m.settings_theme()}>
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

			<Group title={m.settings_accent()}>
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
							{m.settings_accent_custom()}
						</span>
					{/if}
				</div>
				{@render colorField('t-accent', m.settings_accent_custom_color(), config.theme.accent, setAccent)}
				{@render slider(
					m.settings_tint(),
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
					<Button onclick={resetColors}>{m.settings_reset_accent()}</Button>
				</div>
			</Group>

			<Group title={m.settings_background()}>
				<div class="flex gap-2.5">
					<div
						class="grid h-27 w-48 shrink-0 place-items-center overflow-hidden rounded-lg border border-line bg-deep"
					>
						{#if bgPreview}
							<img src={bgPreview} alt={m.settings_background_preview()} class="size-full object-cover" />
						{:else}
							<span class="text-faint">{m.settings_background_none()}</span>
						{/if}
					</div>
					<div class="flex w-35 shrink-0 flex-col gap-1 [&>button]:justify-start">
						<Button onclick={pickBackground}>{m.settings_background_pick()}</Button>
						<Button disabled={!config.theme.background} onclick={clearBackground}>
							{m.common_remove()}
						</Button>
					</div>
				</div>

				<div class="pt-3">
					{@render slider(
						m.settings_opacity(),
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
						m.settings_background_dim(),
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
						m.settings_background_blur(),
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
			<Group title={m.settings_launch()}>
				<Switch
					label={m.settings_show_progress_window()}
					bind:checked={config.game.showProgressWindow}
					onchange={() => save(true)}
				/>
				<Switch
					label={m.settings_close_progress_when_done()}
					bind:checked={config.game.closeProgressWhenDone}
					onchange={() => save(true)}
				/>
			</Group>

			<Group title={m.settings_game_log()}>
				<Field label={m.settings_keep_log_lines()} for="history">
					<TextInput id="history" type="number" style="width:110px" bind:value={config.game.keepLogLines} oninput={() => save()} />
				</Field>
			</Group>

			<Group title={m.settings_play_time()}>
				<Switch
					label={m.settings_record_play_time()}
					bind:checked={config.game.recordPlayTime}
					onchange={() => save(true)}
				/>
			</Group>

			<Group title={m.settings_version_list()}>
				<Switch
					label={m.settings_include_snapshots()}
					bind:checked={config.ui.includeSnapshots}
					onchange={() => save(true)}
				/>
			</Group>

		{:else if page === 'java'}
			<Group title={m.settings_memory()}>
				<Field label={m.settings_min_memory()} for="j-min">
					<div class="flex items-center gap-1.75">
						<TextInput id="j-min" type="number" style="width:110px" bind:value={config.game.minMemoryMb} oninput={() => save()} />
						<span class="text-muted">{m.common_mib()}</span>
					</div>
				</Field>
				<Field label={m.settings_max_memory()} for="j-max">
					<div class="flex items-center gap-1.75">
						<TextInput id="j-max" type="number" style="width:110px" bind:value={config.game.maxMemoryMb} oninput={() => save()} />
						<span class="text-muted">{m.common_mib()}</span>
					</div>
				</Field>
			</Group>

			<Group title={m.settings_java_runtime()}>
				<div class="flex gap-2.25">
					<TableBox class="max-h-40">
						<Table>
							<thead>
								<tr>
									<th style="width:90px">{m.common_version()}</th>
									<th style="width:140px">{m.settings_java_vendor()}</th>
									<th>{m.settings_java_path()}</th>
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
											Java {java.version}{is32bit(java.arch) ? ` (${java.arch})` : ''}
										</td>
										<td class="truncate" title={java.vendor}>{java.vendor || m.common_dash()}</td>
										<td class="font-mono text-[11.5px]" title={java.path}>{java.path}</td>
									</tr>
								{/each}
							</tbody>
						</Table>
						{#if javaList.length === 0}
							<p class="px-3 py-4 text-center text-muted">
								{javaBusy || m.settings_java_none_detected()}
							</p>
						{/if}
					</TableBox>
					<div class="flex w-35 shrink-0 flex-col gap-1 [&>button]:justify-start">
						<Button disabled={!!javaBusy} onclick={detectJava}>{m.settings_java_redetect()}</Button>
						<Button disabled={!!javaBusy} onclick={testJava}>{m.settings_java_test()}</Button>
						<Button disabled={!!javaBusy} onclick={() => downloadJava(21)}>
							{m.settings_java_download({ version: 21 })}
						</Button>
						<Button disabled={!!javaBusy} onclick={() => downloadJava(17)}>
							{m.settings_java_download({ version: 17 })}
						</Button>
						<Button disabled={!!javaBusy} onclick={() => downloadJava(8)}>
							{m.settings_java_download({ version: 8 })}
						</Button>
					</div>
				</div>

				<Field label={m.settings_java_default()} for="java-path">
					<div class="flex items-center gap-1.75">
						<TextInput class="min-w-0 flex-1" id="java-path" bind:value={config.java.path} oninput={() => save()} placeholder={m.settings_java_auto_placeholder()} />
						<Button onclick={pickJavaPath}>{m.common_browse()}</Button>
						<Button disabled={!config.java.path} onclick={() => { config.java.path = ''; save(true); }}>
							{m.settings_java_auto()}
						</Button>
					</div>
				</Field>
				{#if javaBusy}
					<p class="px-0.5 py-1 text-muted">{javaBusy}</p>
				{/if}
			</Group>

			<Group title={m.settings_jvm_args()}>
				<Field label={m.settings_jvm_extra_args()} for="jvm">
					<TextInput id="jvm" bind:value={config.game.jvmArgs} oninput={() => save()} placeholder="-XX:+UseG1GC -XX:MaxGCPauseMillis=50" />
				</Field>
			</Group>

		{:else if page === 'commands'}
			<Group title={m.settings_commands()}>
				<Field label={m.settings_pre_launch()} for="c-pre">
					<TextInput id="c-pre" bind:value={config.commands.preLaunch} oninput={() => save()} placeholder={m.settings_command_placeholder()} />
				</Field>
				<Field label={m.settings_post_exit()} for="c-post">
					<TextInput id="c-post" bind:value={config.commands.postExit} oninput={() => save()} />
				</Field>
			</Group>
			<p class="px-0.5 py-1 leading-[1.7] text-faint">
				{m.settings_command_variables()}
				{#each ['$INST_ID', '$INST_NAME', '$INST_DIR', '$INST_MC_DIR', '$INST_JAVA'] as name, i (name)}
					{#if i > 0}{m.common_list_separator()}{/if}
					<code class="rounded-sm border border-line bg-deep px-1 py-px font-mono">{name}</code>
				{/each}
			</p>

		{:else if page === 'accounts'}
			<div class="flex min-h-0 flex-1 gap-2.25">
				<TableBox>
					<Table>
						<thead>
							<tr>
								<th style="width:44px"></th>
								<th style="width:40%">{m.settings_account_name()}</th>
								<th>{m.settings_account_type()}</th>
								<th style="width:110px">{m.settings_account_status()}</th>
							</tr>
						</thead>
						<tbody>
							{#each accounts as item (item.id)}
								<tr
									class:selected={selectedAccountId === item.id}
									onclick={() => (selectedAccountId = item.id)}
								>
									<td><PlayerHead account={item} size={20} /></td>
									<td>{item.playerName}</td>
									<td class="text-muted">{item.kind === 'microsoft' ? m.account_microsoft() : m.account_offline()}</td>
									<td class="text-muted">
										{item.selected
											? m.settings_account_in_use()
											: item.expired
												? m.account_needs_relogin()
												: m.settings_account_signed_in()}
									</td>
								</tr>
							{/each}
						</tbody>
					</Table>
					{#if accounts.length === 0}
						<p class="px-3 py-4 text-center text-muted">{m.settings_account_empty()}</p>
					{/if}
				</TableBox>
				<div class="flex w-35 shrink-0 flex-col gap-1 [&>button]:justify-start">
					<Button onclick={startLogin}>{m.settings_add_microsoft()}</Button>
					<Button onclick={() => (offlineDialog = '')}>{m.settings_add_offline()}</Button>
					<Button disabled={!selectedAccountId} onclick={() => selectedAccountId && accountAction(m.settings_status_account_active(), () => api.selectAccount(selectedAccountId!))}>
						{m.settings_set_active()}
					</Button>
					<Button disabled={!selectedAccountId} onclick={() => selectedAccountId && accountAction(m.settings_status_account_refreshed(), () => api.refreshAccount(selectedAccountId!))}>
						{m.common_refresh()}
					</Button>
					<Button variant="destructive" disabled={!selectedAccountId} onclick={() => selectedAccountId && accountAction(m.settings_status_account_removed(), () => api.removeAccount(selectedAccountId!))}>
						{m.common_remove()}
					</Button>
				</div>
			</div>

		{:else}
			<Group title="CurseForge">
				<Field label={m.settings_api_key()} for="a-cf">
					<TextInput id="a-cf" type="password" bind:value={config.api.curseforgeApiKey} oninput={() => save()} placeholder={m.settings_curseforge_placeholder()} />
				</Field>
			</Group>
		{/if}
	</div>
</div>

{#if login}
	<Dialog title={m.login_title()} width="440px" onclose={cancelLogin}>
		<div class="flex flex-col gap-3 px-5 py-5">
			<p class="text-muted">{m.login_desc()}</p>
			<div
				class="rounded-lg border border-line bg-deep px-4 py-3 text-center font-mono text-[22px] tracking-[4px]"
			>
				{login.userCode}
			</div>
			<Button onclick={() => login && api.openExternal(login.verificationUri)}>
				{m.login_reopen({ url: login.verificationUri })}
			</Button>
			<p class="flex items-center gap-2 text-faint">
				<span class="size-3.5 animate-spin rounded-full border-2 border-line border-t-brand"></span>
				{m.login_waiting()}
			</p>
		</div>
		{#snippet footer()}
			<Button onclick={cancelLogin}>{m.common_cancel()}</Button>
		{/snippet}
	</Dialog>
{/if}

{#if loginError}
	<Dialog title={m.login_failed()} width="420px" onclose={() => (loginError = '')}>
		<p class="px-5 py-5 leading-[1.8] text-danger">{loginError}</p>
		{#snippet footer()}
			<Button variant="primary" onclick={() => (loginError = '')}>{m.common_close()}</Button>
		{/snippet}
	</Dialog>
{/if}

{#if offlineDialog !== null}
	<Dialog title={m.offline_add_title()} width="380px" onclose={() => (offlineDialog = null)}>
		<div class="px-5 py-5">
			<Field label={m.offline_player_name()} for="offline-name">
				<TextInput id="offline-name" bind:value={offlineDialog} maxlength={16} />
			</Field>
			<p class="mt-2 text-faint">{m.offline_name_hint()}</p>
		</div>
		{#snippet footer()}
			<Button variant="primary" disabled={!offlineDialog?.trim()} onclick={addOffline}>
				{m.offline_add()}
			</Button>
			<Button onclick={() => (offlineDialog = null)}>{m.common_cancel()}</Button>
		{/snippet}
	</Dialog>
{/if}
