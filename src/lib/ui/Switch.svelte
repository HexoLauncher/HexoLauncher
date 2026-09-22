<script lang="ts">
	import { Toggle } from 'melt/builders';

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

	const toggle = new Toggle({
		value: () => checked,
		disabled: () => disabled,
		onValueChange: (next) => {
			checked = next;
			onchange?.(next);
		}
	});
</script>

<label
	class="flex items-center gap-2.75 py-1.25"
	class:cursor-pointer={!disabled}
	class:text-faint={disabled}
>
	<button
		{...toggle.trigger}
		class="relative h-5 w-9 shrink-0 rounded-full bg-btn transition-colors duration-150
			data-[checked]:bg-brand
			focus-visible:shadow-[0_0_0_3px_var(--color-brand-soft)]
			disabled:opacity-45"
	>
		<span
			class="absolute top-0.75 left-0.75 h-3.5 w-3.5 rounded-full bg-muted
				transition-[transform,background-color] duration-150"
			class:translate-x-4={checked}
			class:bg-brand-ink={checked}
		></span>
	</button>
	<span class="min-w-0">{label}</span>
</label>
