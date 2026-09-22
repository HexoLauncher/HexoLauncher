<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLSelectAttributes } from 'svelte/elements';
	import { FIELD } from './field';

	let {
		value = $bindable(''),
		class: klass = '',
		children,
		...rest
	}: { value?: string | number; class?: string; children: Snippet } & Omit<
		HTMLSelectAttributes,
		'value'
	> = $props();
</script>

<select bind:value {...rest} class="{FIELD} {klass}">
	{@render children()}
</select>
