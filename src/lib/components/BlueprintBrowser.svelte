<script lang="ts">
	import { onMount } from 'svelte';
	import { Button, Select, Spinner, TextInput } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import BlueprintDetail from './BlueprintDetail.svelte';
	import * as api from '$lib/api';
	import type { Blueprint } from '$lib/api';
	import { formatCount } from '$lib/format';
	import { hub, instanceVersions } from '$lib/blueprints.svelte';
	import { app } from '$lib/state.svelte';
	import { releaseVersions, withCurrent } from '$lib/versions';
	import { m } from '$lib/paraglide/messages.js';

	/** owner 有值時只列這個人的藍圖（我的上傳）。 */
	let { owner = null }: { owner?: string | null } = $props();

	const PER_PAGE = 24;

	let query = $state('');
	let version = $state('');
	let tags = $state<string[]>([]);

	let items = $state<Blueprint[]>([]);
	let total = $state(0);
	let pageNo = $state(1);
	let loading = $state(false);
	let message = $state('');
	let selected = $state<Blueprint | null>(null);

	let list = $state<HTMLElement>();
	let timer: ReturnType<typeof setTimeout> | undefined;
	let seq = 0;

	let releases = $state<string[]>([]);
	releaseVersions().then((list) => (releases = list)).catch(() => {});
	const versions = $derived(withCurrent(releases.length ? releases : instanceVersions(app.instances), version));
	const hasMore = $derived(items.length < total);

	onMount(() => {
		search();
		return () => clearTimeout(timer);
	});

	function searchSoon() {
		clearTimeout(timer);
		timer = setTimeout(() => search(), 400);
	}

	function toggleTag(slug: string) {
		tags = tags.includes(slug) ? tags.filter((t) => t !== slug) : [...tags, slug];
		search();
	}

	async function search(append = false) {
		clearTimeout(timer);
		const current = ++seq;
		const next = append ? pageNo + 1 : 1;
		loading = true;
		message = '';
		try {
			const result = await api.blueprintSearch({
				q: query.trim(),
				owner,
				tags,
				version: version || null,
				page: next,
				perPage: PER_PAGE
			});
			if (current !== seq) return;

			items = append ? [...items, ...result.items] : result.items;
			total = result.total;
			pageNo = next;
			if (!append) {
				list?.scrollTo({ top: 0 });
				if (selected && !items.some((b) => b.id === selected?.id)) selected = null;
			}
		} catch (e) {
			if (current !== seq) return;
			if (!append) {
				items = [];
				total = 0;
			}
			message = api.errorMessage(e);
		} finally {
			if (current === seq) loading = false;
		}
	}

	function replace(updated: Blueprint) {
		items = items.map((b) => (b.id === updated.id ? updated : b));
		if (selected?.id === updated.id) selected = updated;
	}

	function remove(id: string) {
		items = items.filter((b) => b.id !== id);
		total = Math.max(0, total - 1);
		if (selected?.id === id) selected = null;
	}
</script>

<div class="flex min-h-0 flex-1 flex-col gap-2.5">
	<div class="flex shrink-0 gap-1.75">
		<TextInput
			class="min-w-0 flex-1"
			bind:value={query}
			oninput={searchSoon}
			onkeydown={(e) => e.key === 'Enter' && search()}
			placeholder={m.blueprint_search_placeholder()}
			aria-label={m.blueprint_search_placeholder()}
		/>
		<Select class="w-36" bind:value={version} onchange={() => search()} aria-label={m.common_version()}>
			<option value="">{m.blueprint_any_version()}</option>
			{#each versions as v (v)}
				<option value={v}>{m.blueprint_supports_version({ version: v })}</option>
			{/each}
		</Select>
		<Button icon="refresh" onclick={() => search()} aria-label={m.common_refresh()} title={m.common_refresh()}></Button>
	</div>

	{#if hub.tags.length}
		<div class="flex shrink-0 flex-wrap gap-1.5" role="group" aria-label={m.blueprint_tags()}>
			{#each hub.tags as tag (tag.slug)}
				{@const on = tags.includes(tag.slug)}
				<button
					class="rounded-full px-2.5 py-0.75 text-[11.5px] font-medium transition-colors duration-150
						{on ? 'bg-brand-soft text-brand-fg' : 'bg-btn text-muted hover:bg-btn-hi hover:text-ink'}"
					aria-pressed={on}
					onclick={() => toggleTag(tag.slug)}
				>
					{tag.name}
				</button>
			{/each}
		</div>
	{/if}

	<div class="flex min-h-0 flex-1 gap-2.5">
		<div
			bind:this={list}
			class="min-w-0 flex-[1.15] overflow-y-auto rounded-lg border border-line bg-deep p-2"
			aria-busy={loading}
		>
			<div class="grid grid-cols-[repeat(auto-fill,minmax(168px,1fr))] gap-2">
				{#each items as item (item.id)}
					{@const active = selected?.id === item.id}
					<button
						class="flex flex-col overflow-hidden rounded-lg border text-left transition-colors duration-150
							{active ? 'border-brand-line bg-brand-soft' : 'border-line bg-window hover:border-line-hi'}"
						onclick={() => (selected = item)}
					>
						<span class="relative block aspect-[16/10] w-full bg-card">
							{#if item.cover.url}
								<img
									src={item.cover.url}
									alt=""
									loading="lazy"
									class="absolute inset-0 size-full object-cover"
								/>
							{:else}
								<span class="absolute inset-0 grid place-items-center text-faint">
									{#if item.render.status === 'pending' || item.render.status === 'rendering'}
										<span class="flex items-center gap-1.5 text-[11.5px]"><Spinner size={12} />{m.blueprint_render_pending()}</span>
									{:else}
										<Icon name="blocks" size={26} />
									{/if}
								</span>
							{/if}
							<span
								class="absolute top-1.5 left-1.5 rounded-sm bg-scrim px-1.5 py-px font-mono text-[10.5px] text-white uppercase"
							>
								{item.format}
							</span>
						</span>
						<span class="flex flex-col gap-0.5 px-2.25 py-1.75">
							<span class="truncate font-semibold {active ? 'text-brand-fg' : 'text-ink'}">{item.name}</span>
							<span class="flex items-center gap-2 text-[11.5px] text-faint">
								<span class="min-w-0 truncate">{item.owner.username}</span>
								<span class="ml-auto flex shrink-0 items-center gap-1">
									<Icon name="download" size={11} />{formatCount(item.downloadCount)}
								</span>
							</span>
						</span>
					</button>
				{/each}
			</div>

			<div class="flex flex-col items-center gap-2 px-3 py-3 text-muted">
				{#if loading}
					<span class="flex items-center gap-2"><Spinner size={16} />{m.common_searching()}</span>
				{:else if message}
					<span class="leading-[1.7] text-danger">{message}</span>
					<Button onclick={() => search(items.length > 0)}>{m.common_retry()}</Button>
				{:else if items.length === 0}
					<span>{owner ? m.blueprint_mine_empty() : m.blueprint_empty()}</span>
				{:else if hasMore}
					<Button onclick={() => search(true)}>
						{m.list_load_more({ loaded: items.length, total })}
					</Button>
				{:else}
					<span class="text-faint">{m.list_total({ total })}</span>
				{/if}
			</div>
		</div>

		<div class="flex min-w-0 flex-1 flex-col">
			{#if selected}
				{#key selected.id}
					<BlueprintDetail blueprint={selected} onchanged={replace} ondeleted={remove} />
				{/key}
			{:else}
				<p
					class="grid flex-1 place-items-center rounded-lg border border-line bg-deep px-4 text-center text-faint"
				>
					{m.blueprint_pick()}
				</p>
			{/if}
		</div>
	</div>
</div>
