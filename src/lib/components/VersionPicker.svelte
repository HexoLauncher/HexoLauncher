<script lang="ts">
	import { Button, CheckRow, Group, Select, Table, TableBox } from '$lib/ui';
	import * as api from '$lib/api';
	import type { LoaderKind, VersionSummary } from '$lib/api';
	import { LOADERS, loaderLabel, releaseDate, versionTypeLabel } from '$lib/format';


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
			onError?.(`無法取得版本清單：${api.errorMessage(e)}`);
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
				onError?.(`${loaderLabel(loader)} 沒有支援 ${mcVersion}（${api.errorMessage(e)}）`);
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
					><th style="width:40%">版本</th><th>發布日期</th><th style="width:90px">類型</th></tr
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
			<p class="px-3 py-4 text-center text-muted">正在取得版本清單…</p>
		{:else if visible.length === 0}
			<p class="px-3 py-4 text-center text-muted">沒有符合篩選條件的版本。</p>
		{/if}
	</TableBox>

	<div class="flex w-36 shrink-0 flex-col gap-2">
		<Group title="篩選" class="mt-1">
			<CheckRow><input type="checkbox" bind:checked={filters.release} />正式版</CheckRow>
			<CheckRow><input type="checkbox" bind:checked={filters.snapshot} />快照</CheckRow>
			<CheckRow><input type="checkbox" bind:checked={filters.old} />舊版本</CheckRow>
		</Group>
		<Button disabled={loadingVersions} onclick={loadVersions}>重新整理</Button>
	</div>
</div>

<div class="flex shrink-0 flex-wrap items-center gap-3.5 pt-0.5">
	<span class="text-muted">模組載入器</span>
	{#each LOADERS as item (item)}
		<CheckRow>
			<input type="radio" name="loader" value={item} bind:group={loader} />
			{item === 'vanilla' ? '無' : loaderLabel(item)}
		</CheckRow>
	{/each}

	{#if loader !== 'vanilla'}
		<label class="flex items-center gap-2 text-muted">
			版本
			<Select bind:value={loaderVersion} disabled={loadingLoader}>
				{#each loaderVersions.slice(0, 60) as v (v)}
					<option value={v}>{v}</option>
				{:else}
					<option value="">{loadingLoader ? '讀取中…' : '沒有可用版本'}</option>
				{/each}
			</Select>
		</label>
	{/if}
</div>
