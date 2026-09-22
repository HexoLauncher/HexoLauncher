<script lang="ts">
	let {
		value,
		id,
		onpick
	}: {
		value: string;
		id?: string;
		onpick: (hex: string) => void;
	} = $props();
</script>

<div class="flex items-center gap-2">
	<input
		{id}
		type="color"
		{value}
		class="size-8 shrink-0 cursor-pointer rounded-md border border-line bg-transparent p-0.75"
		oninput={(e) => onpick(e.currentTarget.value)}
	/>
	<input
		{value}
		maxlength={7}
		spellcheck="false"
		class="w-28 min-h-[30px] rounded-md border border-line bg-deep px-[9px] py-[5px] font-mono uppercase
			transition-[border-color,box-shadow] duration-150 outline-none
			hover:border-line-hi focus:border-brand-line focus:shadow-[0_0_0_3px_var(--color-brand-soft)]"
		oninput={(e) => onpick(e.currentTarget.value)}
	/>
</div>

<style>
	input[type='color']::-webkit-color-swatch-wrapper {
		padding: 0;
	}
	input[type='color']::-webkit-color-swatch {
		border: 0;
		border-radius: var(--radius-sm);
	}
</style>
