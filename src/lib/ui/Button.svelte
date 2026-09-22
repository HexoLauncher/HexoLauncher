<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';
	import Icon from './Icon.svelte';

	type Variant = 'default' | 'primary' | 'destructive' | 'danger';

	let {
		variant = 'default',
		icon,
		iconSize = 15,
		class: klass = '',
		children,
		...rest
	}: {
		variant?: Variant;
		icon?: string;
		iconSize?: number;
		class?: string;
		children?: Snippet;
	} & HTMLButtonAttributes = $props();

	const BASE =
		'inline-flex min-h-[30px] items-center justify-center gap-[7px] rounded-md border border-transparent bg-btn px-3.5 py-[5px] font-medium whitespace-nowrap transition-[background-color,color,box-shadow] duration-150 not-disabled:hover:bg-btn-hi not-disabled:active:bg-btn-lo disabled:bg-btn-off disabled:text-faint';

	const VARIANTS: Record<Variant, string> = {
		default: '',
		primary:
			'bg-brand font-semibold text-brand-ink not-disabled:hover:bg-brand-hi not-disabled:hover:shadow-[0_0_0_4px_var(--color-brand-soft)] not-disabled:active:bg-brand-lo disabled:bg-btn-off disabled:text-faint disabled:shadow-none',
		destructive:
			'bg-danger font-semibold text-danger-ink not-disabled:hover:bg-danger-hi not-disabled:hover:shadow-[0_0_0_4px_var(--color-danger-soft)]',
		danger: 'not-disabled:hover:bg-danger not-disabled:hover:text-danger-ink'
	};
</script>

<button type="button" {...rest} class="{BASE} {VARIANTS[variant]} {klass}">
	{#if icon}
		<Icon name={icon} size={iconSize} />
	{/if}
	{@render children?.()}
</button>
