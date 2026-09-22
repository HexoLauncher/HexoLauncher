<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLInputAttributes } from 'svelte/elements';
	import Icon from './Icon.svelte';

	let {
		value = $bindable(''),
		class: klass = '',
		children,
		...rest
	}: { value?: string; class?: string; children?: Snippet } & Omit<
		HTMLInputAttributes,
		'value'
	> = $props();
</script>

<label
	class="flex items-center gap-[9px] rounded-full border border-transparent px-3 py-1.5 text-muted
		transition-[border-color,box-shadow] duration-150
		focus-within:border-brand-line focus-within:shadow-[0_0_0_3px_var(--color-brand-soft)] {klass}"
>
	<Icon name="search" size={14} />
	<input
		bind:value
		{...rest}
		class="min-w-0 flex-1 border-0 bg-transparent text-ink outline-none placeholder:text-faint"
	/>
	{@render children?.()}
</label>
