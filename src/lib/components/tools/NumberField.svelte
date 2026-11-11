<script lang="ts">
	import { TextInput } from '$lib/ui';

	/** 有標籤、單位與常用值的數字欄位。 */
	let {
		label,
		value = $bindable(),
		unit = '',
		min,
		max,
		step,
		hint = '',
		presets = [],
		disabled = false
	}: {
		label: string;
		value: number;
		unit?: string;
		min?: number;
		max?: number;
		step?: number;
		hint?: string;
		presets?: { value: number; label: string }[];
		disabled?: boolean;
	} = $props();

	const id = `nf-${Math.random().toString(36).slice(2, 9)}`;
</script>

<div class="flex min-w-0 flex-col gap-1">
	<label for={id} class="text-[12px] font-medium text-muted">{label}</label>
	<div class="relative">
		<TextInput {id} type="number" class="w-full tabular-nums [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none {unit ? 'pr-11' : ''}" {min} {max} {step} {disabled} bind:value />
		{#if unit}
			<span class="pointer-events-none absolute top-1/2 right-2.5 -translate-y-1/2 text-[12px] text-faint">{unit}</span>
		{/if}
	</div>
	{#if presets.length}
		<div class="flex flex-wrap gap-1">
			{#each presets as p (p.value)}
				<button
					type="button"
					class="rounded-sm px-1.5 py-0.5 text-[11.5px] tabular-nums transition-colors duration-150
						{Number(value) === p.value ? 'bg-brand-soft text-brand-fg' : 'bg-btn text-muted hover:bg-btn-hi hover:text-ink'}"
					{disabled}
					onclick={() => (value = p.value)}
				>
					{p.label}
				</button>
			{/each}
		</div>
	{/if}
	{#if hint}
		<span class="text-[11.5px] leading-[1.5] text-faint">{hint}</span>
	{/if}
</div>
