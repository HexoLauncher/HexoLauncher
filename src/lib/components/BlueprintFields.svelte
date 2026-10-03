<script lang="ts">
	import { Field, TextInput, Textarea } from '$lib/ui';
	import { hub, instanceVersions, isRelease } from '$lib/blueprints.svelte';
	import { app } from '$lib/state.svelte';
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

	const versions = $derived(instanceVersions(app.instances));

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
		<TextInput
			id="bp-min"
			class="w-28"
			bind:value={mcVersionMin}
			list="bp-versions"
			placeholder={minPlaceholder || m.blueprint_version_min()}
			aria-label={m.blueprint_version_min()}
		/>
		<span class="text-faint">–</span>
		<TextInput
			class="w-28"
			bind:value={mcVersionMax}
			list="bp-versions"
			placeholder={m.blueprint_version_no_max()}
			aria-label={m.blueprint_version_max()}
		/>
		<datalist id="bp-versions">
			{#each versions as v (v)}
				<option value={v}></option>
			{/each}
		</datalist>
	</div>
</Field>
