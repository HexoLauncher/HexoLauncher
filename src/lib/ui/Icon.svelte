<script lang="ts">
	import { ICONS } from './icons';

	let {
		name,
		size = 16,
		class: klass = ''
	}: {
		name: string;
		size?: number;
		class?: string;
	} = $props();

	const Glyph = $derived(ICONS[name] ?? ICONS.box);
</script>

<Glyph width={size} height={size} class="shrink-0 {klass}" aria-hidden="true" />
