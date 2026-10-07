<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		class: klass = '',
		children
	}: {
		class?: string;
		children: Snippet;
	} = $props();
</script>

<table class="nolauncher-table w-full border-separate border-spacing-0 text-left {klass}">
	{@render children()}
</table>

<style>
	.nolauncher-table :global(th) {
		position: sticky;
		top: 0;
		z-index: 2;
		border-bottom: 1px solid var(--color-line);
		background: var(--color-backdrop);
		padding: 7px 10px;
		font-weight: 500;
		white-space: nowrap;
		color: var(--color-muted);
	}

	.nolauncher-table :global(td) {
		overflow: hidden;
		padding: 7px 10px;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.nolauncher-table :global(tbody tr) {
		transition: background-color 150ms;
	}

	.nolauncher-table :global(tbody tr:hover td) {
		background: var(--color-hover);
	}

	.nolauncher-table :global(tbody td:first-child) {
		border-start-start-radius: var(--radius-md);
		border-end-start-radius: var(--radius-md);
	}

	.nolauncher-table :global(tbody td:last-child) {
		border-start-end-radius: var(--radius-md);
		border-end-end-radius: var(--radius-md);
	}

	.nolauncher-table :global(tbody tr.selected td) {
		background: var(--color-brand-soft);
		color: var(--color-ink);
	}

	.nolauncher-table :global(tbody tr.selected td:first-child) {
		box-shadow: inset 2px 0 0 var(--color-brand);
	}

	.nolauncher-table :global(.dim) {
		color: var(--color-muted);
	}
</style>
