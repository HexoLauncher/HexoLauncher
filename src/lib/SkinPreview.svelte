<script lang="ts">
	let {
		src,
		slim = false,
		legacy = false,
		height = 128
	}: { src: string; slim?: boolean; legacy?: boolean; height?: number } = $props();

	type Part = { sx: number; sy: number; w: number; h: number; x: number; y: number; mirror?: boolean };

	const arm = $derived(slim ? 3 : 4);
	const textureHeight = $derived(legacy ? 32 : 64);

	const base = $derived<Part[]>([
		{ sx: 8, sy: 8, w: 8, h: 8, x: 4, y: 0 },
		{ sx: 20, sy: 20, w: 8, h: 12, x: 4, y: 8 },
		{ sx: 44, sy: 20, w: arm, h: 12, x: 4 - arm, y: 8 },
		legacy
			? { sx: 44, sy: 20, w: arm, h: 12, x: 12, y: 8, mirror: true }
			: { sx: 36, sy: 52, w: arm, h: 12, x: 12, y: 8 },
		{ sx: 4, sy: 20, w: 4, h: 12, x: 4, y: 20 },
		legacy
			? { sx: 4, sy: 20, w: 4, h: 12, x: 8, y: 20, mirror: true }
			: { sx: 20, sy: 52, w: 4, h: 12, x: 8, y: 20 }
	]);

	const overlay = $derived<Part[]>(
		legacy
			? [{ sx: 40, sy: 8, w: 8, h: 8, x: 4, y: 0 }]
			: [
					{ sx: 40, sy: 8, w: 8, h: 8, x: 4, y: 0 },
					{ sx: 20, sy: 36, w: 8, h: 12, x: 4, y: 8 },
					{ sx: 44, sy: 36, w: arm, h: 12, x: 4 - arm, y: 8 },
					{ sx: 52, sy: 52, w: arm, h: 12, x: 12, y: 8 },
					{ sx: 4, sy: 36, w: 4, h: 12, x: 4, y: 20 },
					{ sx: 4, sy: 52, w: 4, h: 12, x: 8, y: 20 }
				]
	);
</script>

{#snippet part(p: Part)}
	<g transform={p.mirror ? `translate(${p.x * 2 + p.w} 0) scale(-1 1)` : undefined}>
		<svg x={p.x} y={p.y} width={p.w} height={p.h} viewBox="{p.sx} {p.sy} {p.w} {p.h}">
			<image href={src} width="64" height={textureHeight} />
		</svg>
	</g>
{/snippet}

<svg
	width={height / 2}
	{height}
	viewBox="0 0 16 32"
	aria-hidden="true"
	class="block shrink-0 [image-rendering:pixelated]"
>
	{#each base as p, i (i)}{@render part(p)}{/each}
	{#each overlay as p, i (i)}{@render part(p)}{/each}
</svg>
