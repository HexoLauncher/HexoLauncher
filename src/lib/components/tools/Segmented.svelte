<script lang="ts" generics="T extends string">
	import { ToolButton } from '$lib/ui';

	/** 分段按鈕：一排互斥的選項。 */
	let {
		options,
		value = $bindable(),
		label,
		disabled = false,
		stretch = false,
		onchange
	}: {
		options: { value: T; label: string; title?: string }[];
		value: T;
		label: string;
		disabled?: boolean;
		/** 撐滿整列，選項平均分配寬度。 */
		stretch?: boolean;
		onchange?: (value: T) => void;
	} = $props();
</script>

<div class="flex gap-0.5 rounded-md bg-btn p-0.5 {stretch ? 'w-full' : 'w-fit'}" role="group" aria-label={label}>
	{#each options as option (option.value)}
		<ToolButton
			layout="bare"
			class="justify-center px-2.5 py-1 whitespace-nowrap {stretch ? 'min-w-0 flex-1' : ''}"
			active={value === option.value}
			aria-pressed={value === option.value}
			title={option.title}
			{disabled}
			onclick={() => {
				value = option.value;
				onchange?.(option.value);
			}}
		>
			{option.label}
		</ToolButton>
	{/each}
</div>
