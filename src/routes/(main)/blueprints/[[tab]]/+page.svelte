<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { Button, Spinner } from '$lib/ui';
	import NavList from '$lib/ui/NavList.svelte';
	import type { Page } from '$lib/ui';
	import BlueprintBrowser from '$lib/components/BlueprintBrowser.svelte';
	import BlueprintUploadDialog from '$lib/components/BlueprintUploadDialog.svelte';
	import InstanceSchematics from '$lib/components/InstanceSchematics.svelte';
	import PlayerHead from '$lib/PlayerHead.svelte';
	import { navigate } from '$lib/nav';
	import { hub, refreshSession, refreshTags, signIn, signOut } from '$lib/blueprints.svelte';
	import { report, reportError, selectedAccount } from '$lib/state.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const pages: Page[] = [
		{ id: 'browse', label: m.blueprint_tab_browse(), icon: 'globe' },
		{ id: 'mine', label: m.blueprint_tab_mine(), icon: 'upload' },
		{ id: 'instances', label: m.blueprint_tab_instances(), icon: 'box' }
	];

	const tab = $derived(pages.some((p) => p.id === page.params.tab) ? page.params.tab! : 'browse');
	const account = $derived(selectedAccount());
	const user = $derived(hub.session?.user ?? null);

	let upload = $state<{ path: string; mcVersion: string } | null>(null);

	let refreshKey = $state(0);


	$effect(() => {
		void account?.id;
		refreshSession();
	});

	onMount(() => {
		refreshTags();
	});

	async function login() {
		try {
			await signIn();
			report(m.blueprint_status_signed_in({ name: hub.session?.user?.username ?? '' }));
		} catch (e) {
			reportError(m.blueprint_error_sign_in(), e);
		}
	}

	async function logout() {
		try {
			await signOut();
			report(m.blueprint_status_signed_out());
		} catch (e) {
			reportError(m.blueprint_error_sign_in(), e);
		}
	}

	function retry() {
		hub.offline = '';
		refreshTags();
		refreshSession();
		refreshKey++;
	}
</script>

<svelte:head><title>{m.nav_blueprints()} · Nolauncher</title></svelte:head>

<div class="flex min-h-0 flex-1 bg-window">
	<NavList
		{pages}
		selected={tab}
		onselect={(id) => navigate(id === 'browse' ? '/blueprints' : `/blueprints/${id}`)}
		width="172px"
	/>

	<div class="flex min-w-0 flex-1 flex-col gap-2.5 px-4 pt-3 pb-3.5">
		<header class="flex shrink-0 items-center gap-2.5">
			<h2 class="text-[15px] font-semibold">{pages.find((p) => p.id === tab)?.label}</h2>
			<span class="flex-1"></span>

			<span class="flex min-w-0 items-center gap-2 text-muted">
				<span class="grid place-items-center rounded-full bg-card p-0.75">
					<PlayerHead {account} size={18} />
				</span>
				{#if user}
					<span class="truncate">{m.blueprint_signed_in_as({ name: user.username })}</span>
					<Button icon="logout" onclick={logout}>{m.blueprint_sign_out()}</Button>
				{:else if hub.session?.account?.online}
					<span class="truncate">{m.blueprint_not_signed_in()}</span>
					<Button icon="login" disabled={hub.signingIn} onclick={login}>
						{#if hub.signingIn}<Spinner />{/if}
						{m.blueprint_sign_in()}
					</Button>
				{:else if hub.session?.account}
					<span class="truncate" title={m.blueprint_offline_account_hint()}>
						{m.blueprint_offline_account()}
					</span>
				{:else}
					<span class="truncate">{m.account_not_signed_in()}</span>
					<Button onclick={() => navigate('/settings/accounts')}>{m.account_manage()}</Button>
				{/if}
			</span>

			{#if tab !== 'instances'}
				<Button
					variant="primary"
					icon="upload"
					disabled={!user}
					title={user ? undefined : m.blueprint_sign_in_to_upload()}
					onclick={() => (upload = { path: '', mcVersion: '' })}
				>
					{m.blueprint_upload()}
				</Button>
			{/if}
		</header>

		{#if hub.offline}
			<div
				class="flex shrink-0 items-center gap-2.5 rounded-lg border border-line bg-danger-soft px-3 py-2"
				role="alert"
			>
				<span class="min-w-0 flex-1 leading-[1.6]">
					{m.blueprint_server_unreachable({ server: hub.session?.server ?? '' })}
					<span class="text-muted">{hub.offline}</span>
				</span>
				<Button onclick={retry}>{m.common_retry()}</Button>
			</div>
		{/if}

		{#if tab === 'instances'}
			<InstanceSchematics onupload={(file, version) => (upload = { path: file.path, mcVersion: version })} />
		{:else if tab === 'mine' && !user}
			<div class="grid flex-1 place-items-center text-center text-muted">
				<div class="flex flex-col items-center gap-3">
					<p>{m.blueprint_mine_sign_in()}</p>
					{#if hub.session?.account?.online}
						<Button variant="primary" icon="login" disabled={hub.signingIn} onclick={login}>
							{m.blueprint_sign_in()}
						</Button>
					{/if}
				</div>
			</div>
		{:else}
			{#key `${tab}:${refreshKey}`}
				<BlueprintBrowser owner={tab === 'mine' ? (user?.uuid ?? null) : null} />
			{/key}
		{/if}
	</div>
</div>

{#if upload}
	<BlueprintUploadDialog
		initialPath={upload.path}
		initialVersion={upload.mcVersion}
		onclose={() => (upload = null)}
		onuploaded={(bp, warning) => {
			upload = null;
			refreshKey++;
			if (warning) reportError(m.blueprint_error_images(), warning);
			else report(m.blueprint_status_uploaded({ name: bp.name }));
			if (tab !== 'mine') navigate('/blueprints/mine');
		}}
	/>
{/if}

<svelte:window
	onkeydown={(e) => {
		if (e.key === 'F5' && !upload) {
			e.preventDefault();
			retry();
		}
	}}
/>
