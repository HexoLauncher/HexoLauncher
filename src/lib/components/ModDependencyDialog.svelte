<script lang="ts">
	import { onMount, type Snippet } from 'svelte';
	import Dialog from '$lib/ui/Dialog.svelte';
	import { Button, TextInput } from '$lib/ui';
	import * as api from '$lib/api';
	import { m } from '$lib/paraglide/messages.js';
	import Segmented from './tools/Segmented.svelte';
	import ModDependencyGraph from './ModDependencyGraph.svelte';
	import { layoutDependencyGraph, linkTargets } from './modDependencyGraph';

	let {
		baseDir,
		instanceId,
		onclose
	}: {
		baseDir: string;
		instanceId: string;
		onclose: () => void;
	} = $props();

	let report = $state<api.ModDependencyReport | null>(null);
	let error = $state('');
	let query = $state('');
	let issuesOnly = $state(false);
	let view = $state<'list' | 'graph'>('list');
	let selected = $state<string | null>(null);

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

	const names = $derived(
		new Map((report?.relationships ?? []).map((edge) => [edge.fileName, edge.modName]))
	);
	const graph = $derived(layoutDependencyGraph(visible, names, needsAttention));
	const selectedNode = $derived(graph.nodes.find((node) => node.id === selected));
	const selectedEdges = $derived(visible.filter((edge) => edge.fileName === selected));
	const dependentEdges = $derived(
		selected ? visible.filter((edge) => linkTargets(edge).includes(selected!)) : []
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
		return () => {
			active = false;
		};
	});
</script>

{#snippet edgeDetails(edge: api.ModDependencyEdge, showSource: boolean, providers?: Snippet)}
	<div class="flex flex-wrap justify-between gap-x-4 gap-y-1">
		<div class="min-w-0">
			<p class="break-all">
				{#if showSource}<strong title={edge.fileName}>{edge.modName}</strong> →
				{/if}
				<code>{edge.dependencyId}</code>
			</p>
			<p class="break-all text-muted">
				{kindLabels[edge.kind]()}
				{#if edge.requirement}
					· <code>{edge.requirement}</code>{/if}
			</p>
			{@render providers?.()}
		</div>
		<span class={needsAttention(edge) ? 'text-danger' : 'text-muted'}>
			{statusLabels[edge.status]()}
		</span>
	</div>
{/snippet}

{#snippet providerDetails(provider: api.ModDependencyEdge['providers'][number])}
	<span class="break-all text-muted">
		{provider.fileName}{#if provider.version}
			· <code>{provider.version}</code>{/if}
		{#if !provider.enabled}
			· {statusLabels.disabled()}{/if}
	</span>
{/snippet}

{#snippet edgeWithProviders(edge: api.ModDependencyEdge, showSource: boolean)}
	{#snippet providers()}
		{#each edge.providers as provider (provider.fileName)}
			<p>{@render providerDetails(provider)}</p>
		{/each}
	{/snippet}
	{@render edgeDetails(edge, showSource, providers)}
{/snippet}

{#snippet legendLine(kind: 'solid' | 'dashed' | 'dotted', tone: string, label: string)}
	<span class="flex items-center gap-1.5">
		<svg width="22" height="6" aria-hidden="true">
			<line
				x1="1"
				y1="3"
				x2="21"
				y2="3"
				class={tone}
				stroke-width="1.5"
				stroke-linecap="round"
				stroke-dasharray={kind === 'dashed' ? '6 4' : kind === 'dotted' ? '1 4' : undefined}
			/>
		</svg>
		{label}
	</span>
{/snippet}

<Dialog title={m.editor_dependency_check()} width="900px" {onclose}>
	<div class="flex min-h-0 flex-col gap-3 px-4.5 py-4">
		{#if error}
			<p class="text-danger" role="alert">{error}</p>
		{:else if !report}
			<p class="text-muted" role="status">{m.editor_dependency_scanning()}</p>
		{:else}
			<div class="flex flex-wrap items-center gap-3">
				<TextInput
					bind:value={query}
					placeholder={m.editor_dependency_search()}
					aria-label={m.editor_dependency_search()}
				/>
				<label class="flex items-center gap-1.5">
					<input type="checkbox" bind:checked={issuesOnly} />
					{m.editor_dependency_issues_only()}
				</label>
				<Segmented
					bind:value={view}
					label={m.editor_dependency_view()}
					options={[
						{ value: 'list', label: m.blueprint_view_list() },
						{ value: 'graph', label: m.editor_dependency_graph() }
					]}
				/>
			</div>
			{#if visible.length === 0}
				<p class="py-3 text-muted">{m.editor_dependency_empty()}</p>
			{:else if view === 'graph'}
				<div class="flex flex-wrap gap-x-4 gap-y-1 text-muted">
					{@render legendLine('solid', 'stroke-faint', m.editor_dependency_required())}
					{@render legendLine('dashed', 'stroke-faint', m.editor_dependency_optional())}
					{@render legendLine('solid', 'stroke-warn', m.editor_dependency_incompatible())}
					{@render legendLine('solid', 'stroke-danger', m.editor_dependency_missing())}
					{@render legendLine('dotted', 'stroke-faint', m.editor_dependency_cycle())}
				</div>
				<ModDependencyGraph layout={graph} bind:selected />
				<div class="max-h-[22vh] overflow-auto">
					{#if !selectedNode}
						<p class="text-muted">{m.editor_dependency_graph_hint()}</p>
					{:else}
						<p class="break-all">
							<strong>{selectedNode.label}</strong>
							{#if selectedNode.state !== 'missing'}
								<span class="text-muted">· {selectedNode.id}</span>
							{/if}
						</p>
						<ul class="divide-y divide-line">
							{#each selectedEdges as edge (edge)}
								<li class="py-2">{@render edgeWithProviders(edge, false)}</li>
							{/each}
						</ul>
						{#if dependentEdges.length}
							<p class="mt-2 text-muted">{m.editor_dependency_dependents()}</p>
							<ul class="divide-y divide-line">
								{#each dependentEdges as edge (edge)}
									<li class="py-2">{@render edgeDetails(edge, true)}</li>
								{/each}
							</ul>
						{/if}
					{/if}
				</div>
			{:else}
				<ul class="max-h-[50vh] divide-y divide-line overflow-auto">
					{#each visible as edge (edge)}
						<li class="py-2.5">{@render edgeWithProviders(edge, true)}</li>
					{/each}
				</ul>
			{/if}
			{#if report.unrecognizedFiles.length > 0}
				<details class="text-muted">
					<summary class="cursor-pointer"
						>{m.editor_dependency_unrecognized({ count: report.unrecognizedFiles.length })}</summary
					>
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
