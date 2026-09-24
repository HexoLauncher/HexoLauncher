<script lang="ts">
	import { Button, CheckRow, Field, Select, Table, TableBox, TextInput } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import IconPicker from './IconPicker.svelte';
	import WindowShell from './WindowShell.svelte';
	import NavList from '$lib/ui/NavList.svelte';
	import ModpackBrowser from './ModpackBrowser.svelte';
	import VersionPicker from './VersionPicker.svelte';
	import * as api from '$lib/api';
	import type {
		ForeignImportResult,
		ForeignInstance,
		ForeignLauncher,
		ImportMode,
		InstanceView,
		LoaderKind,
		ModpackInstallResult,
		ModpackPlatform,
		ModpackPreview,
		PackHit,
		PackTarget,
		RemoteVersion
	} from '$lib/api';
	import { loaderLabel } from '$lib/format';
	import { SvelteSet } from 'svelte/reactivity';
	import type { Page } from '$lib/ui';
	import { m } from '$lib/paraglide/messages.js';

	let {
		baseDir,
		groups,
		onclose,
		oncreated
	}: {
		baseDir: string;
		groups: string[];
		onclose: () => void;
		oncreated: (instance: InstanceView) => void;
	} = $props();

	const pages: Page[] = [
		{ id: 'custom', label: m.add_page_custom(), icon: 'box' },
		{ id: 'modrinth', label: 'Modrinth', icon: 'globe' },
		{ id: 'curseforge', label: 'CurseForge', icon: 'globe' },
		{ id: 'ftb', label: 'FTB', icon: 'globe' },
		{ id: 'atlauncher', label: 'ATLauncher', icon: 'globe' },
		{ id: 'modpack', label: m.add_page_modpack(), icon: 'package' },
		{ id: 'launcher', label: m.add_page_launcher(), icon: 'shortcut' },
		{ id: 'import', label: m.add_page_import(), icon: 'upload' }
	];

	const PLATFORMS: string[] = ['modrinth', 'curseforge', 'ftb', 'atlauncher'];

	const FORMAT_LABELS: Record<api.ModpackFormat, string> = {
		modrinth: 'Modrinth',
		curseforge: 'CurseForge',
		atlauncher: 'ATLauncher',
		ftb: 'FTB'
	};

	let page = $state('custom');
	let name = $state('');
	let iconChoice = $state<string | null>(null);
	let iconPath = $state('');
	let iconFile = $state<string | null>(null);
	let group = $state('');
	let loader = $state<LoaderKind>('fabric');
	let mcVersion = $state('');
	let loaderVersion = $state('');

	let busy = $state('');
	let error = $state('');
	let zipPath = $state('');

	let packPath = $state('');
	let filePreview = $state<ModpackPreview | null>(null);

	let remoteHit = $state<PackHit | null>(null);
	let remoteVersion = $state<RemoteVersion | null>(null);
	let includeOptional = $state(true);


	let autoName = '';

	const packIcon = $derived(remoteHit?.iconUrl ?? null);
	const icon = $derived(iconChoice ?? (packIcon ? 'custom' : 'grass'));
	const iconImage = $derived(iconFile ?? packIcon);
	const iconSource = $derived({
		icon,
		iconPath: icon === 'custom' && iconFile ? iconPath : null,
		iconUrl: icon === 'custom' && !iconFile ? packIcon : null
	});


	let result = $state<ModpackInstallResult | null>(null);

	let launchers = $state<ForeignLauncher[]>([]);
	let launcherPath = $state('');
	let foreign = $state<ForeignInstance[]>([]);
	const picked = new SvelteSet<string>();
	let importMode = $state<ImportMode>('copy');
	let foreignResult = $state<ForeignImportResult | null>(null);
	let scanned = false;

	const platform = $derived(PLATFORMS.includes(page) ? (page as ModpackPlatform) : null);

	const action = $derived.by(() => {
		if (platform) {
			return {
				label: m.common_install(),
				icon: 'download',
				ready: !!remoteHit && !!remoteVersion,
				run: () => {
					const hit = remoteHit;
					const version = remoteVersion;
					if (!hit || !version) return;
					installPack(
						hit.name,
						(target) =>
							api.installRemoteModpack(target, platform, hit.id, version.id, includeOptional),
						m.busy_downloading_modpack()
					);
				}
			};
		}
		switch (page) {
			case 'custom':
				return { label: m.common_ok(), icon: 'plus', ready: !!name.trim() && !!mcVersion, run: submit };
			case 'launcher':
				return {
					label: importMode === 'link' ? m.add_action_link() : m.add_action_copy(),
					icon: importMode === 'link' ? 'shortcut' : 'copy',
					ready: picked.size > 0,
					run: importForeign
				};
			case 'modpack':
				return {
					label: m.common_install(),
					icon: 'download',
					ready: !!filePreview,
					run: () =>
						filePreview &&
						installPack(filePreview.name, (target) =>
							api.installModpackFile(target, packPath.trim())
						)
				};
			default:
				return { label: m.common_import(), icon: 'upload', ready: !!zipPath.trim(), run: importZip };
		}
	});

	function selectPage(id: string) {
		page = id;
		error = '';
		remoteHit = null;
		remoteVersion = null;
		if (id === 'launcher' && !scanned) {
			scanned = true;
			scanLaunchers();
		}
	}

	async function submit() {
		error = '';
		busy = m.busy_creating();
		try {
			const created = await api.createInstance({
				baseDir,
				name: name.trim(),
				icon: iconSource.icon,
				iconPath: iconSource.iconPath,
				group,
				mcVersion,
				loader,
				loaderVersion: loader === 'vanilla' ? null : loaderVersion || null
			});
			oncreated(created);
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			busy = '';
		}
	}

	async function pickFile(filterName: string, extensions: string[]) {
		try {
			const { open } = await import('@tauri-apps/api/dialog');
			const picked = await open({ filters: [{ name: filterName, extensions }] });
			return typeof picked === 'string' ? picked : null;
		} catch {
			error = m.common_no_dialog_paste_file();
			return null;
		}
	}

	async function pickIcon() {
		const picked = await pickFile(m.file_filter_image(), ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp']);
		if (!picked) return;
		try {
			iconFile = await api.readImageFile(picked);
			iconPath = picked;
			iconChoice = 'custom';
		} catch (e) {
			error = api.errorMessage(e);
		}
	}

	async function pickZip() {
		const picked = await pickFile(m.file_filter_zip(), ['zip']);
		if (picked) zipPath = picked;
	}

	async function importZip() {
		error = '';
		busy = m.busy_extracting();
		try {
			oncreated(await api.importInstance(baseDir, zipPath.trim()));
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			busy = '';
		}
	}

	async function scanLaunchers() {
		busy = m.busy_scanning_launchers();
		try {
			launchers = await api.detectForeignLaunchers();
			const first = launchers[0];
			if (first) await selectLauncher(first.path);
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			busy = '';
		}
	}

	async function selectLauncher(path: string) {
		launcherPath = path;
		picked.clear();
		foreign = [];
		if (!path.trim()) return;

		error = '';
		busy = m.busy_loading_instances();
		try {
			foreign = await api.listForeignInstances(path.trim());
			if (foreign.length === 0) error = m.add_launcher_empty();
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			busy = '';
		}
	}

	async function pickLauncherDir() {
		try {
			const { open } = await import('@tauri-apps/api/dialog');
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

	async function importForeign() {
		error = '';
		busy = importMode === 'link' ? m.busy_linking() : m.busy_copying();

		let unlisten: (() => void) | undefined;
		try {
			unlisten = await api.onInstallProgress((p) => {
				busy =
					p.total > 1
						? m.common_progress_count({ message: p.message, done: p.done, total: p.total })
						: p.message;
			});

			foreignResult = await api.importForeignInstances({
				baseDir,
				paths: foreign.filter((i) => picked.has(i.path)).map((i) => i.path),
				mode: importMode,
				group
			});
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			unlisten?.();
			busy = '';
		}
	}

	function suggestName(packName: string) {
		if (!name.trim() || name === autoName) name = packName;
		autoName = packName;
	}

	async function pickPack() {
		const picked = await pickFile(m.file_filter_modpack(), ['mrpack', 'zip']);
		if (!picked) return;
		packPath = picked;
		await inspectPack();
	}

	async function inspectPack() {
		filePreview = null;
		if (!packPath.trim()) return;
		error = '';
		busy = m.busy_reading_modpack();
		try {
			filePreview = await api.inspectModpack(packPath.trim());
			suggestName(filePreview.name);
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			busy = '';
		}
	}

	async function installPack(
		packName: string,
		run: (target: PackTarget) => Promise<ModpackInstallResult>,
		initial = m.busy_preparing_install()
	) {
		error = '';
		busy = initial;
		const instanceName = name.trim() || packName;
		let unlisten: (() => void) | undefined;
		try {
			const id = await api.suggestInstanceId(baseDir, instanceName);
			unlisten = await api.onInstallProgress((p) => {
				if (p.instanceName === id) {
					busy =
					p.total > 1
						? m.common_progress_count({ message: p.message, done: p.done, total: p.total })
						: p.message;
				}
			});

			const installed = await run({ baseDir, name: instanceName, ...iconSource, group });
			if (installed.manualDownloads.length || installed.skipped.length) {
				result = installed;
			} else {
				oncreated(installed.instance);
			}
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			unlisten?.();
			busy = '';
		}
	}

	function attempt(task: Promise<unknown>) {
		task.catch((e) => (error = api.errorMessage(e)));
	}
</script>

{#snippet identity(placeholder: string)}
	<div class="flex shrink-0 items-start gap-3">
		<IconPicker {icon} image={iconImage} onselect={(choice) => (iconChoice = choice)} onpickimage={pickIcon} />

		<div class="min-w-0 flex-1 [--field-label:48px]">
			<Field label={m.common_name()} for="inst-name">
				<TextInput id="inst-name" bind:value={name} {placeholder} />
			</Field>
			<Field label={m.common_group()} for="inst-group">
				<TextInput id="inst-group" bind:value={group} list="group-suggestions" placeholder={m.common_ungrouped()} />
				<datalist id="group-suggestions">
					{#each groups as g (g)}
						<option value={g}></option>
					{/each}
				</datalist>
			</Field>
		</div>
	</div>
{/snippet}

{#snippet packSummary(preview: ModpackPreview)}
	<dl
		class="grid shrink-0 grid-cols-[96px_1fr] gap-x-3 gap-y-1.5 rounded-lg border border-line bg-deep px-3.5 py-3"
	>
		<dt class="text-muted">{m.add_pack_field()}</dt>
		<dd class="min-w-0 truncate text-ink">
			{preview.name}{preview.version ? ` ${preview.version}` : ''}
		</dd>
		<dt class="text-muted">{m.add_pack_format()}</dt>
		<dd>{FORMAT_LABELS[preview.format]}</dd>
		<dt class="text-muted">Minecraft</dt>
		<dd>{preview.mcVersion}</dd>
		<dt class="text-muted">{m.common_loader()}</dt>
		<dd>
			{preview.loader === 'vanilla'
				? m.common_none()
				: `${loaderLabel(preview.loader)} ${preview.loaderVersion ?? m.add_pack_loader_latest()}`}
		</dd>
		<dt class="text-muted">{m.add_pack_files()}</dt>
		<dd>{m.common_item_count({ count: preview.fileCount })}</dd>
	</dl>
{/snippet}

<WindowShell>
	<div class="flex min-h-0 flex-1">
		{#if !result && !foreignResult}
			<NavList {pages} selected={page} onselect={selectPage} width="162px" />
		{/if}

		<div class="flex min-w-0 flex-1 flex-col gap-3.5 overflow-y-auto px-4.5 py-4">
			{#if foreignResult}
				{@const done = foreignResult}
				<h2 class="text-[15px] font-semibold text-ink">
					{done.imported.length > 0
						? m.add_imported_title({ count: done.imported.length })
						: m.add_imported_none()}
				</h2>

				{#if done.imported.length}
					<ul class="rounded-lg border border-line bg-deep">
						{#each done.imported as item (item.id)}
							<li class="flex items-center gap-3 border-b border-line px-3 py-2 last:border-b-0">
								<InstanceIcon name={item.icon} image={item.iconImage} size={22} />
								<span class="min-w-0 flex-1 truncate text-ink">{item.name}</span>
								<span class="text-faint">{item.versionId || m.common_unknown_version()}</span>
							</li>
						{/each}
					</ul>
					<p class="leading-[1.8] text-muted">
						{m.add_imported_note()}
					</p>
				{/if}

				{#if done.failed.length}
					<p class="leading-[1.8] text-danger">{m.add_imported_failed()}</p>
					<ul class="rounded-lg border border-line bg-deep">
						{#each done.failed as item (item.path)}
							<li class="border-b border-line px-3 py-2 last:border-b-0">
								<div class="truncate text-ink">{item.name}</div>
								<div class="text-faint">{item.message}</div>
							</li>
						{/each}
					</ul>
				{/if}
			{:else if result}
				{@const done = result}
				<h2 class="text-[15px] font-semibold text-ink">
					{m.add_installed_title({ name: done.instance.name })}
				</h2>

				{#if done.manualDownloads.length}
					<p class="leading-[1.8] text-muted">
						{m.add_manual_downloads()}
					</p>
					<ul class="min-h-0 flex-1 overflow-y-auto rounded-lg border border-line bg-deep">
						{#each done.manualDownloads as file (file.dest)}
							<li class="flex items-center gap-3 border-b border-line px-3 py-2 last:border-b-0">
								<div class="min-w-0 flex-1">
									<div class="truncate text-ink">{file.name}</div>
									<div class="truncate text-faint">{file.dest}</div>
								</div>
								{#if file.website}
									{@const website = file.website}
									<Button onclick={() => attempt(api.openExternal(website))}>
										<Icon name="globe" size={13} />{m.add_manual_download_page()}
									</Button>
								{/if}
								<Button onclick={() => attempt(api.openInstanceFolder(baseDir, done.instance.id, file.folder))}>
									<Icon name="folder" size={13} />{m.action_folder()}
								</Button>
							</li>
						{/each}
					</ul>
				{/if}

				{#if done.skipped.length}
					<p class="leading-[1.8] text-muted">
						{m.add_skipped({ items: done.skipped.join(m.common_list_separator()) })}
					</p>
				{/if}
			{:else if platform}
				{#key platform}
					<ModpackBrowser
						{platform}
						bind:hit={remoteHit}
						bind:version={remoteVersion}
						bind:includeOptional
						disabled={!!busy}
						onchoose={(hit) => suggestName(hit.name)}
						onerror={(message) => (error = message)}
					/>
				{/key}
				{#if remoteHit}
					{@render identity(remoteHit.name)}
				{/if}
			{:else if page === 'custom'}
				{@render identity(m.add_name_placeholder())}

				<VersionPicker
					bind:mcVersion
					bind:loader
					bind:loaderVersion
					onError={(message) => (error = message)}
				/>
			{:else if page === 'modpack'}
				<Field label={m.add_pack_field()} for="pack-path">
					<div class="flex gap-1.75">
						<TextInput class="min-w-0 flex-1" id="pack-path" bind:value={packPath} oninput={() => (filePreview = null)} onchange={inspectPack} placeholder={m.add_pack_path_placeholder()} />
						<Button disabled={!!busy} onclick={pickPack}>{m.common_browse()}</Button>
					</div>
				</Field>

				{#if filePreview}
					{@render identity(filePreview.name)}
					{@render packSummary(filePreview)}
					{#if filePreview.format === 'curseforge'}
						<p class="px-0.5 leading-[1.8] text-faint">
							{m.add_pack_note_curseforge()}
						</p>
					{/if}
				{:else}
					<p class="px-0.5 leading-[1.8] text-faint">
						{m.add_pack_note()}
					</p>
				{/if}
			{:else if page === 'launcher'}
				<div class="flex min-h-0 flex-1 flex-col gap-3">
					<Field label={m.add_source()} for="foreign-src">
						<div class="flex gap-1.75">
							<Select class="min-w-0 flex-1" id="foreign-src" disabled={!!busy} value={launcherPath} onchange={(e) => selectLauncher(e.currentTarget.value)}>
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
							<Button disabled={!!busy} onclick={pickLauncherDir}>{m.common_browse()}</Button>
						</div>
					</Field>

					<TableBox>
						<Table>
							<thead>
								<tr>
									<th style="width:34px">
										<input
											type="checkbox"
											aria-label={m.add_select_all()}
											disabled={foreign.length === 0}
											checked={foreign.length > 0 && picked.size === foreign.length}
											onchange={toggleAll}
										/>
									</th>
									<th>{m.common_name()}</th>
									<th style="width:92px">{m.common_version()}</th>
									<th style="width:128px">{m.common_loader()}</th>
									<th style="width:64px">{m.info_mods()}</th>
								</tr>
							</thead>
							<tbody>
								{#each foreign as item (item.path)}
									<tr>
										<td>
											<input
												type="checkbox"
												aria-label={item.name}
												checked={picked.has(item.path)}
												onchange={() => toggle(item.path)}
											/>
										</td>
										<td class="truncate">{item.name}</td>
										<td class="text-muted">{item.mcVersion || m.common_dash()}</td>
										<td class="text-muted">
											{#if item.unsupportedLoader}
												<span class="text-danger">{m.add_loader_unsupported({ loader: item.unsupportedLoader })}</span>
											{:else if item.loader === 'vanilla'}
												{m.common_none()}
											{:else}
												{loaderLabel(item.loader)}
											{/if}
										</td>
										<td class="text-muted">{item.modCount}</td>
									</tr>
								{/each}
							</tbody>
						</Table>
						{#if foreign.length === 0}
							<p class="px-3 py-4 text-center text-muted">
								{m.add_launcher_hint()}
							</p>
						{/if}
					</TableBox>

					<Field label={m.common_group()} for="foreign-group">
						<TextInput id="foreign-group" bind:value={group} list="group-suggestions" placeholder={m.common_ungrouped()} />
						<datalist id="group-suggestions">
							{#each groups as g (g)}
								<option value={g}></option>
							{/each}
						</datalist>
					</Field>

					<div class="flex shrink-0 flex-col">
						<CheckRow class="items-start">
							<input type="radio" class="mt-[3px]" value="copy" bind:group={importMode} />
							<span class="min-w-0">
								<span class="text-ink">{m.add_mode_copy()}</span>
								<span class="block leading-[1.7] text-faint">
									{m.add_mode_copy_desc()}
								</span>
							</span>
						</CheckRow>
						<CheckRow class="items-start">
							<input type="radio" class="mt-[3px]" value="link" bind:group={importMode} />
							<span class="min-w-0">
								<span class="text-ink">{m.add_mode_link()}</span>
								<span class="block leading-[1.7] text-faint">
									{m.add_mode_link_desc()}
								</span>
							</span>
						</CheckRow>
					</div>
				</div>
			{:else}
				<div class="flex min-h-0 flex-1 flex-col gap-3">
					<Field label={m.add_zip_field()} for="zip-path">
						<div class="flex gap-1.75">
							<TextInput class="min-w-0 flex-1" id="zip-path" bind:value={zipPath} placeholder={m.add_zip_placeholder()} />
							<Button onclick={pickZip}>{m.common_browse()}</Button>
						</div>
					</Field>

					<p class="px-0.5 leading-[1.8] text-faint">
						{m.add_zip_note()}
					</p>

					<div class="flex gap-1.75">
						<Button onclick={() => api.openLauncherFolder('instances')}>
							{m.folder_open_instances()}
						</Button>
					</div>
				</div>
			{/if}

			{#if error}
				<p class="shrink-0 rounded-lg bg-danger-soft px-3 py-2 leading-[1.7] text-danger">
					{error}
				</p>
			{/if}
		</div>
	</div>

	{#snippet footer()}
		{#if busy}
			<span class="mr-auto flex min-w-0 items-center gap-2 pl-1 text-muted">
				<span
					class="size-4 shrink-0 animate-spin rounded-full border-2 border-line border-t-brand"
				></span>
				<span class="truncate">{busy}</span>
			</span>
		{/if}
		{#if foreignResult}
			{@const first = foreignResult.imported[0]}
			<Button variant="primary" onclick={() => (first ? oncreated(first) : onclose())}>
				<Icon name="check" size={13} />{m.common_done()}
			</Button>
		{:else if result}
			{@const done = result}
			<Button variant="primary" onclick={() => oncreated(done.instance)}>
				<Icon name="check" size={13} />{m.common_done()}
			</Button>
		{:else}
			<Button variant="primary" disabled={!action.ready || !!busy} onclick={action.run}>
				<Icon name={action.icon} size={13} />{action.label}
			</Button>
			<Button disabled={!!busy} onclick={onclose}>{m.common_cancel()}</Button>
		{/if}
	{/snippet}
</WindowShell>
