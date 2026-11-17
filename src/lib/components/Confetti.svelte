<script lang="ts">
	import { onMount } from 'svelte';

	let { duration = 4200 }: { duration?: number } = $props();

	let canvas: HTMLCanvasElement;

	type Piece = {
		x: number;
		y: number;
		vx: number;
		vy: number;
		size: number;
		ratio: number;
		angle: number;
		spin: number;
		tilt: number;
		tiltSpeed: number;
		color: string;
		round: boolean;
	};

	onMount(() => {
		if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
		const ctx = canvas.getContext('2d');
		if (!ctx) return;

		const brand = getComputedStyle(document.documentElement).getPropertyValue('--color-brand').trim();
		const colors = [brand || '#e5343f', '#ffd166', '#06d6a0', '#4cc9f0', '#f72585', '#ffffff'];
		const dpr = window.devicePixelRatio || 1;
		let width = 0;
		let height = 0;

		const resize = () => {
			width = canvas.clientWidth;
			height = canvas.clientHeight;
			canvas.width = width * dpr;
			canvas.height = height * dpr;
			ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
		};
		resize();

		const pieces: Piece[] = [];
		const burst = (originX: number, originY: number, count: number, direction: number) => {
			for (let i = 0; i < count; i++) {
				const angle = direction + (Math.random() - 0.5) * 1.1;
				const speed = 7 + Math.random() * 9;
				pieces.push({
					x: originX,
					y: originY,
					vx: Math.cos(angle) * speed,
					vy: Math.sin(angle) * speed,
					size: 5 + Math.random() * 6,
					ratio: 0.4 + Math.random() * 0.5,
					angle: Math.random() * Math.PI * 2,
					spin: (Math.random() - 0.5) * 0.3,
					tilt: Math.random() * Math.PI * 2,
					tiltSpeed: 0.08 + Math.random() * 0.12,
					color: colors[Math.floor(Math.random() * colors.length)],
					round: Math.random() < 0.25
				});
			}
		};

		burst(0, height * 0.9, 90, -Math.PI / 3);
		burst(width, height * 0.9, 90, (-Math.PI * 2) / 3);
		const second = setTimeout(() => burst(width / 2, height * 0.35, 70, -Math.PI / 2), 350);

		const start = performance.now();
		let frame = 0;
		let last = start;

		const tick = (now: number) => {
			const dt = Math.min((now - last) / 16.67, 3);
			last = now;
			const fade = Math.max(0, Math.min(1, (duration - (now - start)) / 800));
			ctx.clearRect(0, 0, width, height);

			for (const p of pieces) {
				p.vx *= Math.pow(0.985, dt);
				p.vy = p.vy * Math.pow(0.985, dt) + 0.28 * dt;
				p.x += p.vx * dt;
				p.y += p.vy * dt;
				p.angle += p.spin * dt;
				p.tilt += p.tiltSpeed * dt;

				ctx.save();
				ctx.globalAlpha = fade;
				ctx.translate(p.x, p.y);
				ctx.rotate(p.angle);
				ctx.scale(1, Math.cos(p.tilt));
				ctx.fillStyle = p.color;
				if (p.round) {
					ctx.beginPath();
					ctx.arc(0, 0, p.size / 2.4, 0, Math.PI * 2);
					ctx.fill();
				} else {
					ctx.fillRect(-p.size / 2, (-p.size * p.ratio) / 2, p.size, p.size * p.ratio);
				}
				ctx.restore();
			}

			if (fade > 0) frame = requestAnimationFrame(tick);
			else ctx.clearRect(0, 0, width, height);
		};
		frame = requestAnimationFrame(tick);
		window.addEventListener('resize', resize);

		return () => {
			clearTimeout(second);
			cancelAnimationFrame(frame);
			window.removeEventListener('resize', resize);
		};
	});
</script>

<canvas bind:this={canvas} class="pointer-events-none absolute inset-0 size-full" aria-hidden="true"></canvas>
