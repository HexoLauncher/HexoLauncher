<script lang="ts">
	import { onMount } from 'svelte';
	import { Button, Spinner, TextInput } from '$lib/ui';
	import * as api from '$lib/api';
	import type { BlueprintMaterials } from '$lib/api';
	import { report } from '$lib/state.svelte';
	import { m } from '$lib/paraglide/messages.js';

	let { id }: { id: string } = $props();

	let data = $state<BlueprintMaterials | null>(null);
	let message = $state('');
	let filter = $state('');

	onMount(async () => {
		try {
			data = await api.blueprintMaterials(id);
		} catch (e) {
			message = api.errorMessage(e);
		}
	});

	/** `minecraft:polished_andesite` → `polished andesite` */
	function itemName(id: string): string {
		const [ns, name] = id.includes(':') ? id.split(':', 2) : ['minecraft', id];
		const pretty = name.replaceAll('_', ' ');
		return ns === 'minecraft' ? pretty : `${ns}:${pretty}`;
	}

	/** 一組 64 個、一盒 27 組。 */
	function stacks(count: number): string {
		const box = 64 * 27;
		const parts: string[] = [];
		const boxes = Math.floor(count / box);
		const full = Math.floor((count % box) / 64);
		const rest = count % 64;
		if (boxes) parts.push(m.blueprint_stacks_box({ count: boxes }));
		if (full) parts.push(m.blueprint_stacks_stack({ count: full }));
		if (rest || !parts.length) parts.push(m.blueprint_stacks_item({ count: rest }));
		return parts.join(' + ');
	}

	const shown = $derived.by(() => {
		const needle = filter.trim().toLowerCase().replaceAll(' ', '_');
		const items = data?.items ?? [];
		return needle ? items.filter((i) => i.item.toLowerCase().includes(needle)) : items;
	});

	async function copy() {
		if (!data) return;
		const text = data.items.map((i) => `${itemName(i.item)}\t${i.count}`).join('\n');
		try {
			await navigator.clipboard.writeText(text);
			report(m.blueprint_materials_copied());
		} catch {
			// 剪貼簿不能用時就算了
		}
	}
</script>

<div class="flex flex-col gap-2 rounded-lg border border-line bg-window p-2.5">
	{#if message}
		<p class="py-3 text-center text-muted">{message}</p>
	{:else if !data}
		<p class="flex items-center justify-center gap-2 py-3 text-muted"><Spinner />{m.blueprint_materials_loading()}</p>
	{:else if !data.items.length}
		<p class="py-3 text-center text-muted">{m.blueprint_materials_empty()}</p>
	{:else}
		<div class="flex items-center gap-2">
			<span class="min-w-0 flex-1 truncate text-muted">
				{m.blueprint_materials_summary({ kinds: data.items.length.toLocaleString(), total: data.total.toLocaleString() })}
				<span class="text-faint">· {stacks(data.total)}</span>
			</span>
			<Button icon="copy" onclick={copy}>{m.blueprint_materials_copy()}</Button>
		</div>
		<TextInput bind:value={filter} placeholder={m.blueprint_materials_search()} aria-label={m.blueprint_materials_search()} />
		<div class="max-h-72 overflow-y-auto rounded-md border border-line">
			<table class="w-full">
				<tbody>
					{#each shown as entry (entry.item)}
						<tr class="border-b border-line last:border-0">
							<td class="px-2 py-1">
								<span class="text-ink capitalize">{itemName(entry.item)}</span>
								<span class="block font-mono text-[10.5px] text-faint">{entry.item}</span>
							</td>
							<td class="px-2 py-1 text-right font-medium tabular-nums">{entry.count.toLocaleString()}</td>
							<td class="px-2 py-1 text-right text-[11.5px] whitespace-nowrap text-faint">{stacks(entry.count)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>
