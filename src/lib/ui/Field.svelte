<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		label,
		for: htmlFor,
		labelWidth = '132px',
		class: klass = '',
		children
	}: {
		label: string;
		for?: string;
		labelWidth?: string;
		class?: string;
		children: Snippet;
	} = $props();
</script>

<div
	class="mb-2 grid items-center gap-2.5 {klass}"
	style:grid-template-columns="var(--field-label, {labelWidth}) 1fr"
>
	<label class="text-right text-muted" for={htmlFor}>{label}</label>
	{@render children()}
</div>
