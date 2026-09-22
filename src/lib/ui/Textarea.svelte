<script lang="ts">
	import type { HTMLTextareaAttributes } from 'svelte/elements';
	import { FIELD } from './field';

	let {
		value = $bindable(''),
		class: klass = '',
		...rest
	}: { value?: string; class?: string } & Omit<HTMLTextareaAttributes, 'value'> = $props();
</script>

<textarea bind:value {...rest} class="{FIELD} resize-none leading-[1.7] {klass}"></textarea>
