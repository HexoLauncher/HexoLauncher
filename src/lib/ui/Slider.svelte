<script lang="ts">
	import { Slider } from 'melt/builders';

	let {
		ariaLabel,
		value = $bindable(0),
		min = 0,
		max = 100,
		step = 1,
		disabled = false,
		orientation = 'horizontal',
		class: klass = '',
		onchange
	}: {
		ariaLabel?: string;
		value?: number;
		min?: number;
		max?: number;
		step?: number;
		disabled?: boolean;
		orientation?: 'horizontal' | 'vertical';
		class?: string;
		onchange?: (value: number) => void;
	} = $props();

	const slider = new Slider({
		value: () => value,
		min: () => min,
		max: () => max,
		step: () => step,
		orientation: () => orientation,
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
	class="relative flex {orientation === 'vertical'
		? 'h-full min-h-0 w-3.5 justify-center'
		: 'h-3.5 w-full min-w-0 items-center'} {disabled
		? 'pointer-events-none opacity-50'
		: 'cursor-pointer'} {klass}"
	aria-disabled={disabled}
>
	<span class="rounded-full bg-btn {orientation === 'vertical' ? 'h-full w-1.5' : 'h-1.5 w-full'}"></span>
	<span
		{...slider.thumb}
		class="absolute size-3.5 rounded-full {orientation === 'vertical'
			? '-translate-y-1/2'
			: '-translate-x-1/2'} {disabled
			? 'bg-faint'
			: 'bg-brand hover:bg-brand-hi'}"
		style:left={orientation === 'vertical' ? undefined : 'calc(var(--percentage))'}
		style:top={orientation === 'vertical' ? 'calc(var(--percentage))' : undefined}
		style:box-shadow="0 0 0 1px var(--color-backdrop)"
	></span>
</div>
