<script lang="ts">
	import { onMount } from 'svelte';
	import Dialog from '$lib/ui/Dialog.svelte';
	import { Button, TextInput } from '$lib/ui';
	import * as api from '$lib/api';
	import { m } from '$lib/paraglide/messages.js';

	let { baseDir, instanceId, onclose }: {
		baseDir: string;
		instanceId: string;
		onclose: () => void;
	} = $props();

	let report = $state<api.ModDependencyReport | null>(null);
	let error = $state('');
	let query = $state('');
	let issuesOnly = $state(false);

	function needsAttention(edge: api.ModDependencyEdge) {
		return (
			(edge.kind === 'required' && (edge.status === 'missing' || edge.status === 'disabled')) ||
			(edge.kind === 'incompatible' && (edge.status === 'installed' || edge.status === 'runtime'))
		);
	}

	const visible = $derived(
		(report?.relationships ?? []).filter((edge) => {
			const term = query.trim().toLowerCase();
			return (
				(!issuesOnly || needsAttention(edge)) &&
				[edge.modName, edge.modId, edge.dependencyId, edge.fileName].some((name) =>
					name.toLowerCase().includes(term)
				)
			);
		})
	);

	const statusLabels = {
		installed: m.editor_dependency_installed,
		disabled: m.editor_dependency_disabled,
		missing: m.editor_dependency_missing,
		runtime: m.editor_dependency_runtime
	};
	const kindLabels = {
		required: m.editor_dependency_required,
		optional: m.editor_dependency_optional,
		incompatible: m.editor_dependency_incompatible
	};

	onMount(() => {
		let active = true;
		api.checkModDependencies(baseDir, instanceId).then(
			(result) => {
				if (active) report = result;
			},
			(reason: unknown) => {
				if (active) error = api.errorMessage(reason);
			}
		);
		return () => { active = false; };
	});
</script>

<Dialog title={m.editor_dependency_check()} width="760px" {onclose}>
	<div class="flex min-h-0 flex-col gap-3 px-4.5 py-4">
		{#if error}
			<p class="text-danger" role="alert">{error}</p>
		{:else if !report}
			<p class="text-muted" role="status">{m.editor_dependency_scanning()}</p>
		{:else}
			<div class="flex flex-wrap items-center gap-3">
				<TextInput bind:value={query} placeholder={m.editor_dependency_search()} aria-label={m.editor_dependency_search()} />
				<label class="flex items-center gap-1.5">
					<input type="checkbox" bind:checked={issuesOnly} />
					{m.editor_dependency_issues_only()}
				</label>
			</div>
			<p class="text-muted">{m.editor_dependency_range_hint()}</p>
			<div class="max-h-[50vh] overflow-auto">
				{#if visible.length === 0}
					<p class="py-3 text-muted">{m.editor_dependency_empty()}</p>
				{:else}
					<ul class="divide-y divide-line">
						{#each visible as edge}
							<li class="flex flex-wrap justify-between gap-x-4 gap-y-1 py-2.5">
								<div class="min-w-0">
									<p class="break-all"><strong title={edge.fileName}>{edge.modName}</strong> → <code>{edge.dependencyId}</code></p>
									<p class="break-all text-muted">
										{kindLabels[edge.kind]()}
										{#if edge.requirement} · <code>{edge.requirement}</code>{/if}
									</p>
									{#each edge.providers as provider (provider.fileName)}
										<p class="break-all text-muted">
											{provider.fileName}{#if provider.version} · <code>{provider.version}</code>{/if}
											{#if !provider.enabled} · {statusLabels.disabled()}{/if}
										</p>
									{/each}
								</div>
								<span class={needsAttention(edge) ? 'text-danger' : 'text-muted'}>{statusLabels[edge.status]()}</span>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
			{#if report.unrecognizedFiles.length > 0}
				<details class="text-muted">
					<summary class="cursor-pointer">{m.editor_dependency_unrecognized({ count: report.unrecognizedFiles.length })}</summary>
					<ul class="max-h-24 overflow-auto">
						{#each report.unrecognizedFiles as file (file)}
							<li class="break-all">{file}</li>
						{/each}
					</ul>
				</details>
			{/if}
		{/if}
	</div>
	{#snippet footer()}
		<Button onclick={onclose}>{m.common_close()}</Button>
	{/snippet}
</Dialog>
