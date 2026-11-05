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
		--table-bleed: 0.25rem;
		position: sticky;
		top: 0;
		z-index: 2;
		box-shadow: inset 0 -1px 0 var(--color-line);
		background: var(--color-backdrop);
		padding: 7px 10px 8px;
		font-weight: 500;
		white-space: nowrap;
		color: var(--color-muted);
	}

	.nolauncher-table :global(th:first-child) {
		box-shadow:
			inset 0 -1px 0 var(--color-line),
			calc(var(--table-bleed) * -1) -1px 0 var(--color-backdrop),
			calc(var(--table-bleed) * -1) 0 0 var(--color-line);
	}

	.nolauncher-table :global(th:last-child) {
		box-shadow:
			inset 0 -1px 0 var(--color-line),
			var(--table-bleed) -1px 0 var(--color-backdrop),
			var(--table-bleed) 0 0 var(--color-line);
	}

	.nolauncher-table :global(th:only-child) {
		box-shadow:
			inset 0 -1px 0 var(--color-line),
			calc(var(--table-bleed) * -1) -1px 0 var(--color-backdrop),
			calc(var(--table-bleed) * -1) 0 0 var(--color-line),
			var(--table-bleed) -1px 0 var(--color-backdrop),
			var(--table-bleed) 0 0 var(--color-line);
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
