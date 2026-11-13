<script lang="ts">
	import { onMount } from 'svelte';
	import { Button, Select, Spinner, TextInput } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import GroupDetail from './GroupDetail.svelte';
	import * as api from '$lib/api';
	import type { Group } from '$lib/api';
	import { formatCount } from '$lib/format';
	import { groupKindName, hub, refreshGroupKinds } from '$lib/blueprints.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const PER_PAGE = 24;

	let query = $state('');
	let kind = $state('');
	let mine = $state(false);

	let items = $state<Group[]>([]);
	let total = $state(0);
	let pageNo = $state(1);
	let loading = $state(false);
	let message = $state('');
	let selected = $state<Group | null>(null);

	let timer: ReturnType<typeof setTimeout> | undefined;
	let seq = 0;
	const me = $derived(hub.session?.user?.uuid ?? null);

	onMount(() => {
		if (!hub.groupKinds.length) refreshGroupKinds();
		search();
		return () => clearTimeout(timer);
	});

	function searchSoon() {
		clearTimeout(timer);
		timer = setTimeout(() => search(), 400);
	}

	async function search(append = false) {
		clearTimeout(timer);
		const current = ++seq;
		const next = append ? pageNo + 1 : 1;
		loading = true;
		message = '';
		try {
			const result = await api.groupList({
				q: query.trim(),
				kind: kind || null,
				member: mine ? me : null,
				page: next,
				perPage: PER_PAGE
			});
			if (current !== seq) return;
			items = append ? [...items, ...result.items] : result.items;
			total = result.total;
			pageNo = next;
		} catch (e) {
			if (current !== seq) return;
			if (!append) items = [];
			message = api.errorMessage(e);
		} finally {
			if (current === seq) loading = false;
		}
	}

	function replace(updated: Group) {
		items = items.map((g) => (g.id === updated.id ? updated : g));
		if (selected?.id === updated.id) selected = updated;
	}
</script>

{#if selected}
	<div class="flex min-h-0 flex-1 flex-col gap-2.5">
		<div class="flex shrink-0">
			<Button icon="back" onclick={() => (selected = null)}>{m.group_back()}</Button>
		</div>
		<div class="mx-auto flex min-h-0 w-full max-w-200 flex-1 flex-col">
			{#key selected.id}
				<GroupDetail group={selected} onchanged={replace} />
			{/key}
		</div>
	</div>
{/if}

<div class="min-h-0 flex-1 flex-col gap-2.5 {selected ? 'hidden' : 'flex'}">
	<div class="flex shrink-0 gap-1.75">
		<TextInput
			class="min-w-0 flex-1"
			bind:value={query}
			oninput={searchSoon}
			onkeydown={(e) => e.key === 'Enter' && search()}
			placeholder={m.group_search_placeholder()}
			aria-label={m.group_search_placeholder()}
		/>
		<Select class="w-32" bind:value={kind} onchange={() => search()} aria-label={m.group_any_kind()}>
			<option value="">{m.group_any_kind()}</option>
			{#each hub.groupKinds as k (k.slug)}
				<option value={k.slug}>{k.name}</option>
			{/each}
		</Select>
		{#if me}
			<Button
				icon="user"
				aria-pressed={mine}
				class={mine ? 'bg-brand-soft text-brand-fg' : ''}
				onclick={() => {
					mine = !mine;
					search();
				}}
			>
				{m.group_mine()}
			</Button>
		{/if}
		<Button icon="refresh" onclick={() => search()} aria-label={m.common_refresh()} title={m.common_refresh()}></Button>
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto" aria-busy={loading}>
		<div class="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-2.5">
			{#each items as group (group.id)}
				<button
					class="flex flex-col overflow-hidden rounded-lg border border-line bg-deep text-left transition-colors duration-150 hover:border-line-hi"
					onclick={() => (selected = group)}
				>
					<span class="relative block aspect-[4/1] w-full bg-card">
						{#if group.bannerUrl}
							<img src={group.bannerUrl} alt="" loading="lazy" class="absolute inset-0 size-full object-cover" />
						{/if}
					</span>
					<span class="flex flex-col gap-2 px-3 pb-3">
						<span class="-mt-5 flex items-end gap-2">
							{#if group.avatarUrl}
								<img src={group.avatarUrl} alt="" loading="lazy" class="size-11 shrink-0 rounded-lg border-3 border-deep bg-card object-cover" />
							{:else}
								<span class="grid size-11 shrink-0 place-items-center rounded-lg border-3 border-deep bg-brand font-bold text-white">
									{[...group.name.trim()][0]?.toUpperCase() ?? '?'}
								</span>
							{/if}
							<span class="min-w-0 truncate pb-0.5 text-[11.5px] text-faint">{groupKindName(group.kind)}</span>
						</span>
						<span class="truncate font-semibold text-ink">{group.name}</span>
						<span class="line-clamp-2 min-h-[2lh] text-[12px] leading-normal text-muted">{group.description}</span>
						<span class="flex items-center gap-3 text-[11.5px] text-faint">
							<span class="flex items-center gap-1"><Icon name="users" size={11} />{formatCount(group.memberCount)}</span>
							<span class="flex items-center gap-1"><Icon name="blocks" size={11} />{formatCount(group.blueprintCount)}</span>
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
				<span>{m.group_empty()}</span>
			{:else if items.length < total}
				<Button onclick={() => search(true)}>{m.list_load_more({ loaded: items.length, total })}</Button>
			{:else}
				<span class="text-faint">{m.list_total({ total })}</span>
			{/if}
		</div>
	</div>
</div>
