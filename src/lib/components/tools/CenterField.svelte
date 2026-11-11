<script lang="ts">
	import { TextInput } from '$lib/ui';
	import { tools } from '$lib/tools/state.svelte';
	import { m } from '$lib/paraglide/messages.js';

	/** 搜尋中心：X / Z 與快速填入（地圖中心、玩家、原點）。 */
	let { x = $bindable(), z = $bindable() }: { x: number; z: number } = $props();

	const player = $derived(
		tools.mode === 'world'
			? (tools.world?.players.find((p) => p.dimension === 'minecraft:overworld') ?? tools.world?.players[0] ?? null)
			: null
	);

	function set(nx: number, nz: number) {
		x = Math.round(nx);
		z = Math.round(nz);
	}

	const chip =
		'rounded-sm bg-btn px-1.5 py-0.5 text-[11.5px] text-muted transition-colors duration-150 hover:bg-btn-hi hover:text-ink';
</script>

<div class="flex min-w-0 flex-col gap-1">
	<span class="text-[12px] font-medium text-muted">{m.tools_center()}</span>
	<div class="grid grid-cols-2 gap-1.5">
		<div class="relative">
			<span class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-[12px] text-faint">X</span>
			<TextInput type="number" class="w-full pl-6 tabular-nums [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none" bind:value={x} aria-label={m.tools_center_x()} />
		</div>
		<div class="relative">
			<span class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-[12px] text-faint">Z</span>
			<TextInput type="number" class="w-full pl-6 tabular-nums [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none" bind:value={z} aria-label={m.tools_center_z()} />
		</div>
	</div>
	<div class="flex flex-wrap gap-1">
		<button type="button" class={chip} onclick={() => set(tools.seedView.x, tools.seedView.z)}>{m.tools_map_center()}</button>
		{#if player}
			<button type="button" class={chip} onclick={() => set(player.x, player.z)}>{m.tools_player()}</button>
		{/if}
		<button type="button" class={chip} onclick={() => set(0, 0)}>{m.tools_origin_short()}</button>
	</div>
</div>
