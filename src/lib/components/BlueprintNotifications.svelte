<script lang="ts">
	import { onMount } from 'svelte';
	import { Button, Dialog, Spinner } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import * as api from '$lib/api';
	import type { BlueprintNotification } from '$lib/api';
	import { releaseDate } from '$lib/format';
	import { handleAuthError, hub, roleName } from '$lib/blueprints.svelte';
	import { navigate } from '$lib/nav';
	import { report, reportError } from '$lib/state.svelte';
	import { m } from '$lib/paraglide/messages.js';

	let { onclose }: { onclose: () => void } = $props();

	let items = $state<BlueprintNotification[] | null>(null);
	let total = $state(0);
	let pageNo = $state(1);
	let busy = $state('');

	onMount(() => load());

	async function load(append = false) {
		const next = append ? pageNo + 1 : 1;
		try {
			const list = await api.notificationList(next);
			items = append ? [...(items ?? []), ...list.items] : list.items;
			total = list.total;
			pageNo = next;
			hub.unread = list.unread;
		} catch (e) {
			handleAuthError(e);
			reportError(m.notification_error(), e);
			items ??= [];
		}
	}

	async function run(label: string, action: () => Promise<void>) {
		busy = label;
		try {
			await action();
		} catch (e) {
			handleAuthError(e);
			reportError(m.notification_error(), e);
		} finally {
			busy = '';
		}
	}

	const markRead = (n: BlueprintNotification) => {
		if (!n.readAt) {
			n.readAt = new Date().toISOString();
			hub.unread = Math.max(0, hub.unread - 1);
		}
	};

	function respond(n: BlueprintNotification, accept: boolean) {
		if (n.kind !== 'group_invite') return;
		const data = n.data;
		return run(n.id, async () => {
			await api.groupInviteRespond(data.inviteId, accept);
			data.status = accept ? 'accepted' : 'declined';
			markRead(n);
			report(accept ? m.group_status_joined({ name: data.group.name }) : m.group_status_declined());
		});
	}

	const read = (n: BlueprintNotification) =>
		run(n.id, async () => {
			await api.notificationRead(n.id);
			markRead(n);
		});

	const remove = (n: BlueprintNotification) =>
		run(n.id, async () => {
			await api.notificationDelete(n.id);
			if (!n.readAt) hub.unread = Math.max(0, hub.unread - 1);
			items = (items ?? []).filter((i) => i.id !== n.id);
			total = Math.max(0, total - 1);
		});

	const readAll = () =>
		run('all', async () => {
			await api.notificationRead(null);
			for (const n of items ?? []) n.readAt ??= new Date().toISOString();
			hub.unread = 0;
		});

	function openGroups() {
		onclose();
		navigate('/blueprints/groups');
	}

	const STATUS: Record<string, () => string> = {
		accepted: m.notification_status_accepted,
		declined: m.notification_status_declined,
		cancelled: m.notification_status_cancelled
	};
</script>

<Dialog title={m.notification_title()} width="520px" height="min(560px, 85vh)" {onclose}>
	<div class="flex min-h-0 flex-1 flex-col overflow-y-auto">
		{#if items === null}
			<span class="grid flex-1 place-items-center py-10"><Spinner size={20} /></span>
		{:else if items.length === 0}
			<p class="grid flex-1 place-items-center py-10 text-faint">{m.notification_empty()}</p>
		{:else}
			{#each items as n (n.id)}
				<div class="group flex gap-2.5 border-b border-line px-4 py-3 last:border-0">
					<span class="mt-1.5 size-2 shrink-0 rounded-full {n.readAt ? 'bg-transparent' : 'bg-brand'}"></span>
					<div class="flex min-w-0 flex-1 flex-col gap-1.5">
						{#if n.kind === 'group_invite'}
							<p class="leading-[1.6] text-ink">
								{m.notification_invite({ inviter: n.data.invitedBy.username, role: roleName(n.data.role), group: n.data.group.name })}
							</p>
							{#if n.data.status === 'pending'}
								<div class="flex gap-1.5">
									<Button variant="primary" icon="check" disabled={!!busy} onclick={() => respond(n, true)}>
										{m.notification_accept()}
									</Button>
									<Button icon="close" disabled={!!busy} onclick={() => respond(n, false)}>{m.notification_decline()}</Button>
								</div>
							{:else}
								<p class="text-[11.5px] text-faint">{STATUS[n.data.status]?.() ?? n.data.status}</p>
							{/if}
						{:else if n.kind === 'group_invite_accepted' || n.kind === 'group_invite_declined'}
							{@const text = n.kind === 'group_invite_accepted' ? m.notification_invite_accepted : m.notification_invite_declined}
							<p class="leading-[1.6] text-ink">{text({ user: n.data.user.username, group: n.data.group.name })}</p>
						{:else}
							<p class="text-muted">{m.notification_unknown()}</p>
						{/if}
						<p class="text-[11.5px] text-faint">{releaseDate(n.createdAt)}</p>
					</div>
					<div class="flex shrink-0 items-start gap-1 opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100">
						{#if !n.readAt}
							<button class="grid size-7 place-items-center rounded-md text-muted hover:bg-btn hover:text-ink" title={m.notification_read_all()} aria-label={m.notification_read_all()} disabled={!!busy} onclick={() => read(n)}>
								<Icon name="check" size={14} />
							</button>
						{/if}
						<button class="grid size-7 place-items-center rounded-md text-muted hover:bg-btn hover:text-ink" title={m.notification_delete()} aria-label={m.notification_delete()} disabled={!!busy} onclick={() => remove(n)}>
							<Icon name="trash" size={14} />
						</button>
					</div>
				</div>
			{/each}
			{#if items.length < total}
				<div class="flex justify-center py-3">
					<Button onclick={() => load(true)}>{m.list_load_more({ loaded: items.length, total })}</Button>
				</div>
			{/if}
		{/if}
	</div>
	{#snippet footer()}
		<Button icon="users" onclick={openGroups}>{m.blueprint_tab_groups()}</Button>
		<span class="flex-1"></span>
		<Button icon="check-all" disabled={!!busy || hub.unread === 0} onclick={readAll}>{m.notification_read_all()}</Button>
		<Button onclick={onclose}>{m.common_cancel()}</Button>
	{/snippet}
</Dialog>
