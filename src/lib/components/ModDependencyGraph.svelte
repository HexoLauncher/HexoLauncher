<script lang="ts">
	import { Button } from '$lib/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { NODE_HEIGHT, NODE_WIDTH, type GraphLayout, type GraphLink } from './modDependencyGraph';

	let { layout, selected = $bindable(null) }: { layout: GraphLayout; selected?: string | null } =
		$props();

	const PAD = 16;
	const MIN_SCALE = 0.15;
	const MAX_SCALE = 2.5;
	const LABEL_UNITS = 25;
	const uid = $props.id();

	let svg = $state<SVGSVGElement>();
	let width = $state(0);
	let height = $state(0);
	let view = $state({ x: 0, y: 0, scale: 1 });
	let drag: { pointer: number; x: number; y: number } | null = null;
	let dragged = false;
	let panning = $state(false);

	const nodeById = $derived(new Map(layout.nodes.map((node) => [node.id, node])));
	const related = $derived(
		selected
			? [
					selected,
					...layout.links.flatMap((link) =>
						link.from === selected ? [link.to] : link.to === selected ? [link.from] : []
					)
				]
			: null
	);

	const clamp = (value: number) => Math.min(MAX_SCALE, Math.max(MIN_SCALE, value));

	function fit() {
		const scale = Math.max(
			0.3,
			Math.min(1, (width - PAD * 2) / layout.width, (height - PAD * 2) / layout.height)
		);
		view = {
			scale,
			x: Math.max(PAD, (width - layout.width * scale) / 2),
			y: Math.max(PAD, (height - layout.height * scale) / 2)
		};
	}

	function zoomAt(factor: number, px = width / 2, py = height / 2) {
		const scale = clamp(view.scale * factor);
		const k = scale / view.scale;
		view = { scale, x: px - (px - view.x) * k, y: py - (py - view.y) * k };
	}

	$effect(() => {
		if (width && height) fit();
	});

	$effect(() => {
		if (selected && !nodeById.has(selected)) selected = null;
	});

	$effect(() => {
		const element = svg;
		if (!element) return;
		const onwheel = (event: WheelEvent) => {
			event.preventDefault();
			const rect = element.getBoundingClientRect();
			zoomAt(Math.exp(-event.deltaY * 0.0015), event.clientX - rect.left, event.clientY - rect.top);
		};
		element.addEventListener('wheel', onwheel, { passive: false });
		return () => element.removeEventListener('wheel', onwheel);
	});

	function onpointerdown(event: PointerEvent) {
		if (event.button !== 0) return;
		drag = { pointer: event.pointerId, x: event.clientX, y: event.clientY };
		dragged = false;
	}

	function onpointermove(event: PointerEvent) {
		if (drag?.pointer !== event.pointerId) return;
		const dx = event.clientX - drag.x;
		const dy = event.clientY - drag.y;
		if (!dragged) {
			if (Math.hypot(dx, dy) < 4) return;
			dragged = panning = true;
			svg?.setPointerCapture(event.pointerId);
		}
		drag.x = event.clientX;
		drag.y = event.clientY;
		view = { ...view, x: view.x + dx, y: view.y + dy };
	}

	function onpointerup(event: PointerEvent) {
		if (drag?.pointer !== event.pointerId) return;
		drag = null;
		panning = false;
	}

	function toggle(id: string) {
		if (!dragged) selected = selected === id ? null : id;
	}

	function truncate(label: string) {
		let units = 0;
		for (let i = 0; i < label.length; i++) {
			units += label.charCodeAt(i) > 0x2e80 ? 2 : 1;
			if (units > LABEL_UNITS) return `${label.slice(0, i)}…`;
		}
		return label;
	}

	function linkPath(link: GraphLink) {
		const from = nodeById.get(link.from)!;
		const to = nodeById.get(link.to)!;
		const x1 = from.x + NODE_WIDTH;
		const y1 = from.y + NODE_HEIGHT / 2;
		const x2 = to.x;
		const y2 = to.y + NODE_HEIGHT / 2;
		const dx = Math.max(48, Math.abs(x2 - x1) / 2);
		return `M${x1} ${y1} C${x1 + dx} ${y1} ${x2 - dx} ${y2} ${x2} ${y2}`;
	}

	function linkTone(link: GraphLink) {
		if (link.attention) return 'danger';
		if (link.kind === 'incompatible') return 'warn';
		return 'faint';
	}

	const TONE_STROKE = { danger: 'stroke-danger', warn: 'stroke-warn', faint: 'stroke-faint' };
	const TONE_FILL = { danger: 'fill-danger', warn: 'fill-warn', faint: 'fill-faint' };
	const dash = (link: GraphLink) =>
		link.cyclic ? '1 5' : link.kind === 'optional' ? '6 4' : undefined;
</script>

<div
	class="relative h-[46vh] overflow-hidden rounded-md border border-line bg-deep"
	bind:clientWidth={width}
	bind:clientHeight={height}
>
	<svg
		bind:this={svg}
		class="size-full touch-none select-none {panning ? 'cursor-grabbing' : 'cursor-grab'}"
		role="presentation"
		{onpointerdown}
		{onpointermove}
		{onpointerup}
		onpointercancel={onpointerup}
		onclick={(event) => {
			if (event.target === svg && !dragged) selected = null;
		}}
	>
		<defs>
			{#each Object.entries(TONE_FILL) as [tone, fill] (tone)}
				<marker
					id="{uid}-{tone}"
					viewBox="0 0 10 10"
					refX="9"
					refY="5"
					markerWidth="7"
					markerHeight="7"
					orient="auto-start-reverse"
				>
					<path d="M0 0 L10 5 L0 10 z" class={fill} />
				</marker>
			{/each}
		</defs>
		<g transform="translate({view.x} {view.y}) scale({view.scale})">
			{#each layout.links as link (`${link.from}\n${link.to}\n${link.kind}`)}
				{@const tone = linkTone(link)}
				{@const active = related && (link.from === selected || link.to === selected)}
				<path
					d={linkPath(link)}
					fill="none"
					class="{TONE_STROKE[tone]} transition-opacity"
					stroke-width={active ? 2 : 1.25}
					stroke-dasharray={dash(link)}
					stroke-linecap="round"
					opacity={related && !active ? 0.12 : 1}
					marker-end="url(#{uid}-{tone})"
				/>
			{/each}
			{#each layout.nodes as node (node.id)}
				<g
					transform="translate({node.x} {node.y})"
					class="cursor-pointer transition-opacity outline-none"
					opacity={related && !related.includes(node.id)
						? 0.25
						: node.state === 'disabled'
							? 0.55
							: 1}
					role="button"
					tabindex="0"
					aria-label={node.label}
					aria-pressed={selected === node.id}
					onclick={() => toggle(node.id)}
					onkeydown={(event) => {
						if (event.key === 'Enter' || event.key === ' ') {
							event.preventDefault();
							dragged = false;
							toggle(node.id);
						}
					}}
				>
					<title>{node.state === 'missing' ? node.label : `${node.label}\n${node.id}`}</title>
					<rect
						width={NODE_WIDTH}
						height={NODE_HEIGHT}
						rx="6"
						class="{selected === node.id ? 'fill-brand-soft' : 'fill-card'}
							{selected === node.id ? 'stroke-brand' : node.attention ? 'stroke-danger' : 'stroke-line-hi'}"
						stroke-width={selected === node.id ? 2 : 1}
						stroke-dasharray={node.state === 'missing' ? '5 3' : undefined}
					/>
					<text
						x="10"
						y={NODE_HEIGHT / 2}
						dominant-baseline="central"
						class="text-xs {node.state === 'missing'
							? node.attention
								? 'fill-danger'
								: 'fill-muted'
							: 'fill-ink'} {node.state === 'missing' ? 'font-mono' : ''}"
					>
						{truncate(node.label)}
					</text>
				</g>
			{/each}
		</g>
	</svg>
	<div class="absolute top-2 right-2 flex gap-1">
		<Button
			icon="plus"
			onclick={() => zoomAt(1.25)}
			aria-label={m.tools_zoom_in()}
			title={m.tools_zoom_in()}
		></Button>
		<Button
			icon="minus"
			onclick={() => zoomAt(0.8)}
			aria-label={m.tools_zoom_out()}
			title={m.tools_zoom_out()}
		></Button>
		<Button
			icon="scan"
			onclick={fit}
			aria-label={m.editor_dependency_fit()}
			title={m.editor_dependency_fit()}
		></Button>
	</div>
</div>
