<script lang="ts">
	import { onMount } from 'svelte';
	import { Button, Dialog, Pill, Spinner, TextInput } from '$lib/ui';
	import SkinPreview from '$lib/SkinPreview.svelte';
	import { refreshPlayerHead } from '$lib/PlayerHead.svelte';
	import * as api from '$lib/api';
	import type { AccountView, SkinProfile, SkinVariant, WardrobeSkin } from '$lib/api';
	import { m } from '$lib/paraglide/messages.js';

	let {
		account,
		onclose,
		report
	}: {
		account: AccountView;
		onclose: () => void;
		report: (message: string) => void;
	} = $props();

	let profile = $state<SkinProfile | null>(null);
	let skins = $state<WardrobeSkin[]>([]);
	let selectedId = $state<string | null>(null);
	let editName = $state('');
	let busy = $state('');
	let error = $state('');

	const selected = $derived(skins.find((s) => s.id === selectedId) ?? null);
	const activeSha = $derived(profile?.skin?.sha256 ?? '');
	const currentSaved = $derived(!!activeSha && skins.some((s) => s.sha256 === activeSha));

	$effect(() => {
		editName = selected?.name ?? '';
	});

	onMount(() => {
		run(m.wardrobe_busy_loading(), async () => {
			const [p, list] = await Promise.all([api.skinProfile(account.id), api.wardrobeList()]);
			profile = p;
			skins = list;
		});
	});

	async function run(label: string, action: () => Promise<void>) {
		busy = label;
		error = '';
		try {
			await action();
		} catch (e) {
			error = api.errorMessage(e);
		} finally {
			busy = '';
		}
	}

	async function detectVariant(src: string): Promise<SkinVariant> {
		const img = new Image();
		img.src = src;
		await img.decode();
		if (img.naturalHeight !== 64) return 'classic';
		const canvas = document.createElement('canvas');
		canvas.width = 64;
		canvas.height = 64;
		const ctx = canvas.getContext('2d');
		if (!ctx) return 'classic';
		ctx.drawImage(img, 0, 0);
		return ctx.getImageData(54, 20, 1, 1).data[3] === 0 ? 'slim' : 'classic';
	}

	async function addSkins() {
		let picked: string[] = [];
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const chosen = await open({
				multiple: true,
				filters: [{ name: m.wardrobe_file_filter(), extensions: ['png'] }]
			});
			picked = Array.isArray(chosen) ? chosen : typeof chosen === 'string' ? [chosen] : [];
		} catch {
			error = m.common_no_dialog();
			return;
		}
		if (!picked.length) return;

		await run(m.wardrobe_busy_adding(), async () => {
			const failures: string[] = [];
			for (const path of picked) {
				const name = path.split(/[\\/]/).pop()?.replace(/\.png$/i, '') ?? '';
				try {
					const variant = await detectVariant(await api.readImageFile(path));
					skins = await api.wardrobeAdd(path, name, variant);
				} catch (e) {
					failures.push(`${name}: ${api.errorMessage(e)}`);
				}
			}
			if (failures.length) throw new Error(failures.join('\n'));
		});
	}

	function saveCurrent() {
		run(m.wardrobe_busy_adding(), async () => {
			skins = await api.wardrobeSaveCurrent(account.id, m.wardrobe_current_name({ name: account.playerName }));
		});
	}

	function update(variant: SkinVariant = selected?.variant ?? 'classic') {
		const skin = selected;
		if (!skin) return;
		const name = editName.trim() || skin.name;
		if (name === skin.name && variant === skin.variant) return;
		run(m.wardrobe_busy_saving(), async () => {
			skins = await api.wardrobeUpdate(skin.id, name, variant);
		});
	}

	function remove() {
		const skin = selected;
		if (!skin) return;
		run(m.wardrobe_busy_saving(), async () => {
			skins = await api.wardrobeRemove(skin.id);
			selectedId = null;
		});
	}

	function apply() {
		const skin = selected;
		if (!skin) return;
		run(m.wardrobe_busy_applying(), async () => {
			profile = await api.wardrobeApply(account.id, skin.id);
			refreshPlayerHead(account.uuid, skin.image);
			report(m.wardrobe_status_applied({ name: skin.name }));
		});
	}

	function chooseCape(capeId: string | null) {
		run(m.wardrobe_busy_applying(), async () => {
			profile = await api.setCape(account.id, capeId);
			report(m.wardrobe_status_cape());
		});
	}
</script>

{#snippet cape(image: string)}
	<svg width="30" height="48" viewBox="1 1 10 16" aria-hidden="true" class="block [image-rendering:pixelated]">
		<image href={image} width="64" height="32" />
	</svg>
{/snippet}

<Dialog
	title={m.wardrobe_title({ name: account.playerName })}
	width="780px"
	height="580px"
	onclose={() => !busy && onclose()}
>
	<div class="flex min-h-0 flex-1">
		<aside class="flex w-56 shrink-0 flex-col gap-3 overflow-y-auto border-r border-line px-4 py-4">
			<h3 class="font-semibold">{m.wardrobe_current()}</h3>
			<div class="grid h-52 place-items-center rounded-lg border border-line bg-deep">
				{#if profile?.skin}
					<SkinPreview
						src={profile.skin.image}
						slim={profile.skin.variant === 'slim'}
						legacy={profile.skin.legacy}
						height={180}
					/>
				{:else if profile}
					<p class="text-faint">{m.wardrobe_no_skin()}</p>
				{:else}
					<Spinner />
				{/if}
			</div>
			{#if profile?.skin}
				<div class="flex items-center gap-2">
					<Pill>{profile.skin.variant === 'slim' ? m.wardrobe_slim() : m.wardrobe_classic()}</Pill>
					<span class="flex-1"></span>
					<Button icon="download" disabled={!!busy || currentSaved} onclick={saveCurrent}>
						{m.wardrobe_save_current()}
					</Button>
				</div>
			{/if}

			<h3 class="mt-2 font-semibold">{m.wardrobe_capes()}</h3>
			{#if profile && profile.capes.length === 0}
				<p class="text-faint">{m.wardrobe_no_capes()}</p>
			{:else if profile}
				{@const none = !profile.capes.some((c) => c.active)}
				<div class="flex flex-wrap gap-1.5">
					<button
						class="grid h-15 w-11 place-items-center rounded-md border bg-deep text-[11px] text-muted
							{none ? 'border-brand' : 'border-line hover:border-ink/30'}"
						title={m.wardrobe_cape_none()}
						aria-label={m.wardrobe_cape_none()}
						aria-pressed={none}
						disabled={!!busy}
						onclick={() => !none && chooseCape(null)}
					>
						—
					</button>
					{#each profile.capes as item (item.id)}
						<button
							class="grid h-15 w-11 place-items-center rounded-md border bg-deep
								{item.active ? 'border-brand' : 'border-line hover:border-ink/30'}"
							title={item.alias}
							aria-label={item.alias}
							aria-pressed={item.active}
							disabled={!!busy}
							onclick={() => !item.active && chooseCape(item.id)}
						>
							{@render cape(item.image)}
						</button>
					{/each}
				</div>
			{/if}
		</aside>

		<section class="flex min-w-0 flex-1 flex-col gap-3 px-4 py-4">
			<header class="flex shrink-0 items-center gap-2">
				<h3 class="font-semibold">{m.wardrobe_mine()}</h3>
				<span class="text-faint">{skins.length}</span>
				<span class="flex-1"></span>
				<Button icon="plus" disabled={!!busy} onclick={addSkins}>{m.wardrobe_add()}</Button>
			</header>

			{#if skins.length === 0 && profile}
				<div class="grid flex-1 place-items-center px-6 text-center text-muted">
					<p class="leading-[1.7]">{m.wardrobe_empty()}</p>
				</div>
			{:else}
				<div
					class="grid min-h-0 flex-1 auto-rows-min grid-cols-[repeat(auto-fill,minmax(112px,1fr))] gap-2 overflow-y-auto"
				>
					{#each skins as skin (skin.id)}
						{@const inUse = skin.sha256 === activeSha}
						<button
							class="flex flex-col items-center gap-1.5 rounded-lg border bg-deep px-2 pt-3 pb-2
								{selectedId === skin.id ? 'border-brand' : 'border-line hover:border-ink/30'}"
							onclick={() => (selectedId = skin.id)}
							ondblclick={() => {
								selectedId = skin.id;
								if (!inUse) apply();
							}}
						>
							<SkinPreview src={skin.image} slim={skin.variant === 'slim'} legacy={skin.legacy} height={104} />
							<span class="w-full truncate text-center text-[12px]">{skin.name}</span>
							{#if inUse}
								<Pill accent>{m.wardrobe_in_use()}</Pill>
							{/if}
						</button>
					{/each}
				</div>
			{/if}

			{#if error}
				<p class="shrink-0 leading-[1.6] whitespace-pre-line text-danger" role="alert">{error}</p>
			{/if}
		</section>
	</div>

	{#snippet footer()}
		{#if busy}
			<span class="mr-auto flex items-center gap-2 text-muted"><Spinner />{busy}</span>
		{:else if selected}
			<div class="mr-auto flex items-center gap-2">
				<TextInput
					class="w-40!"
					bind:value={editName}
					aria-label={m.wardrobe_name()}
					onblur={() => update()}
					onkeydown={(e) => e.key === 'Enter' && update()}
				/>
				<div class="flex overflow-hidden rounded-md border border-line">
					{#each [['classic', m.wardrobe_classic()], ['slim', m.wardrobe_slim()]] as [value, label] (value)}
						<button
							class="px-2.5 py-1 text-[12px] {selected.variant === value
								? 'bg-brand-soft text-brand-fg'
								: 'text-muted hover:bg-btn'}"
							aria-pressed={selected.variant === value}
							onclick={() => update(value as SkinVariant)}
						>
							{label}
						</button>
					{/each}
				</div>
				<Button variant="danger" icon="trash" aria-label={m.common_delete()} onclick={remove}></Button>
			</div>
		{:else}
			<span class="mr-auto text-faint">{m.wardrobe_select_hint()}</span>
		{/if}
		<Button
			variant="primary"
			icon="shirt"
			disabled={!!busy || !selected || selected.sha256 === activeSha}
			onclick={apply}
		>
			{m.wardrobe_apply()}
		</Button>
		<Button disabled={!!busy} onclick={onclose}>{m.common_close()}</Button>
	{/snippet}
</Dialog>
