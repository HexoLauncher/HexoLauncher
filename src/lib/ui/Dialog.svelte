<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Dialog } from 'melt/builders';
	import Icon from './Icon.svelte';
	import { m } from '$lib/paraglide/messages.js';

	let {
		title,
		width = '560px',
		// 模態 dialog 上下 inset 都是 0，高度用 auto 會被撐滿整個視窗；fit-content 才會貼合內容
		// （仍受 max-h 限制）。需要固定大小的對話框請明確傳入高度。
		height = 'fit-content',
		onclose,
		titleIcon,
		children,
		footer
	}: {
		title: string;
		width?: string;
		height?: string;
		onclose: () => void;
		titleIcon?: Snippet;
		children: Snippet;
		footer?: Snippet;
	} = $props();

	const dialog = new Dialog({
		onOpenChange: (open) => {
			if (!open) onclose();
		}
	});

	$effect(() => {
		dialog.open = true;
	});
</script>

<div
	{...dialog.overlay}
	class="fixed inset-0 m-0 h-full w-full animate-fade bg-scrim backdrop-blur-[3px]"
></div>

<dialog
	{...dialog.content}
	class="m-auto max-h-[calc(100dvh-3rem)] max-w-[calc(100vw-3rem)] animate-rise flex-col
		overflow-hidden rounded-xl border border-line bg-window p-0 text-ink shadow-lg
		open:flex backdrop:bg-transparent"
	style:width
	style:height
	aria-label={title}
>
	<header class="flex h-10.5 shrink-0 items-center gap-2.25 border-b border-line pl-4">
		{@render titleIcon?.()}
		<h2 class="text-[13px] font-semibold">{title}</h2>

		<div class="ml-auto flex h-full">
			<button
				class="grid h-full w-10.5 place-items-center text-muted transition-colors duration-150 disabled:text-faint"
				aria-label={m.common_minimize()}
				disabled
			>
				<Icon name="win-min" size={13} />
			</button>
			<button
				class="grid h-full w-10.5 place-items-center text-muted transition-colors duration-150 disabled:text-faint"
				aria-label={m.common_maximize()}
				disabled
			>
				<Icon name="win-max" size={11} />
			</button>
			<button
				class="grid h-full w-10.5 place-items-center text-muted transition-colors duration-150
					hover:bg-danger hover:text-white"
				aria-label={m.common_close()}
				onclick={() => (dialog.open = false)}
			>
				<Icon name="close" size={13} />
			</button>
		</div>
	</header>

	<!-- flex-auto（基準為內容高度）而不是 flex-1：WebKit 在 fit-content 高度下會把 0% 基準算成 0，內容區會被壓扁。 -->
	<div class="flex min-h-0 flex-auto flex-col overflow-hidden">
		{@render children()}
	</div>

	{#if footer}
		<footer class="flex shrink-0 justify-end gap-2 border-t border-line px-4 py-3">
			{@render footer()}
		</footer>
	{/if}
</dialog>
