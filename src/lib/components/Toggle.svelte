<script lang="ts">

	let {
		checked = $bindable(false),
		label,
		disabled = false,
		onchange
	}: {
		checked?: boolean;
		label: string;
		disabled?: boolean;

		onchange?: (checked: boolean) => void;
	} = $props();
</script>

<label
	class="flex items-center gap-2.75 py-1.25"
	class:cursor-pointer={!disabled}
	class:text-faint={disabled}
>
	<input
		type="checkbox"
		bind:checked
		{disabled}
		class="peer sr-only"
		onchange={(e) => onchange?.(e.currentTarget.checked)}
	/>


	<span
		class="relative h-5 w-9 shrink-0 rounded-full bg-btn transition-colors duration-150
			peer-checked:bg-brand
			peer-checked:[&>span]:translate-x-4 peer-checked:[&>span]:bg-brand-ink
			peer-focus-visible:shadow-[0_0_0_3px_var(--color-brand-soft)]
			peer-disabled:opacity-45"
	>
		<span
			class="absolute left-0.75 top-0.75 h-3.5 w-3.5 rounded-full bg-muted
				transition-[transform,background-color] duration-150"
		></span>
	</span>

	<span class="min-w-0">{label}</span>
</label>
