<script lang="ts">
	import { Button, CheckRow, Group, Select, Table, TableBox } from '$lib/ui';
	import * as api from '$lib/api';
	import type { LoaderKind, VersionSummary } from '$lib/api';
	import { LOADERS, loaderLabel, releaseDate, versionTypeLabel } from '$lib/format';
	import { m } from '$lib/paraglide/messages.js';


	let {
		mcVersion = $bindable(''),
		loader = $bindable('vanilla'),
		loaderVersion = $bindable(''),
		onError
	}: {
		mcVersion?: string;
		loader?: LoaderKind;
		loaderVersion?: string;
		onError?: (message: string) => void;
	} = $props();

	let versions = $state<VersionSummary[]>([]);
	let loadingVersions = $state(true);
	let filters = $state({ release: true, snapshot: false, old: false });

	let loaderVersions = $state<string[]>([]);
	let loadingLoader = $state(false);

	const visible = $derived(
		versions.filter((v) =>
			v.versionType === 'release'
				? filters.release
				: v.versionType === 'snapshot'
					? filters.snapshot
					: filters.old
		)
	);

	async function loadVersions() {
		loadingVersions = true;
		try {

			const list = await api.listVersions(true);
			versions = list.versions;
			if (!mcVersion) mcVersion = list.latestRelease;
		} catch (e) {
			onError?.(m.error_version_list({ message: api.errorMessage(e) }));
		} finally {
			loadingVersions = false;
		}
	}

	loadVersions();


	api
		.getLauncherConfig()
		.then((config) => (filters.snapshot = config.ui.includeSnapshots))
		.catch(() => {});


	$effect(() => {
		if (loader === 'vanilla' || !mcVersion) {
			loaderVersions = [];
			loaderVersion = '';
			return;
		}

		let cancelled = false;
		loadingLoader = true;
		api
			.listLoaderVersions(loader, mcVersion)
			.then((list) => {
				if (cancelled) return;
				loaderVersions = list;

				if (!list.includes(loaderVersion)) loaderVersion = list[0] ?? '';
			})
			.catch((e) => {
				if (cancelled) return;
				loaderVersions = [];
				loaderVersion = '';
				onError?.(
					m.error_loader_unsupported({
						loader: loaderLabel(loader),
						version: mcVersion,
						message: api.errorMessage(e)
					})
				);
			})
			.finally(() => {
				if (!cancelled) loadingLoader = false;
			});

		return () => {
			cancelled = true;
		};
	});
</script>

<div class="flex min-h-0 flex-1 gap-2.5">
	<TableBox>
		<Table>
			<thead>
				<tr
					><th style="width:40%">{m.common_version()}</th><th>{m.version_table_date()}</th><th style="width:90px">{m.version_table_type()}</th></tr
				>
			</thead>
			<tbody>
				{#each visible as row (row.id)}
					<tr class:selected={mcVersion === row.id} onclick={() => (mcVersion = row.id)}>
						<td>{row.id}</td>
						<td class="text-muted">{releaseDate(row.releaseTime)}</td>
						<td class="text-muted">{versionTypeLabel(row.versionType)}</td>
					</tr>
				{/each}
			</tbody>
		</Table>
		{#if loadingVersions}
			<p class="px-3 py-4 text-center text-muted">{m.version_loading()}</p>
		{:else if visible.length === 0}
			<p class="px-3 py-4 text-center text-muted">{m.version_no_match()}</p>
		{/if}
	</TableBox>

	<div class="flex w-36 shrink-0 flex-col gap-2">
		<Group title={m.filter_title()} class="mt-1">
			<CheckRow><input type="checkbox" bind:checked={filters.release} />{m.version_type_release()}</CheckRow>
			<CheckRow><input type="checkbox" bind:checked={filters.snapshot} />{m.version_type_snapshot()}</CheckRow>
			<CheckRow><input type="checkbox" bind:checked={filters.old} />{m.filter_old()}</CheckRow>
		</Group>
		<Button disabled={loadingVersions} onclick={loadVersions}>{m.common_refresh()}</Button>
	</div>
</div>

<div class="flex shrink-0 flex-wrap items-center gap-3.5 pt-0.5">
	<span class="text-muted">{m.common_loader()}</span>
	{#each LOADERS as item (item)}
		<CheckRow>
			<input type="radio" name="loader" value={item} bind:group={loader} />
			{item === 'vanilla' ? m.common_none() : loaderLabel(item)}
		</CheckRow>
	{/each}

	{#if loader !== 'vanilla'}
		<label class="flex items-center gap-2 text-muted">
			{m.common_version()}
			<Select bind:value={loaderVersion} disabled={loadingLoader}>
				{#each loaderVersions.slice(0, 60) as v (v)}
					<option value={v}>{v}</option>
				{:else}
					<option value="">{loadingLoader ? m.common_loading() : m.version_no_available()}</option>
				{/each}
			</Select>
		</label>
	{/if}
</div>
