<script lang="ts">
	import { Slider } from 'melt/builders';

	let {
		ariaLabel,
		value = $bindable(0),
		min = 0,
		max = 100,
		step = 1,
		disabled = false,
		class: klass = '',
		onchange
	}: {
		ariaLabel?: string;
		value?: number;
		min?: number;
		max?: number;
		step?: number;
		disabled?: boolean;
		class?: string;
		onchange?: (value: number) => void;
	} = $props();

	const slider = new Slider({
		value: () => value,
		min: () => min,
		max: () => max,
		step: () => step,
		onValueChange: (next) => {
			if (disabled) return;
			value = next;
			onchange?.(next);
		}
	});
</script>

<div
	{...slider.root}
	aria-label={ariaLabel}
	class="relative flex h-3.5 w-full min-w-0 items-center {disabled
		? 'pointer-events-none opacity-50'
		: 'cursor-pointer'} {klass}"
	aria-disabled={disabled}
>
	<span class="h-1.5 w-full rounded-full bg-btn"></span>
	<span
		{...slider.thumb}
		class="absolute size-3.5 -translate-x-1/2 rounded-full {disabled
			? 'bg-faint'
			: 'bg-brand hover:bg-brand-hi'}"
		style:left="calc(var(--percentage))"
		style:box-shadow="0 0 0 1px var(--color-backdrop)"
	></span>
</div>
