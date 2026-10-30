<script lang="ts">
	import { Field, Select, TextInput, Textarea } from '$lib/ui';
	import { hub, instanceVersions, isRelease } from '$lib/blueprints.svelte';
	import { app } from '$lib/state.svelte';
	import { releaseVersions, withCurrent } from '$lib/versions';
	import { m } from '$lib/paraglide/messages.js';

	const MAX_TAGS = 5;

	let {
		name = $bindable(''),
		description = $bindable(''),
		tags = $bindable<string[]>([]),
		mcVersionMin = $bindable(''),
		mcVersionMax = $bindable(''),
		namePlaceholder = '',
		minPlaceholder = ''
	}: {
		name?: string;
		description?: string;
		tags?: string[];
		mcVersionMin?: string;
		mcVersionMax?: string;
		namePlaceholder?: string;
		minPlaceholder?: string;
	} = $props();

	let releases = $state<string[]>([]);
	releaseVersions().then((list) => (releases = list)).catch(() => {});

	const versions = $derived(
		withCurrent(releases.length ? releases : instanceVersions(app.instances), mcVersionMin, mcVersionMax)
	);

	function toggle(slug: string) {
		if (tags.includes(slug)) tags = tags.filter((t) => t !== slug);
		else if (tags.length < MAX_TAGS) tags = [...tags, slug];
	}

	/** 欄位內容不是正式版格式時顯示的錯誤。 */
	export function versionError(): string {
		for (const v of [mcVersionMin, mcVersionMax]) {
			if (v.trim() && !isRelease(v.trim())) return m.blueprint_version_format({ version: v.trim() });
		}
		return '';
	}
</script>

<Field label={m.common_name()} for="bp-name">
	<TextInput id="bp-name" bind:value={name} maxlength={100} placeholder={namePlaceholder} />
</Field>

<Field label={m.blueprint_description()} for="bp-desc" class="items-start!">
	<Textarea id="bp-desc" bind:value={description} maxlength={5000} rows={5} />
</Field>

<Field label={m.blueprint_tags()}>
	<div class="flex flex-wrap gap-1.5">
		{#each hub.tags as tag (tag.slug)}
			{@const on = tags.includes(tag.slug)}
			<button
				type="button"
				class="rounded-full px-2.5 py-0.75 text-[11.5px] font-medium transition-colors duration-150
					disabled:opacity-40
					{on ? 'bg-brand-soft text-brand-fg' : 'bg-btn text-muted not-disabled:hover:bg-btn-hi not-disabled:hover:text-ink'}"
				aria-pressed={on}
				disabled={!on && tags.length >= MAX_TAGS}
				onclick={() => toggle(tag.slug)}
			>
				{tag.name}
			</button>
		{:else}
			<span class="text-faint">{m.blueprint_no_tags()}</span>
		{/each}
	</div>
</Field>

<Field label={m.blueprint_versions()} for="bp-min">
	<div class="flex items-center gap-2">
		<Select id="bp-min" class="w-32" bind:value={mcVersionMin} aria-label={m.blueprint_version_min()}>
			<option value="">{minPlaceholder || m.blueprint_version_min()}</option>
			{#each versions as v (v)}
				<option value={v}>{v}</option>
			{/each}
		</Select>
		<span class="text-faint">–</span>
		<Select class="w-32" bind:value={mcVersionMax} aria-label={m.blueprint_version_max()}>
			<option value="">{m.blueprint_version_no_max()}</option>
			{#each versions as v (v)}
				<option value={v}>{v}</option>
			{/each}
		</Select>
	</div>
</Field>
