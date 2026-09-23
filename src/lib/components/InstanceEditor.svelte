<script lang="ts">
	import { Button, CheckRow, Field, Group, Menu, Pill, SearchField, Select, Table, TableBox, TextInput, Textarea } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import WindowShell from './WindowShell.svelte';
	import NavList from '$lib/ui/NavList.svelte';
	import Switch from '$lib/ui/Switch.svelte';
	import Dialog from '$lib/ui/Dialog.svelte';
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
		ModpackUpdateResult,
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
	import { m } from '$lib/paraglide/messages.js';

	let {
		baseDir,
		instance,
		onclose,
		onchanged
	}: {
		baseDir: string;
		instance: InstanceView;
		onclose: () => void;
		onchanged: (instance: InstanceView) => void;
	} = $props();

	const pages: Page[] = [
		{ id: 'general', label: m.editor_page_general(), icon: 'box' },
		{ id: 'console', label: m.editor_page_console(), icon: 'terminal' },
		{ id: 'version', label: m.editor_page_version(), icon: 'package' },
		{ id: 'mods', label: m.editor_page_mods(), icon: 'package' },
		{ id: 'resourcepacks', label: m.editor_page_resourcepacks(), icon: 'image' },
		{ id: 'shaderpacks', label: m.editor_page_shaderpacks(), icon: 'sun' },
		{ id: 'worlds', label: m.editor_page_worlds(), icon: 'globe' },
		{ id: 'servers', label: m.editor_page_servers(), icon: 'server' },
		{ id: 'screenshots', label: m.editor_page_screenshots(), icon: 'image' },
		{ id: 'notes', label: m.editor_page_notes(), icon: 'notes' },
		{ id: 'logs', label: m.editor_page_logs(), icon: 'list' },
		{ id: 'settings', label: m.editor_page_settings(), icon: 'settings' }
	];

	const BROWSE_TITLES: Record<ContentKind, string> = {
		mod: m.editor_browse_mod(),
		resourcepack: m.editor_browse_resourcepack(),
		shader: m.editor_browse_shader()
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

	let packUpdate = $state<ModpackUpdateResult | null>(null);


	let browse = $state<ContentKind | null>(null);

	const title = $derived(pages.find((p) => p.id === page)?.label ?? '');

	function report(message: string) {
		status = message;
		setTimeout(() => {
			if (status === message) status = '';
		}, 5000);
	}

	function fail(prefix: string, e: unknown) {
		status = m.common_error_detail({ prefix, message: api.errorMessage(e) });
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
			fail(m.editor_error_load(), e);
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
			fail(m.editor_error_output(), e);
		}
	}

	async function refreshRunning() {
		try {
			const list = await api.runningInstances();
			running = list.find((r) => r.instanceName === instance.id) ?? null;
		} catch (e) {
			fail(m.editor_error_running(), e);
		}
	}

	async function stopGame() {
		try {
			await api.stopInstance(instance.id);
			report(m.editor_status_stop_requested());
		} catch (e) {
			fail(m.editor_error_stop(), e);
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
			fail(m.editor_error_clear(), e);
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
			fail(m.editor_error_read_log(), e);
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
			report(m.editor_status_saved());
		} catch (e) {
			fail(m.editor_error_save(), e);
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
			fail(m.editor_error_open_folder(), e);
		}
	}


	async function toggleMod(mod: ModEntry) {
		try {
			await api.setModEnabled(baseDir, instance.id, mod.fileName, !mod.enabled);
			await load('mods', true);
		} catch (e) {
			fail(m.editor_error_toggle_mod(), e);
		}
	}

	async function removeMod(fileName: string) {
		try {
			await api.deleteMod(baseDir, instance.id, fileName);
			await load('mods', true);
			report(m.editor_status_mod_removed({ file: fileName }));
		} catch (e) {
			fail(m.editor_error_remove_mod(), e);
		}
	}

	async function checkUpdates() {
		busy = m.editor_busy_check_updates();
		try {
			modUpdates = await api.checkModUpdates({ baseDir, instanceName: instance.id });
			report(
				modUpdates.length
					? m.editor_status_updates_found({ count: modUpdates.length })
					: m.editor_status_up_to_date()
			);
		} catch (e) {
			fail(m.editor_error_check_updates(), e);
		} finally {
			busy = '';
		}
	}

	async function applyUpdate(update: ModUpdateEntry) {
		busy = m.editor_busy_update_mod({ name: update.name });
		try {
			await api.applyModUpdate(baseDir, instance.id, update.id);
			modUpdates = modUpdates.filter((u) => u.id !== update.id);
			await load('mods', true);
			report(m.editor_status_mod_updated({ name: update.name, version: update.newVersion }));
		} catch (e) {
			fail(m.editor_error_update(), e);
		} finally {
			busy = '';
		}
	}


	async function reinstall(request?: { mcVersion: string; loader: LoaderKind; loaderVersion: string }) {
		versionDialog = null;
		busy = request ? m.editor_busy_install_version() : m.editor_busy_repair();

		let unlisten: (() => void) | undefined;
		try {
			unlisten = await api.onInstallProgress((p) => {
				if (p.instanceName === instance.id) {
					busy =
							p.total > 1
								? m.common_progress_count({ message: p.message, done: p.done, total: p.total })
								: p.message;
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
			report(m.editor_status_done());
		} catch (e) {
			fail(m.editor_error_install(), e);
		} finally {
			unlisten?.();
			busy = '';
		}
	}

	async function pickPack() {
		try {
			const { open } = await import('@tauri-apps/api/dialog');
			const picked = await open({
				filters: [{ name: m.file_filter_modpack(), extensions: ['mrpack', 'zip'] }]
			});
			return typeof picked === 'string' ? picked : null;
		} catch {
			status = m.editor_error_no_dialog_pack();
			return null;
		}
	}

	async function updatePack() {
		const packPath = await pickPack();
		if (!packPath) return;

		busy = m.editor_busy_update_pack();
		let unlisten: (() => void) | undefined;
		try {
			unlisten = await api.onInstallProgress((p) => {
				if (p.instanceName === instance.id) {
					busy =
							p.total > 1
								? m.common_progress_count({ message: p.message, done: p.done, total: p.total })
								: p.message;
				}
			});

			const result = await api.updateModpackFile(baseDir, instance.id, packPath);
			onchanged(result.instance);
			packUpdate = result;
			report(m.editor_status_pack_updated());
		} catch (e) {
			fail(m.editor_error_update_pack(), e);
		} finally {
			unlisten?.();
			busy = '';
		}
	}

	const versionRows = $derived([
		{
			name: 'Minecraft',
			version: instance.versionId || m.editor_version_unknown(),
			note: m.editor_version_required()
		},
		...(instance.loader !== 'vanilla'
			? [
					{
						name: `${loaderLabel(instance.loader)} Loader`,
						version: instance.loaderVersion || m.editor_version_unrecorded(),
						note: m.editor_version_loader_note()
					}
				]
			: []),
		{
			name: 'Java',
			version: m.editor_version_java({ version: instance.javaVersion }),
			note: m.editor_version_runtime()
		},
		{
			name: m.editor_version_assets(),
			version: instance.assetsId || m.common_dash(),
			note: m.editor_version_assets_note()
		},
		{
			name: m.editor_version_main_class(),
			version: instance.mainClass || m.common_dash(),
			note: m.editor_version_main_class_note()
		}
	]);
</script>

{#snippet sideButtons(items: { label: string; disabled?: boolean; action: () => void }[])}
	<div class="flex w-34.5 shrink-0 flex-col gap-1 [&>button]:justify-start">
		{#each items as item (item.label)}
			<Button disabled={item.disabled} onclick={item.action}>
				{item.label}
			</Button>
		{/each}
	</div>
{/snippet}

<WindowShell>
	<div class="flex min-h-0 flex-1">
		<NavList {pages} selected={page} onselect={(id) => (page = id)} />

		<div class="flex min-w-0 flex-1 flex-col">
			<div
				class="flex shrink-0 items-center gap-2.75 border-b border-line px-4.5 pb-3.25 pt-3.75 text-brand-fg"
			>
				<Icon name={pages.find((p) => p.id === page)?.icon ?? 'box'} size={19} />
				<h3>{title}</h3>
				{#if running}
					<span class="ml-auto text-[12px] text-brand-fg">{m.info_running_pid({ pid: running.pid })}</span>
				{:else if !instance.healthy}
					<span class="ml-auto text-[12px] text-warn">{m.editor_needs_repair()}</span>
				{/if}
			</div>

			<div class="flex min-h-0 flex-1 flex-col gap-2.5 px-4.5 pb-4.5 pt-3.5">
				{#if page === 'general'}
					<div class="flex items-start gap-3">
						<Menu width="228px">
							{#snippet trigger(attrs)}
								<Button {...attrs} class="size-14.5 rounded-lg p-0" aria-label={m.common_select_icon()}>
									<InstanceIcon name={form.icon} size={34} />
								</Button>
							{/snippet}
							<div class="grid grid-cols-5 gap-0.75 p-0.75">
								{#each ICON_CHOICES as choice (choice)}
									<button
										class="grid place-items-center rounded-sm border border-transparent p-1.25 hover:bg-hover {form.icon ===
										choice
											? 'border-brand bg-brand-soft'
											: ''}"
										aria-label={choice}
										onclick={() => (form.icon = choice)}
									>
										<InstanceIcon name={choice} size={30} />
									</button>
								{/each}
							</div>
						</Menu>

						<div class="min-w-0 flex-1 [--field-label:64px]">
							<Field label={m.common_name()} for="edit-name">
								<TextInput id="edit-name" bind:value={form.name} />
							</Field>
							<Field label={m.common_group()} for="edit-group">
								<TextInput id="edit-group" bind:value={form.group} placeholder={m.common_ungrouped()} />
							</Field>
						</div>
					</div>

					<Group title={m.editor_state()}>
						<Field label={m.editor_folder_name()} for="s1">
<span id="s1">{instance.id}</span>
						</Field>
						<Field label={m.editor_location()} for="s2">
							<span id="s2" class="overflow-hidden text-ellipsis font-mono text-[11.5px] text-muted"
								>{instance.dir}</span
							>
						</Field>
						<Field label={m.info_last_played()} for="s3">
<span id="s3">{formatTime(instance.lastPlayed)}</span>
						</Field>
						<Field label={m.info_total_time()} for="s4">
							<span id="s4">
								{m.editor_play_time_launches({
									duration: formatDuration(instance.playTimeSeconds),
									count: instance.launchCount
								})}
							</span>
						</Field>
						<Field label={m.info_mods()} for="s5">
<span id="s5">{m.common_item_count({ count: instance.modCount })}</span>
						</Field>
					</Group>

				{:else if page === 'console'}
					<div class="flex shrink-0 items-center gap-2.5">
						{#if running}
							<Pill accent>{m.info_running_pid({ pid: running.pid })}</Pill>
							<span class="text-muted">
								{running.online ? m.editor_online_mode() : m.editor_offline_mode()} · {running.playerName}
								· {formatTime(running.startedAt)}
							</span>
						{:else}
							<Pill>{m.editor_not_running()}</Pill>
						{/if}
						<span class="flex-1"></span>
						<Button variant="destructive" disabled={!running} onclick={stopGame}>
							{m.editor_stop_game()}
						</Button>
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
							<p class="py-6 text-center font-sans text-faint">{m.editor_no_output()}</p>
						{/each}
					</div>

					<div class="flex shrink-0 items-center gap-4.5">
						<CheckRow compact><input type="checkbox" bind:checked={autoScroll} />{m.editor_auto_scroll()}</CheckRow>
						<CheckRow compact><input type="checkbox" bind:checked={onlyErrors} />{m.editor_only_stderr()}</CheckRow>
						<CheckRow compact><input type="checkbox" bind:checked={wrapLines} />{m.editor_wrap_lines()}</CheckRow>
						<span class="flex-1"></span>
						<span class="text-faint">{m.editor_line_progress({ visible: visibleOutput.length, total: output.length })}</span>
						<Button disabled={output.length === 0} onclick={copyOutput}>
							{m.editor_copy()}
						</Button>
						<Button disabled={!!running || output.length === 0} onclick={clearOutput}>
							{m.editor_clear()}
						</Button>
					</div>

				{:else if page === 'version'}
					<div class="flex min-h-0 flex-1 gap-2.25">
						<TableBox>
							<Table>
								<thead>
									<tr><th style="width:34%">{m.common_name()}</th><th>{m.common_version()}</th><th style="width:110px">{m.editor_version_note()}</th></tr>
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
							</Table>
						</TableBox>
						{@render sideButtons([
							{
								label: m.editor_change_version(),
								action: () =>
									(versionDialog = {
										mcVersion: instance.versionId,
										loader: instance.loader,
										loaderVersion: instance.loaderVersion
									})
							},
							{ label: m.action_repair_reinstall(), action: () => reinstall() },
							{ label: m.editor_update_pack(), action: updatePack },
							{ label: m.folder_instance(), action: () => openFolder('instance') }
						])}
					</div>
				{:else if page === 'mods'}
					{#if modUpdates.length > 0}
						<div class="shrink-0 rounded-lg border border-line bg-deep p-2.5">
							<p class="mb-2 font-medium">{m.editor_mod_updates()}</p>
							<div class="flex flex-col gap-1">
								{#each modUpdates as update (update.id)}
									<div class="flex items-center gap-2.5">
										<span class="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap">
											{update.name}
											<span class="text-faint">
												{m.editor_mod_update_detail({
													from: update.currentVersion,
													to: update.newVersion,
													source: update.source
												})}
											</span>
										</span>
										<Button onclick={() => applyUpdate(update)}>{m.common_update()}</Button>
									</div>
								{/each}
							</div>
						</div>
					{/if}

					<div class="flex min-h-0 flex-1 gap-2.25">
						<TableBox>
							<Table>
								<thead>
									<tr>
										<th style="width:28px"></th>
										<th style="width:34%">{m.common_name()}</th>
										<th>{m.common_version()}</th>
										<th>{m.editor_mod_file()}</th>
										<th style="width:80px">{m.editor_mod_loader()}</th>
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
													aria-label={m.editor_mod_enable({ name: mod.name })}
													onclick={(e) => {
														e.stopPropagation();
														toggleMod(mod);
													}}
												/>
											</td>
											<td class:line-through={!mod.enabled} class:text-faint={!mod.enabled}
												>{mod.name}</td
											>
											<td class="text-muted">{mod.version || m.common_dash()}</td>
											<td class="text-muted">{mod.fileName}</td>
											<td class="text-muted">{mod.loader}</td>
										</tr>
									{/each}
								</tbody>
							</Table>
							{#if mods.length === 0}
								<p class="px-3 py-4 text-center text-muted">{m.editor_mods_empty()}</p>
							{/if}
						</TableBox>
						{@render sideButtons([
							{ label: m.editor_download_mods(), action: () => (browse = 'mod') },
							{ label: m.editor_check_updates(), disabled: mods.length === 0, action: checkUpdates },
							{
								label: m.common_remove(),
								disabled: !selectedMod,
								action: () => selectedMod && removeMod(selectedMod)
							},
							{ label: m.common_reload(), action: () => load('mods', true) },
							{ label: m.editor_mods_folder(), action: () => openFolder('mods') }
						])}
					</div>

				{:else if page === 'resourcepacks' || page === 'shaderpacks'}
					{@const packs = page === 'resourcepacks' ? resourcePacks : shaderPacks}
					<div class="flex min-h-0 flex-1 gap-2.25">
						<TableBox>
							<Table>
								<thead>
									<tr><th style="width:45%">{m.common_name()}</th><th>{m.editor_pack_type()}</th><th>{m.editor_pack_location()}</th></tr>
								</thead>
								<tbody>
									{#each packs as pack (pack.path)}
										<tr>
											<td>{pack.name}</td>
											<td class="text-muted">{pack.isFolder ? m.editor_pack_folder() : m.editor_pack_archive()}</td>
											<td class="overflow-hidden text-ellipsis font-mono text-[11px] text-muted"
												>{pack.path}</td
											>
										</tr>
									{/each}
								</tbody>
							</Table>
							{#if packs.length === 0}
								<p class="px-3 py-4 text-center text-muted">
									{page === 'resourcepacks' ? m.editor_resourcepacks_empty() : m.editor_shaderpacks_empty()}
								</p>
							{/if}
						</TableBox>
						{@render sideButtons([
							{
								label:
									page === 'resourcepacks'
										? m.editor_download_resourcepacks()
										: m.editor_download_shaderpacks(),
								action: () => (browse = page === 'resourcepacks' ? 'resourcepack' : 'shader')
							},
							{ label: m.common_reload(), action: () => load(page, true) },
							{
								label: m.common_open_folder(),
								action: () => openFolder(page === 'resourcepacks' ? 'resourcepacks' : 'shaderpacks')
							}
						])}
					</div>

				{:else if page === 'worlds'}
					<div class="flex min-h-0 flex-1 gap-2.25">
						<TableBox>
							<Table>
								<thead>
									<tr><th style="width:44px"></th><th style="width:45%">{m.common_name()}</th><th>{m.editor_world_folder()}</th></tr>
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
							</Table>
							{#if worlds.length === 0}
								<p class="px-3 py-4 text-center text-muted">{m.editor_worlds_empty()}</p>
							{/if}
						</TableBox>
						{@render sideButtons([
							{ label: m.common_reload(), action: () => load('worlds', true) },
							{ label: m.common_open_folder(), action: () => openFolder('saves') }
						])}
					</div>

				{:else if page === 'servers'}
					<div class="flex min-h-0 flex-1 gap-2.25">
						<TableBox>
							<Table>
								<thead>
									<tr>
										<th style="width:44px"></th>
										<th style="width:40%">{m.common_name()}</th>
										<th>{m.editor_server_address()}</th>
										<th style="width:90px">{m.editor_server_textures()}</th>
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
											<td class="text-muted">{server.acceptTextures ? m.editor_server_accept() : m.editor_server_prompt()}</td>
										</tr>
									{/each}
								</tbody>
							</Table>
							{#if servers.length === 0}
								<p class="px-3 py-4 text-center text-muted">
									{m.editor_servers_empty()}
								</p>
							{/if}
						</TableBox>
						{@render sideButtons([
							{ label: m.common_reload(), action: () => load('servers', true) },
							{ label: m.common_open_folder(), action: () => openFolder('minecraft') }
						])}
					</div>
				{:else if page === 'screenshots'}
					<div class="flex min-h-0 flex-1 gap-2.25">
						<TableBox>
							<Table>
								<thead>
									<tr><th style="width:55%">{m.editor_screenshot_name()}</th><th>{m.editor_screenshot_size()}</th><th>{m.editor_screenshot_taken()}</th></tr>
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
							</Table>
							{#if screenshots.length === 0}
								<p class="px-3 py-4 text-center text-muted">{m.editor_screenshots_empty()}</p>
							{/if}
						</TableBox>
						{@render sideButtons([
							{ label: m.common_reload(), action: () => load('screenshots', true) },
							{ label: m.common_open_folder(), action: () => openFolder('screenshots') }
						])}
					</div>

				{:else if page === 'notes'}
					<Textarea class="min-h-0 flex-1 p-2.5" bind:value={form.notes} placeholder={m.editor_notes_placeholder()} aria-label={m.editor_notes_aria()} />
				{:else if page === 'logs'}
					<div class="flex shrink-0 items-center gap-2.5">
						<Select value={logFile} onchange={(e) => openLog(e.currentTarget.value)} aria-label={m.editor_log_file()}>
							{#each logFiles as file (file.name)}
								<option value={file.name}>{m.editor_log_size({ name: file.name, size: formatSize(file.size) })}</option>
							{:else}
								<option value="">{m.editor_log_none()}</option>
							{/each}
						</Select>
						<SearchField
							class="bg-deep"
							placeholder={m.editor_log_search()}
							aria-label={m.editor_log_search()}
							bind:value={logSearch}
						>
							{#if logSearch}
								<span class="whitespace-nowrap text-faint">{m.common_line_count({ count: visibleLogLines.length })}</span>
							{/if}
						</SearchField>
						<span class="flex-1"></span>
						<Button onclick={() => load('logs', true)}>{m.common_reload()}</Button>
						<Button onclick={() => openFolder('logs')}>{m.action_folder()}</Button>
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
								{logFile ? m.editor_log_empty_file() : m.editor_log_no_files()}
							</p>
						{/each}
					</div>

					<div class="flex shrink-0 items-center gap-4.5">
						<CheckRow compact><input type="checkbox" bind:checked={wrapLines} />{m.editor_wrap_lines()}</CheckRow>
						<CheckRow compact><input type="checkbox" bind:checked={colorLog} />{m.editor_color_log()}</CheckRow>
					</div>

				{:else}
					<div class="min-h-0 flex-1 overflow-y-auto pr-1">
						<Group title="Java">
							<Field label={m.editor_java_executable()} for="java-path">
								<div class="flex items-center gap-1.75">
									<Select class="min-w-0 flex-1" bind:value={form.javaPath}>
										<option value="">{m.editor_java_follow_launcher()}</option>
										{#each javaCandidates as java (java.path)}
											<option value={java.path}>{m.editor_java_option({ version: java.version, path: java.path })}</option>
										{/each}
										{#if form.javaPath && !javaCandidates.some((j) => j.path === form.javaPath)}
											<option value={form.javaPath}>{form.javaPath}</option>
										{/if}
									</Select>
									<Button onclick={() => load('settings', true)}>{m.settings_java_redetect()}</Button>
								</div>
							</Field>
							<p class="px-0.5 py-1 text-faint">{m.editor_java_required({ version: instance.javaVersion })}</p>
						</Group>

						<Group title={m.settings_memory()} disabled={!form.settings.overrideMemory}>
							{#snippet header()}
								<span class="ml-auto text-[11.5px] font-normal text-muted">
									<Switch label={m.editor_override()} bind:checked={form.settings.overrideMemory} />
								</span>
							{/snippet}
							<Field label={m.editor_min_memory()} for="mem-min">
								<div class="flex items-center gap-1.75">
									<TextInput id="mem-min" type="number" style="width:100px" bind:value={form.settings.minMemoryMb} />
									<span class="text-muted">{m.common_mib()}</span>
								</div>
							</Field>
							<Field label={m.editor_max_memory()} for="mem-max">
								<div class="flex items-center gap-1.75">
									<TextInput id="mem-max" type="number" style="width:100px" bind:value={form.settings.maxMemoryMb} />
									<span class="text-muted">{m.common_mib()}</span>
								</div>
							</Field>
						</Group>

						<Group title={m.settings_jvm_args()} disabled={!form.settings.overrideJvmArgs}>
							{#snippet header()}
								<span class="ml-auto text-[11.5px] font-normal text-muted">
									<Switch label={m.editor_override()} bind:checked={form.settings.overrideJvmArgs} />
								</span>
							{/snippet}
							<Field label={m.editor_jvm_args_field()} for="jvm-args">
								<TextInput id="jvm-args" bind:value={form.settings.jvmArgs} placeholder="-XX:+UseG1GC" />
							</Field>
						</Group>

						<Group title={m.settings_commands()} disabled={!form.settings.overrideCommands}>
							{#snippet header()}
								<span class="ml-auto text-[11.5px] font-normal text-muted">
									<Switch label={m.editor_override()} bind:checked={form.settings.overrideCommands} />
								</span>
							{/snippet}
							<Field label={m.settings_pre_launch()} for="pre">
								<TextInput id="pre" bind:value={form.settings.preLaunch} placeholder={m.settings_command_placeholder()} />
							</Field>
							<Field label={m.settings_post_exit()} for="post">
								<TextInput id="post" bind:value={form.settings.postExit} />
							</Field>
						</Group>
					</div>
				{/if}
			</div>
		</div>
	</div>

	{#snippet footer()}
		{#if status}
			<span class="mr-auto pl-1 text-muted" role="status">{status}</span>
		{/if}
		<Button disabled={!dirty} onclick={save}>{m.common_save()}</Button>
		<Button variant="primary" onclick={saveAndClose}>{m.common_close()}</Button>
	{/snippet}
</WindowShell>

{#if versionDialog}
	<Dialog title={m.editor_change_version_title()} width="720px" height="560px" onclose={() => (versionDialog = null)}>
		<div class="flex min-h-0 flex-1 flex-col gap-3 px-4.5 py-4">
			<p class="shrink-0 text-muted">{m.editor_change_version_desc()}</p>
			<VersionPicker
				bind:mcVersion={versionDialog.mcVersion}
				bind:loader={versionDialog.loader}
				bind:loaderVersion={versionDialog.loaderVersion}
				onError={(message) => (status = message)}
			/>
		</div>
		{#snippet footer()}
			<Button variant="primary" disabled={!versionDialog?.mcVersion} onclick={() => versionDialog && reinstall({ ...versionDialog })}>
				{m.common_install()}
			</Button>
			<Button onclick={() => (versionDialog = null)}>{m.common_cancel()}</Button>
		{/snippet}
	</Dialog>
{/if}

{#if packUpdate}
	{@const result = packUpdate}
	<Dialog title={m.editor_pack_updated_title()} width="560px" height="480px" onclose={() => (packUpdate = null)}>
		<div class="flex min-h-0 flex-1 flex-col gap-3 overflow-auto px-4.5 py-4">
			<p>
				{m.editor_pack_updated_to()} <b>{result.instance.versionId || instance.versionId}</b>
				{#if result.instance.loader !== 'vanilla'}
					/ {loaderLabel(result.instance.loader)} {result.instance.loaderVersion}
				{/if}
			</p>

			{#if result.manualDownloads.length > 0}
				<div class="flex flex-col gap-1">
					<p class="font-medium text-danger">
						{m.editor_pack_manual({ count: result.manualDownloads.length })}
					</p>
					{#each result.manualDownloads as item (item.dest)}
						<div class="flex items-center gap-2.5">
							<span class="min-w-0 flex-1 truncate">{item.name}</span>
							<span class="font-mono text-[11px] text-muted">{item.dest}</span>
							{#if item.website}
								<Button onclick={() => api.openExternal(item.website!)}>
									{m.editor_pack_open_page()}
								</Button>
							{/if}
						</div>
					{/each}
				</div>
			{/if}

			{#if result.removedFiles.length > 0}
				<details>
					<summary class="cursor-pointer">{m.editor_pack_removed({ count: result.removedFiles.length })}</summary>
					<ul class="mt-1 flex flex-col gap-0.5 font-mono text-[11px] text-muted">
						{#each result.removedFiles as file (file)}
							<li>{file}</li>
						{/each}
					</ul>
				</details>
			{/if}

			{#if result.skipped.length > 0}
				<details>
					<summary class="cursor-pointer">{m.editor_pack_skipped({ count: result.skipped.length })}</summary>
					<ul class="mt-1 flex flex-col gap-0.5 font-mono text-[11px] text-muted">
						{#each result.skipped as file (file)}
							<li>{file}</li>
						{/each}
					</ul>
				</details>
			{/if}

			{#if result.manualDownloads.length === 0 && result.removedFiles.length === 0 && result.skipped.length === 0}
				<p class="text-muted">{m.editor_pack_nothing_manual()}</p>
			{/if}
		</div>
		{#snippet footer()}
			<Button variant="primary" onclick={() => (packUpdate = null)}>{m.common_close()}</Button>
		{/snippet}
	</Dialog>
{/if}

{#if browse}
	{@const kind = browse}
	<Dialog title={BROWSE_TITLES[kind]} width="940px" height="660px" onclose={() => (browse = null)}>
		<div class="flex min-h-0 flex-1 flex-col px-4.5 py-4">
			<ContentBrowser
				{baseDir}
				{instance}
				{kind}
				oninstalled={(fileName) => {
					load(BROWSE_PAGES[kind], true);
					report(m.editor_status_added({ file: fileName }));
				}}
			/>
		</div>
	</Dialog>
{/if}

{#if busy}
	<div class="fixed inset-0 z-100 grid place-items-center bg-scrim backdrop-blur-[3px]">
		<div
			class="flex items-center gap-3 rounded-xl border border-line bg-window px-6 py-5 shadow-lg"
			role="status"
		>
			<span class="size-5 animate-spin rounded-full border-2 border-line border-t-brand"></span>
			{busy}
		</div>
	</div>
{/if}
