<script lang="ts">
	import { onMount } from 'svelte';
	import { Button, Dialog, Pill, Spinner } from '$lib/ui';
	import Icon from '$lib/ui/Icon.svelte';
	import * as api from '$lib/api';
	import type { Group, GroupMember } from '$lib/api';
	import { formatCount, releaseDate } from '$lib/format';
	import { groupKindName, handleAuthError, hub, refreshUnread, roleName } from '$lib/blueprints.svelte';
	import { navigate } from '$lib/nav';
	import { report, reportError } from '$lib/state.svelte';
	import { m } from '$lib/paraglide/messages.js';

	let { group, onchanged }: { group: Group; onchanged: (updated: Group) => void } = $props();

	let busy = $state('');
	let members = $state<GroupMember[] | null>(null);
	let confirmingLeave = $state(false);

	const signedIn = $derived(!!hub.session?.user);
	const isAdmin = $derived(group.viewerRole === 'owner' || group.viewerRole === 'admin');
	const webUrl = $derived(hub.session?.web ? `${hub.session.web}/groups/${group.id}` : null);

	onMount(async () => {
		// 選到群組時重新拿一次，才有自己的角色與邀請。
		try {
			onchanged(await api.groupGet(group.id));
		} catch {
			// 拿不到就先顯示列表裡的資料
		}
		try {
			members = (await api.groupMembers(group.id)).items;
		} catch {
			members = [];
		}
	});

	async function run(label: string, action: () => Promise<void>) {
		busy = label;
		try {
			await action();
		} catch (e) {
			handleAuthError(e);
			reportError(m.group_error(), e);
		} finally {
			busy = '';
		}
	}

	// 有邀請時 join 就是接受邀請。
	const join = () =>
		run('join', async () => {
			onchanged(await api.groupJoin(group.id));
			report(m.group_status_joined({ name: group.name }));
			refreshUnread();
		});

	const decline = () =>
		run('decline', async () => {
			await api.groupInviteRespond(group.viewerInvite!.id, false);
			onchanged({ ...group, viewerInvite: null });
			report(m.group_status_declined());
			refreshUnread();
		});

	const leave = () => {
		confirmingLeave = false;
		return run('leave', async () => {
			await api.groupLeave(group.id);
			onchanged(await api.groupGet(group.id));
			report(m.group_status_left({ name: group.name }));
		});
	};

	function browseBlueprints() {
		hub.groupFilter = { id: group.id, name: group.name };
		navigate('/blueprints');
	}

	const open = (url: string) => api.openExternal(url).catch((e) => reportError(m.group_error(), e));

	const LINK_ICONS: Record<string, string> = {
		website: 'globe',
		discord: 'link-discord',
		youtube: 'play',
		bilibili: 'link-bilibili',
		twitch: 'link-twitch',
		x: 'link-x',
		github: 'link-github',
		qq: 'link-qq'
	};
</script>

<div class="flex min-h-0 flex-1 flex-col gap-2.5">
	<div class="min-h-0 flex-1 overflow-y-auto rounded-lg border border-line bg-deep">
		<div class="relative aspect-[4/1] w-full bg-card">
			{#if group.bannerUrl}<img src={group.bannerUrl} alt="" class="absolute inset-0 size-full object-cover" />{/if}
		</div>
		<div class="flex flex-col gap-3 px-3.5 pb-3">
			<div class="-mt-6 flex items-end gap-2.5">
				{#if group.avatarUrl}
					<img src={group.avatarUrl} alt="" class="size-14 rounded-lg border-3 border-deep bg-card object-cover" />
				{:else}
					<span class="grid size-14 place-items-center rounded-lg border-3 border-deep bg-brand text-xl font-bold text-white">
						{[...group.name.trim()][0]?.toUpperCase() ?? '?'}
					</span>
				{/if}
				<div class="flex flex-wrap gap-1.5 pb-0.5">
					<Pill accent>{groupKindName(group.kind)}</Pill>
					<Pill>{group.joinPolicy === 'invite' ? m.group_invite_only() : m.group_open()}</Pill>
					{#if group.viewerRole}<Pill>{roleName(group.viewerRole)}</Pill>{/if}
				</div>
			</div>

			<div>
				<h3 class="text-[15px] font-semibold break-words text-ink">{group.name}</h3>
				<p class="text-faint">
					{m.group_members_count({ count: formatCount(group.memberCount) })} ·
					{m.group_blueprints_count({ count: formatCount(group.blueprintCount) })} ·
					{m.group_owner()} {group.owner.username} · {releaseDate(group.createdAt)}
				</p>
			</div>

			{#if group.viewerInvite}
				<div class="flex items-center gap-2 rounded-lg border border-brand-line bg-brand-soft px-3 py-2">
					<Icon name="bell" size={14} />
					<span class="min-w-0 flex-1 text-brand-fg">{m.group_invited_as({ role: roleName(group.viewerInvite.role) })}</span>
				</div>
			{/if}

			{#if group.description.trim()}
				<p class="leading-[1.75] break-words whitespace-pre-wrap text-ink">{group.description}</p>
			{:else}
				<p class="text-faint">{m.group_no_description()}</p>
			{/if}

			{#if group.links.length}
				<div class="flex flex-col gap-1">
					<p class="text-faint">{m.group_links()}</p>
					<div class="flex flex-wrap gap-1.5">
						{#each group.links as link, i (i)}
							<Button icon={LINK_ICONS[link.type] ?? 'link'} title={link.url} onclick={() => open(link.url)}>
								{link.label || link.type}
							</Button>
						{/each}
					</div>
				</div>
			{/if}

			<div class="flex flex-col gap-1">
				<p class="text-faint">{m.group_members()}</p>
				{#if members === null}
					<Spinner />
				{:else}
					<div class="flex flex-wrap gap-1.5">
						{#each members as member (member.uuid)}
							<Pill accent={member.role !== 'member'}>
								{#if member.role === 'owner'}<Icon name="crown" size={11} />{:else if member.role === 'admin'}<Icon name="shield" size={11} />{/if}
								{member.username}
							</Pill>
						{/each}
					</div>
				{/if}
			</div>
		</div>
	</div>

	<div class="flex shrink-0 flex-wrap items-center gap-1.75">
		{#if busy}<span class="flex items-center gap-2 text-muted"><Spinner /></span>{/if}
		{#if signedIn && group.viewerInvite}
			<Button variant="primary" icon="check" disabled={!!busy} onclick={join}>
				{m.group_accept_invite({ role: roleName(group.viewerInvite.role) })}
			</Button>
			<Button icon="close" disabled={!!busy} onclick={decline}>{m.group_decline()}</Button>
		{:else if signedIn && !group.viewerRole && group.joinPolicy === 'open'}
			<Button variant="primary" icon="plus" disabled={!!busy} onclick={join}>{m.group_join()}</Button>
		{/if}
		<Button icon="blocks" onclick={browseBlueprints}>{m.group_view_blueprints()}</Button>
		<span class="flex-1"></span>
		{#if webUrl}
			<Button icon="shortcut" onclick={() => open(webUrl)}>{isAdmin ? m.group_manage_web() : m.group_open_web()}</Button>
		{/if}
		{#if group.viewerRole && group.viewerRole !== 'owner'}
			<Button variant="danger" icon="logout" disabled={!!busy} onclick={() => (confirmingLeave = true)}>{m.group_leave()}</Button>
		{/if}
	</div>
</div>

{#if confirmingLeave}
	<Dialog title={m.group_leave_title()} width="420px" onclose={() => (confirmingLeave = false)}>
		<div class="px-5 py-5.5">
			<p class="mb-2 text-[15px] font-semibold">{m.group_leave_confirm({ name: group.name })}</p>
			<p class="leading-[1.75] text-muted">{m.group_leave_desc()}</p>
		</div>
		{#snippet footer()}
			<Button variant="destructive" onclick={leave}>{m.group_leave()}</Button>
			<Button onclick={() => (confirmingLeave = false)}>{m.common_cancel()}</Button>
		{/snippet}
	</Dialog>
{/if}
