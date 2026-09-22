<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';
	import { FIELD } from './field';

	let {
		value = $bindable(''),
		class: klass = '',
		...rest
	}: { value?: string | number; class?: string } & Omit<HTMLInputAttributes, 'value'> = $props();
</script>

<input bind:value {...rest} class="{FIELD} {klass}" />
