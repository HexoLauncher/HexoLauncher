<script lang="ts">
	import { Button, Dialog } from '$lib/ui';
	import * as api from '$lib/api';
	import type { BlueprintTerms } from '$lib/api';
	import { renderRichText } from '$lib/richtext';
	import { m } from '$lib/paraglide/messages.js';

	let {
		terms,
		onagree,
		onclose
	}: {
		terms: BlueprintTerms;
		onagree: () => void;
		onclose: () => void;
	} = $props();

	const html = $derived(renderRichText(terms.content, 'markdown'));

	let read = $state(false);

	function externalLinks(node: HTMLElement) {
		function onClick(event: MouseEvent) {
			const href = (event.target as HTMLElement | null)?.closest('a')?.getAttribute('href');
			if (!href) return;
			event.preventDefault();
			if (/^https?:\/\//i.test(href)) api.openExternal(href).catch(() => {});
		}

		function onScroll() {
			if (node.scrollTop + node.clientHeight >= node.scrollHeight - 8) read = true;
		}

		node.addEventListener('click', onClick);
		node.addEventListener('scroll', onScroll);
		requestAnimationFrame(onScroll);
		return {
			destroy: () => {
				node.removeEventListener('click', onClick);
				node.removeEventListener('scroll', onScroll);
			}
		};
	}
</script>

<Dialog title={m.blueprint_terms_title()} width="620px" {onclose}>
	<div class="flex min-h-0 flex-col gap-2.5 px-5 py-4.5">
		<p class="text-muted">{m.blueprint_terms_intro()}</p>
		<div
			class="rich-text max-h-[52vh] min-h-0 overflow-y-auto rounded-lg border border-line bg-deep px-3.5 py-3"
			use:externalLinks
		>
			{@html html}
		</div>
	</div>

	{#snippet footer()}
		<Button variant="primary" disabled={!read} onclick={onagree}>{m.blueprint_terms_agree()}</Button>
		<Button onclick={onclose}>{m.common_cancel()}</Button>
	{/snippet}
</Dialog>
