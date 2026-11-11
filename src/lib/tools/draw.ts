import type { Frame } from '$lib/components/tools/MapCanvas.svelte';
import { m } from '$lib/paraglide/messages.js';

export interface ThemeColors {
	bg: string;
	ink: string;
	line: string;
	brand: string;
}

export function themeColors(): ThemeColors {
	const css = getComputedStyle(document.documentElement);
	const pick = (name: string, fallback: string) => css.getPropertyValue(name).trim() || fallback;
	return {
		bg: pick('--color-deep', '#1d1518'),
		ink: pick('--color-ink', '#e8e8e8'),
		line: pick('--color-line', '#342d30'),
		brand: pick('--color-brand', '#e5343f')
	};
}

export type MarkerShape = 'dot' | 'diamond' | 'ring' | 'player';

export function drawMarker(
	ctx: CanvasRenderingContext2D,
	f: Frame,
	x: number,
	y: number,
	color: string,
	shape: MarkerShape
) {
	if (x < -12 || y < -12 || x > f.width + 12 || y > f.height + 12) return;
	ctx.beginPath();
	if (shape === 'diamond') {
		ctx.moveTo(x, y - 7);
		ctx.lineTo(x + 6, y);
		ctx.lineTo(x, y + 7);
		ctx.lineTo(x - 6, y);
		ctx.closePath();
	} else if (shape === 'player') {
		ctx.moveTo(x, y - 9);
		ctx.lineTo(x + 7, y + 6);
		ctx.lineTo(x, y + 2);
		ctx.lineTo(x - 7, y + 6);
		ctx.closePath();
	} else {
		ctx.arc(x, y, shape === 'ring' ? 7 : 4.5, 0, Math.PI * 2);
	}
	if (shape === 'ring') {
		ctx.lineWidth = 2.5;
		ctx.strokeStyle = color;
		ctx.stroke();
		return;
	}
	ctx.fillStyle = color;
	ctx.fill();
	ctx.lineWidth = 1.5;
	ctx.strokeStyle = 'rgba(0, 0, 0, 0.75)';
	ctx.stroke();
}

export function drawChunkGrid(ctx: CanvasRenderingContext2D, f: Frame) {
	const chunkPx = 16 / f.b;
	if (chunkPx < 24) return;
	const cx0 = Math.floor(f.left / 16);
	const cz0 = Math.floor(f.top / 16);
	ctx.strokeStyle = 'rgba(0, 0, 0, 0.18)';
	ctx.lineWidth = 1;
	ctx.beginPath();
	for (let i = 0; i <= f.width / chunkPx + 1; i++) {
		const x = Math.round(f.toX((cx0 + i) * 16)) + 0.5;
		ctx.moveTo(x, 0);
		ctx.lineTo(x, f.height);
	}
	for (let j = 0; j <= f.height / chunkPx + 1; j++) {
		const y = Math.round(f.toY((cz0 + j) * 16)) + 0.5;
		ctx.moveTo(0, y);
		ctx.lineTo(f.width, y);
	}
	ctx.stroke();
}

export function visibleRegions(f: Frame) {
	return {
		rx0: Math.floor(f.left / 512),
		rz0: Math.floor(f.top / 512),
		rx1: Math.floor((f.left + f.width * f.b) / 512),
		rz1: Math.floor((f.top + f.height * f.b) / 512)
	};
}

export function centerDistance(f: Frame, x: number, z: number) {
	return Math.hypot(f.toX(x) - f.width / 2, f.toY(z) - f.height / 2);
}

export function scaleText(zoom: number) {
	const b = 2 ** zoom;
	return b >= 1
		? m.tools_scale({ blocks: Math.round(b * 10) / 10 })
		: m.tools_scale_in({ px: Math.round(1 / b) });
}

export function formatTicks(ticks: number) {
	const minutes = Math.floor(ticks / 1200);
	if (minutes < 1) return m.tools_seconds({ count: Math.floor(ticks / 20) });
	if (minutes < 60) return m.tools_minutes({ count: minutes });
	return m.tools_hours_minutes({ hours: Math.floor(minutes / 60), minutes: minutes % 60 });
}

/** 預估耗時的文字。 */
export function estimateText(seconds: number) {
	if (seconds < 1) return m.tools_estimate_instant();
	if (seconds < 90) return m.tools_estimate({ seconds: Math.round(seconds) });
	return m.tools_estimate_minutes({ minutes: Math.round(seconds / 60) });
}

/** 秒數 → 「12 秒」「3 分 20 秒」。 */
export function formatDuration(seconds: number) {
	if (seconds < 1) return m.tools_duration_under_second();
	const total = Math.round(seconds);
	if (total < 60) return m.tools_duration_seconds({ seconds: total });
	return m.tools_duration_minutes({ minutes: Math.floor(total / 60), seconds: total % 60 });
}
