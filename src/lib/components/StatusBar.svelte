<script lang="ts">
	import Icon from '$lib/ui/Icon.svelte';

	let {
		message,
		baseDir,
		instanceCount,
		account,
		onOpenBaseDir
	}: {
		message: string;

		baseDir: string;
		instanceCount: number;
		account: string;
		onOpenBaseDir: () => void;
	} = $props();
</script>

<footer
	class="flex h-8.5 shrink-0 items-center gap-1 border-t border-line bg-window px-3 text-[12px] text-muted"
>
	<button
		class="flex min-w-0 items-center gap-1.75 rounded-full bg-brand-soft px-2.75 py-1
			font-medium text-brand-fg transition-colors duration-150 hover:bg-brand-soft-hi"
		title={baseDir}
		onclick={onOpenBaseDir}
	>
		<Icon name="folder" size={13} />
		<span class="overflow-hidden text-ellipsis whitespace-nowrap">{baseDir || '尚未設定資料目錄'}</span>
	</button>

	<span class="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap pl-3 text-right">
		{message}
	</span>

	<span class="mx-1.25 h-4 w-px self-center bg-line"></span>
	<span class="whitespace-nowrap px-1.5">{instanceCount} 個實例</span>
	<span class="mx-1.25 h-4 w-px self-center bg-line"></span>
	<span class="whitespace-nowrap px-1.5">{account}</span>
</footer>
