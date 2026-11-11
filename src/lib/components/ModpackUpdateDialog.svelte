<script lang="ts">
	import { Button, Field, Select } from '$lib/ui';
	import Dialog from '$lib/ui/Dialog.svelte';
	import ModpackBrowser from './ModpackBrowser.svelte';
	import * as api from '$lib/api';
	import type { InstanceView, ModpackUpdateResult, PackHit, RemoteVersion } from '$lib/api';
	import { untrack } from 'svelte';
	import { m } from '$lib/paraglide/messages.js';

	type OnlinePlatform = 'modrinth' | 'curseforge';

	let {
		baseDir,
		instance,
		onclose,
		onupdate,
		onlocal
	}: {
		baseDir: string;
		instance: InstanceView;
		onclose: () => void;
		onupdate: (task: () => Promise<ModpackUpdateResult>) => void;
		onlocal: () => void;
	} = $props();

	const source = $derived(instance.modpack);
	const known = $derived(
		!!source?.projectId && (source.platform === 'modrinth' || source.platform === 'curseforge')
	);

	let linking = $state(false);
	const mode = $derived(instance.linkedTo ? 'blocked' : known && !linking ? 'known' : 'link');

	let checking = $state(false);
	let checkError = $state('');
	let versions = $state<RemoteVersion[]>([]);
	let selected = $state('');
	let missing = $state<string[]>([]);

	let platform = $state<OnlinePlatform>(
		untrack(() => (instance.modpack?.platform === 'curseforge' ? 'curseforge' : 'modrinth'))
	);
	let hit = $state<PackHit | null>(null);
	let linkVersion = $state<RemoteVersion | null>(null);
	let linkError = $state('');

	$effect(() => {
		if (mode !== 'known') return;
		let cancelled = false;
		checking = true;
		checkError = '';
		Promise.all([
			api.checkModpackUpdate(baseDir, instance.id),
			api.modpackMissingFiles(baseDir, instance.id)
		])
			.then(([check, absent]) => {
				if (cancelled) return;
				versions = check.versions;
				selected = check.versions[0]?.id ?? '';
				missing = absent;
			})
			.catch((e) => {
				if (!cancelled) checkError = api.errorMessage(e);
			})
			.finally(() => {
				if (!cancelled) checking = false;
			});
		return () => {
			cancelled = true;
		};
	});

	$effect(() => {
		if (mode !== 'link') return;
		let cancelled = false;
		api
			.modpackMissingFiles(baseDir, instance.id)
			.then((absent) => {
				if (!cancelled) missing = absent;
			})
			.catch(() => {});
		return () => {
			cancelled = true;
		};
	});

	const repair = $derived(
		mode === 'known' && versions.length === 0 && missing.length > 0 && !!source?.versionId
	);

	const currentPackId = $derived(source?.platform === platform ? (source?.projectId ?? '') : '');
	const sameSelected = $derived(
		mode === 'link' &&
			!!hit &&
			!!linkVersion &&
			!!source?.versionId &&
			hit.id === currentPackId &&
			linkVersion.id === source.versionId
	);
	const linkRepair = $derived(sameSelected && missing.length > 0);

	function platformLabel(id: string | null | undefined) {
		return id === 'modrinth' ? 'Modrinth' : id === 'curseforge' ? 'CurseForge' : m.editor_version_modpack_local();
	}

	function versionLabel(v: RemoteVersion) {
		return v.mcVersion ? `${v.name} (MC ${v.mcVersion})` : v.name;
	}

	const ready = $derived(
		mode === 'known'
			? !checking && (!!selected || repair)
			: !!hit && !!linkVersion?.downloadable && (!sameSelected || linkRepair)
	);

	function update() {
		if (repair && source) {
			const { versionId, versionName } = source;
			onupdate(() => api.updateRemoteModpack(baseDir, instance.id, versionId, versionName));
			return;
		}
		if (mode === 'known') {
			const version = versions.find((v) => v.id === selected);
			if (!version) return;
			onupdate(() => api.updateRemoteModpack(baseDir, instance.id, version.id, version.name));
			return;
		}
		const pack = hit;
		const version = linkVersion;
		if (!pack || !version) return;
		const link = { platform, packId: pack.id };
		onupdate(() => api.updateRemoteModpack(baseDir, instance.id, version.id, version.name, link));
	}
</script>

{#snippet missingNote()}
	<div class="rounded-lg border border-line bg-deep px-3 py-2 leading-[1.7]">
		<p class="font-medium text-danger">{m.editor_pack_missing({ count: missing.length })}</p>
		<details>
			<summary class="cursor-pointer text-muted">{m.editor_pack_missing_list()}</summary>
			<ul class="mt-1 max-h-32 overflow-y-auto font-mono text-[11px] text-muted">
				{#each missing as file (file)}
					<li class="truncate">{file}</li>
				{/each}
			</ul>
		</details>
	</div>
{/snippet}

<Dialog
	title={m.editor_pack_update_title()}
	width={mode === 'link' ? '940px' : '480px'}
	height={mode === 'link' ? '660px' : 'fit-content'}
	{onclose}
>
	{#if mode === 'blocked'}
		<div class="flex flex-col gap-2.5 px-4.5 py-4 leading-[1.7]">
			<p class="font-medium text-danger">{m.editor_pack_linked_title()}</p>
			<p>{m.editor_pack_linked()}</p>
			<p class="rounded-lg border border-line bg-deep px-3 py-2 font-mono text-[11.5px] break-all">
				{instance.linkedTo}
			</p>
			<p class="text-muted">{m.editor_pack_linked_hint()}</p>
		</div>
	{:else if mode === 'known'}
		<div class="flex flex-col gap-3 px-4.5 py-4">
			<p>
				{m.editor_pack_update_current({
					version: source?.versionName || m.editor_version_unrecorded()
				})}
				<span class="text-faint">· {platformLabel(source?.platform)}</span>
			</p>

			{#if checking}
				<p class="flex items-center gap-2 text-muted">
					<span class="size-3.5 animate-spin rounded-full border-2 border-line border-t-brand"></span>
					{m.editor_busy_check_pack_update()}
				</p>
			{:else if checkError}
				<p class="rounded-lg bg-danger-soft px-3 py-2 leading-[1.7] text-danger">{checkError}</p>
			{:else if versions.length === 0}
				{#if missing.length > 0}
					<p>{m.editor_pack_latest_but_missing()}</p>
					{@render missingNote()}
					<p class="leading-[1.7] text-muted">{m.editor_pack_repair_hint()}</p>
				{:else}
					<p class="text-muted">{m.editor_status_pack_up_to_date()}</p>
				{/if}
			{:else}
				{#if missing.length > 0}
					{@render missingNote()}
				{/if}
				<Field label={m.editor_pack_update_target()} for="pack-version" labelWidth="88px">
					<Select id="pack-version" class="w-full" bind:value={selected}>
						{#each versions as version (version.id)}
							<option value={version.id}>{versionLabel(version)}</option>
						{/each}
					</Select>
				</Field>
				<p class="leading-[1.7] text-muted">{m.editor_pack_update_hint()}</p>
			{/if}
		</div>
	{:else}
		<div class="flex min-h-0 flex-1 flex-col gap-2.5 px-4.5 py-4">
			{#if source?.versionName}
				<p>
					{m.editor_pack_update_current({ version: source.versionName })}
					<span class="text-faint">· {platformLabel(source.platform)}</span>
				</p>
			{/if}
			<p class="leading-[1.7] text-muted">
				{known ? m.editor_pack_link_change() : m.editor_pack_link_intro()}
			</p>
			<div class="flex shrink-0 gap-1.5">
				{#each ['modrinth', 'curseforge'] as const as id (id)}
					<Button
						variant={platform === id ? 'primary' : 'default'}
						onclick={() => {
							if (platform === id) return;
							platform = id;
							hit = null;
							linkVersion = null;
						}}
					>
						{platformLabel(id)}
					</Button>
				{/each}
			</div>
			{#if missing.length > 0}
				{@render missingNote()}
			{/if}
			{#if linkError}
				<p class="rounded-lg bg-danger-soft px-3 py-2 leading-[1.7] text-danger">{linkError}</p>
			{/if}
			{#key platform}
				<ModpackBrowser
					{platform}
					initialQuery={instance.name}
					{currentPackId}
					currentVersionId={source?.versionId ?? ''}
					bind:hit
					bind:version={linkVersion}
					onerror={(message) => (linkError = message)}
				/>
			{/key}
			{#if sameSelected && !linkRepair}
				<p class="shrink-0 font-medium text-ink">{m.editor_pack_same_version()}</p>
			{:else}
				<p class="shrink-0 leading-[1.7] text-muted">
					{linkRepair ? m.editor_pack_repair_hint() : m.editor_pack_update_hint()}
				</p>
			{/if}
		</div>
	{/if}

	{#snippet footer()}
		{#if mode === 'blocked'}
			<Button variant="primary" onclick={onclose}>{m.common_close()}</Button>
		{:else}
			<Button class="mr-auto" onclick={onlocal}>{m.editor_pack_update_local()}</Button>
			{#if known}
				<Button onclick={() => (linking = !linking)}>
					{linking ? m.editor_pack_link_back() : m.editor_pack_link_other()}
				</Button>
			{/if}
			<Button onclick={onclose}>{m.common_cancel()}</Button>
			<Button variant="primary" disabled={!ready} onclick={update}>
				{repair || linkRepair ? m.editor_pack_repair() : m.common_update()}
			</Button>
		{/if}
	{/snippet}
</Dialog>
