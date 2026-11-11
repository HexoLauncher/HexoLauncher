<script lang="ts" module>
	/** 一幀的座標換算：方塊座標 → 畫布上的 CSS 像素。 */
	export interface Frame {
		width: number;
		height: number;
		/** 畫面左上角的方塊座標。 */
		left: number;
		top: number;
		/** 每像素幾格。 */
		b: number;
		zoom: number;
		toX: (x: number) => number;
		toY: (z: number) => number;
	}

	/** 框選結果：區塊座標，含兩端。 */
	export interface ChunkRect {
		cx0: number;
		cz0: number;
		cx1: number;
		cz1: number;
	}
</script>

<script lang="ts">
	import { onDestroy, onMount, type Snippet } from 'svelte';
	import type { MapView } from '$lib/tools/state.svelte';

	const MIN_ZOOM = -3;
	const MAX_ZOOM = 13;

	let {
		view,
		draw,
		describe,
		mode = 'pan',
		onselectrect,
		onblockclick,
		overlay
	}: {
		/** 視角物件（會直接修改它的欄位）。 */
		view: MapView;
		draw: (ctx: CanvasRenderingContext2D, frame: Frame) => void;
		/** 滑鼠所在位置要顯示的文字，每行一個字串。 */
		describe?: (x: number, z: number, sx: number, sy: number) => string[];
		mode?: 'pan' | 'select';
		onselectrect?: (rect: ChunkRect, additive: boolean) => void;
		onblockclick?: (x: number, z: number) => void;
		overlay?: Snippet;
	} = $props();

	let container: HTMLDivElement;
	let canvas: HTMLCanvasElement;
	let width = $state(0);
	let height = $state(0);
	let hoverLines = $state<string[]>([]);
	let drag = $state<{ x: number; y: number; vx: number; vz: number; id: number; moved: boolean } | null>(null);
	let selecting = $state<{ x0: number; z0: number; x1: number; z1: number; additive: boolean; id: number } | null>(
		null
	);
	let lastPointer: { x: number; y: number } | null = null;
	let frameId = 0;

	const b = $derived(2 ** view.zoom);

	/** 要求下一幀重畫。 */
	export function redraw() {
		if (!frameId) frameId = requestAnimationFrame(paint);
	}

	/** 重新計算滑鼠所在位置的說明（例如資料載入完成後）。 */
	export function refreshHover() {
		if (lastPointer) updateHover(lastPointer.x, lastPointer.y);
	}

	/** 縮放到能完整看到以目前中心為圓心、半徑 `radius` 格的範圍；畫布尚未有尺寸時回傳 false。 */
	export function fit(radius: number) {
		// 直接量容器：版面剛變動時 ResizeObserver 還沒更新 width / height。
		const size = Math.min(container?.clientWidth ?? 0, container?.clientHeight ?? 0);
		if (!size) return false;
		view.zoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, Math.log2((radius * 2.3) / size)));
		return true;
	}

	export function zoomBy(delta: number) {
		zoomAt(view.zoom + delta, width / 2, height / 2);
	}

	$effect(() => {
		void [view.x, view.z, view.zoom, width, height, selecting, mode];
		redraw();
	});

	function frame(): Frame {
		const left = view.x - (width / 2) * b;
		const top = view.z - (height / 2) * b;
		return {
			width,
			height,
			left,
			top,
			b,
			zoom: view.zoom,
			toX: (x) => (x - left) / b,
			toY: (z) => (z - top) / b
		};
	}

	function paint() {
		frameId = 0;
		const ctx = canvas?.getContext('2d');
		if (!ctx || !width || !height) return;
		const dpr = window.devicePixelRatio || 1;
		ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
		ctx.imageSmoothingEnabled = false;
		const f = frame();
		draw(ctx, f);
		if (selecting) {
			const r = chunkRect(selecting);
			const x = f.toX(r.cx0 * 16);
			const y = f.toY(r.cz0 * 16);
			const w = ((r.cx1 - r.cx0 + 1) * 16) / f.b;
			const h = ((r.cz1 - r.cz0 + 1) * 16) / f.b;
			ctx.fillStyle = 'rgba(229, 52, 63, 0.18)';
			ctx.fillRect(x, y, w, h);
			ctx.strokeStyle = 'rgba(229, 52, 63, 0.95)';
			ctx.lineWidth = 1.5;
			ctx.strokeRect(x, y, w, h);
		}
	}

	function chunkRect(s: { x0: number; z0: number; x1: number; z1: number }): ChunkRect {
		return {
			cx0: Math.floor(Math.min(s.x0, s.x1) / 16),
			cz0: Math.floor(Math.min(s.z0, s.z1) / 16),
			cx1: Math.floor(Math.max(s.x0, s.x1) / 16),
			cz1: Math.floor(Math.max(s.z0, s.z1) / 16)
		};
	}

	function local(event: { clientX: number; clientY: number }) {
		const rect = canvas.getBoundingClientRect();
		return { x: event.clientX - rect.left, y: event.clientY - rect.top };
	}

	function blockAt(sx: number, sy: number) {
		return { x: view.x + (sx - width / 2) * b, z: view.z + (sy - height / 2) * b };
	}

	function pointerDown(event: PointerEvent) {
		if (event.button !== 0) return;
		canvas.setPointerCapture(event.pointerId);
		const p = local(event);
		if (mode === 'select') {
			const w = blockAt(p.x, p.y);
			selecting = { x0: w.x, z0: w.z, x1: w.x, z1: w.z, additive: event.shiftKey || event.metaKey || event.ctrlKey, id: event.pointerId };
			return;
		}
		drag = { x: event.clientX, y: event.clientY, vx: view.x, vz: view.z, id: event.pointerId, moved: false };
	}

	function pointerMove(event: PointerEvent) {
		const p = local(event);
		lastPointer = p;
		if (drag) {
			const dx = event.clientX - drag.x;
			const dy = event.clientY - drag.y;
			if (Math.abs(dx) + Math.abs(dy) > 3) drag.moved = true;
			view.x = drag.vx - dx * b;
			view.z = drag.vz - dy * b;
		} else if (selecting) {
			const w = blockAt(p.x, p.y);
			selecting = { ...selecting, x1: w.x, z1: w.z };
		}
		updateHover(p.x, p.y);
	}

	function pointerUp(event: PointerEvent) {
		if (selecting?.id === event.pointerId) {
			onselectrect?.(chunkRect(selecting), selecting.additive);
			selecting = null;
		}
		if (drag?.id === event.pointerId) {
			if (!drag.moved) {
				const p = local(event);
				const w = blockAt(p.x, p.y);
				onblockclick?.(Math.floor(w.x), Math.floor(w.z));
			}
			drag = null;
		}
	}

	function updateHover(sx: number, sy: number) {
		const w = blockAt(sx, sy);
		hoverLines = describe?.(Math.floor(w.x), Math.floor(w.z), sx, sy) ?? [];
	}

	function zoomAt(next: number, sx: number, sy: number) {
		next = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, next));
		const w = blockAt(sx, sy);
		const nb = 2 ** next;
		view.zoom = next;
		view.x = w.x - (sx - width / 2) * nb;
		view.z = w.z - (sy - height / 2) * nb;
	}

	function wheel(event: WheelEvent) {
		event.preventDefault();
		const p = local(event);
		const step = event.deltaMode === 1 ? event.deltaY * 0.05 : event.deltaY * 0.0022;
		zoomAt(view.zoom + step, p.x, p.y);
	}

	function dblclick(event: MouseEvent) {
		if (mode !== 'pan') return;
		const p = local(event);
		zoomAt(view.zoom - 1, p.x, p.y);
	}

	onMount(() => {
		const observer = new ResizeObserver(() => {
			const dpr = window.devicePixelRatio || 1;
			width = container.clientWidth;
			height = container.clientHeight;
			canvas.width = Math.round(width * dpr);
			canvas.height = Math.round(height * dpr);
			redraw();
		});
		observer.observe(container);
		// 需要非被動監聽才能擋下 webview 本身的縮放與捲動。
		canvas.addEventListener('wheel', wheel, { passive: false });
		return () => {
			observer.disconnect();
			canvas.removeEventListener('wheel', wheel);
		};
	});

	onDestroy(() => {
		if (frameId) cancelAnimationFrame(frameId);
	});
</script>

<div class="relative min-w-0 flex-1" bind:this={container}>
	<canvas
		bind:this={canvas}
		class="absolute inset-0 h-full w-full touch-none {mode === 'select'
			? 'cursor-crosshair'
			: drag
				? 'cursor-grabbing'
				: 'cursor-grab'}"
		onpointerdown={pointerDown}
		onpointermove={pointerMove}
		onpointerup={pointerUp}
		onpointercancel={pointerUp}
		onpointerleave={() => {
			lastPointer = null;
			hoverLines = [];
		}}
		ondblclick={dblclick}
	></canvas>

	{@render overlay?.()}

	{#if hoverLines.length}
		<div
			class="pointer-events-none absolute bottom-2 left-2 flex max-w-[calc(100%-1rem)] flex-col gap-0.5 rounded-md bg-scrim px-2.5 py-1.5 text-[12px] text-ink tabular-nums"
		>
			{#each hoverLines as line, i (i)}
				<span class="truncate {i === 0 && hoverLines.length > 1 ? 'font-medium' : ''}">{line}</span>
			{/each}
		</div>
	{/if}
</div>
