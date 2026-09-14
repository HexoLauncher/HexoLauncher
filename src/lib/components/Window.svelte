<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from '$lib/Icon.svelte';


	let {
		title,
		width = '560px',
		height = 'auto',
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
</script>

<svelte:window
	onkeydown={(e) => {
		if (e.key === 'Escape') onclose();
	}}
/>

<div
	class="fixed inset-0 z-100 flex animate-fade items-center justify-center bg-[#07090b99] p-6 backdrop-blur-[3px]"
>
	<div
		class="flex max-h-full max-w-full animate-rise flex-col overflow-hidden rounded-xl
			border border-line bg-window shadow-lg"
		style:width
		style:height
		role="dialog"
		aria-modal="true"
		aria-label={title}
	>
		<header class="flex h-10.5 shrink-0 items-center gap-2.25 border-b border-line bg-window pl-4">
			{#if titleIcon}
				{@render titleIcon()}
			{/if}
			<h2 class="text-[13px] font-semibold">{title}</h2>

			<div class="ml-auto flex h-full">
				<button
					class="grid h-full w-10.5 place-items-center text-muted transition-colors duration-150 disabled:text-faint"
					aria-label="最小化"
					disabled
				>
					<Icon name="win-min" size={13} />
				</button>
				<button
					class="grid h-full w-10.5 place-items-center text-muted transition-colors duration-150 disabled:text-faint"
					aria-label="最大化"
					disabled
				>
					<Icon name="win-max" size={11} />
				</button>
				<button
					class="grid h-full w-10.5 place-items-center text-muted transition-colors duration-150
						hover:bg-danger hover:text-white"
					aria-label="關閉"
					onclick={onclose}
				>
					<Icon name="close" size={13} />
				</button>
			</div>
		</header>

		<div class="flex min-h-0 flex-1 flex-col overflow-hidden">
			{@render children()}
		</div>

		{#if footer}
			<footer class="flex shrink-0 justify-end gap-2 border-t border-line bg-window px-4 py-3">
				{@render footer()}
			</footer>
		{/if}
	</div>
</div>
