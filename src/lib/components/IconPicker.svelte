<script lang="ts">
	import { Button, Menu, MenuItem, MenuSeparator } from '$lib/ui';
	import InstanceIcon from '$lib/InstanceIcon.svelte';
	import { ICON_CHOICES } from '$lib/icons';
	import { m } from '$lib/paraglide/messages.js';

	let {
		icon,
		image = null,
		onselect,
		onpickimage
	}: {
		icon: string;
		image?: string | null;
		onselect: (icon: string) => void;
		onpickimage: () => void;
	} = $props();

	const choices = $derived(image ? ['custom', ...ICON_CHOICES] : ICON_CHOICES);
</script>

<Menu width="228px">
	{#snippet trigger(attrs)}
		<Button {...attrs} class="size-14.5 rounded-lg p-0" aria-label={m.common_select_icon()}>
			<InstanceIcon name={icon} {image} size={34} />
		</Button>
	{/snippet}
	<div class="grid grid-cols-5 gap-0.75 p-0.75">
		{#each choices as choice (choice)}
			<button
				class="grid place-items-center rounded-sm border border-transparent p-1.25 hover:bg-hover {icon ===
				choice
					? 'border-brand bg-brand-soft'
					: ''}"
				aria-label={choice === 'custom' ? m.icon_use_image() : choice}
				onclick={() => onselect(choice)}
			>
				<InstanceIcon name={choice} {image} size={30} />
			</button>
		{/each}
	</div>
	<MenuSeparator />
	<MenuItem icon="image" onclick={onpickimage}>{m.icon_custom_image()}</MenuItem>
</Menu>
