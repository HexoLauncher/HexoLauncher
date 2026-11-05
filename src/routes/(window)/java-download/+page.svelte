<script lang="ts">
	import { Button, Field, Select } from '$lib/ui';
	import WindowShell from '$lib/components/WindowShell.svelte';
	import * as api from '$lib/api';
	import type { JavaRelease, JavaVendor, JavaVendorId } from '$lib/api';
	import { describeJavaProgress } from '$lib/format';
	import { closeSelf } from '$lib/windows';
	import { m } from '$lib/paraglide/messages.js';

	let baseDir = $state('');
	let vendors = $state<JavaVendor[]>([]);
	let vendor = $state<JavaVendorId>('temurin');
	let releases = $state<JavaRelease[]>([]);
	let version = $state(0);
	let loading = $state(false);
	let listError = $state('');

	let progress = $state<{ label: string; percent: number | null } | null>(null);
	let result = $state<{ ok: boolean; text: string } | null>(null);
	const busy = $derived(progress !== null);

	api.resolvePaths().then((paths) => (baseDir = paths.baseDir));
	api.javaVendors().then((list) => (vendors = list));

	$effect(() => {
		const current = vendor;
		let cancelled = false;
		loading = true;
		listError = '';
		releases = [];
		api
			.listJavaReleases(current)
			.then((list) => {
				if (cancelled) return;
				releases = list;
				if (!list.some((r) => r.version === version)) {
					version = (list.find((r) => r.lts) ?? list[0])?.version ?? 0;
				}
			})
			.catch((e) => {
				if (!cancelled) listError = m.java_download_list_failed({ message: api.errorMessage(e) });
			})
			.finally(() => {
				if (!cancelled) loading = false;
			});
		return () => {
			cancelled = true;
		};
	});

	async function download() {
		const target = { vendor, version };
		result = null;
		progress = { label: m.launch_java_preparing_download({ version: target.version }), percent: null };
		const unlisten = await api.onJavaProgress((p) => {
			if (p.version === target.version) progress = describeJavaProgress(p);
		});
		try {
			const java = await api.installJava(baseDir, target.version, target.vendor);
			result = { ok: true, text: m.java_download_done({ version: java.version, path: java.path }) };
		} catch (e) {
			result = { ok: false, text: m.settings_status_download_failed({ message: api.errorMessage(e) }) };
		} finally {
			unlisten();
			progress = null;
		}
	}
</script>

<svelte:head><title>{m.window_java_download()}</title></svelte:head>

<WindowShell>
	<div class="flex flex-1 flex-col gap-1 overflow-auto px-5 py-4.5">
		<Field label={m.java_download_vendor()} for="java-vendor" labelWidth="72px">
			<Select id="java-vendor" class="w-full" bind:value={vendor} disabled={busy}>
				{#each vendors as item (item.id)}
					<option value={item.id}>{item.name}</option>
				{/each}
			</Select>
		</Field>

		<Field label={m.java_download_version()} for="java-version" labelWidth="72px">
			<Select
				id="java-version"
				class="w-full"
				bind:value={version}
				disabled={busy || loading || releases.length === 0}
			>
				{#if loading}
					<option value={0}>{m.java_download_loading()}</option>
				{/if}
				{#each releases as release (release.version)}
					<option value={release.version}>
						{release.lts
							? m.java_download_release_lts({ version: release.version })
							: m.java_download_release({ version: release.version })}
					</option>
				{/each}
			</Select>
		</Field>

		<p class="mt-1 leading-[1.7] text-muted">{m.java_download_hint()}</p>

		{#if listError}
			<p class="mt-2 rounded-lg bg-danger-soft px-3 py-2 leading-[1.7] text-danger">{listError}</p>
		{/if}

		{#if progress}
			<div class="mt-2 rounded-lg border border-line bg-deep px-3 py-2">
				{#if progress.percent !== null}
					<div class="mb-2 h-1.5 overflow-hidden rounded-full bg-window">
						<span
							class="block h-full rounded-full bg-brand transition-[width] duration-100 ease-linear"
							style:width={`${progress.percent}%`}
						></span>
					</div>
				{/if}
				<div class="flex items-center gap-2 text-ink">
					<span
						class="size-2.5 shrink-0 animate-spin rounded-full border-[1.5px] border-line border-t-brand"
					></span>
					<span class="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap">
						{progress.label}
					</span>
					{#if progress.percent !== null}
						<span class="shrink-0 tabular-nums text-faint">{progress.percent}%</span>
					{/if}
				</div>
			</div>
		{:else if result}
			<p
				class="mt-2 rounded-lg px-3 py-2 leading-[1.7] break-all {result.ok
					? 'border border-line bg-deep text-ink'
					: 'bg-danger-soft text-danger'}"
			>
				{result.text}
			</p>
		{/if}
	</div>

	{#snippet footer()}
		<Button onclick={closeSelf}>{m.common_close()}</Button>
		<Button
			variant="primary"
			disabled={busy || loading || !baseDir || version === 0}
			onclick={download}
		>
			{m.java_download_start()}
		</Button>
	{/snippet}
</WindowShell>
