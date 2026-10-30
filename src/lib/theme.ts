import { m } from '$lib/paraglide/messages.js';

export type ThemeMode = 'dark' | 'light' | 'system';

export interface ThemeConfig {
	mode: ThemeMode;

	preset: string;

	accent: string;

	tint: number;

	opacity: number;

	background: string;

	backgroundDim: number;

	backgroundBlur: number;
}

export const MODES: { id: ThemeMode; label: string; icon: string }[] = [
	{ id: 'dark', label: m.theme_mode_dark(), icon: 'moon' },
	{ id: 'light', label: m.theme_mode_light(), icon: 'sun' },
	{ id: 'system', label: m.theme_mode_system(), icon: 'monitor' }
];

export interface ThemePreset {
	id: string;
	label: string;
	accent: string;
}

export const PRESETS: ThemePreset[] = [
	{ id: 'crimson', label: m.theme_preset_crimson(), accent: '#e5343f' },
	{ id: 'ember', label: m.theme_preset_ember(), accent: '#f2652b' },
	{ id: 'sakura', label: m.theme_preset_sakura(), accent: '#ff5d8f' },
	{ id: 'emerald', label: m.theme_preset_emerald(), accent: '#1bd96a' },
	{ id: 'ocean', label: m.theme_preset_ocean(), accent: '#3b82f6' },
	{ id: 'amethyst', label: m.theme_preset_amethyst(), accent: '#a855f7' }
];

const NEUTRAL = { dark: '#16181c', light: '#eceef2' };

export function defaultTheme(): ThemeConfig {
	return {
		mode: 'dark',
		preset: PRESETS[0].id,
		accent: PRESETS[0].accent,
		tint: 0.5,
		opacity: 0.82,
		background: '',
		backgroundDim: 0.35,
		backgroundBlur: 0
	};
}

export function isDarkMode(mode: ThemeMode | undefined): boolean {
	if (mode === 'light') return false;
	if (mode === 'dark' || mode === undefined) return true;
	if (typeof window === 'undefined' || !window.matchMedia) return true;
	return window.matchMedia('(prefers-color-scheme: dark)').matches;
}

type RGB = [number, number, number];

const clamp = (v: number, lo: number, hi: number) => (v < lo ? lo : v > hi ? hi : v);

export function parseHex(hex: string): RGB | null {
	const m = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec((hex ?? '').trim());
	if (!m) return null;
	const raw = m[1];
	const full =
		raw.length === 3
			? raw
					.split('')
					.map((c) => c + c)
					.join('')
			: raw;
	return [
		parseInt(full.slice(0, 2), 16),
		parseInt(full.slice(2, 4), 16),
		parseInt(full.slice(4, 6), 16)
	];
}

export function toHex(c: RGB): string {
	return '#' + c.map((v) => clamp(Math.round(v), 0, 255).toString(16).padStart(2, '0')).join('');
}

function mix(a: RGB, b: RGB, t: number): RGB {
	return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
}

function lighten(c: RGB, t: number): RGB {
	return mix(c, [255, 255, 255], t);
}

function darken(c: RGB, t: number): RGB {
	return mix(c, [0, 0, 0], t);
}

function luminance(c: RGB): number {
	const lin = c.map((v) => {
		const s = v / 255;
		return s <= 0.04045 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
	});
	return 0.2126 * lin[0] + 0.7152 * lin[1] + 0.0722 * lin[2];
}

function contrast(a: RGB, b: RGB): number {
	const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
	return (hi + 0.05) / (lo + 0.05);
}

function readable(color: RGB, bg: RGB, dark: boolean, target = 4.5): RGB {
	let c = color;
	for (let i = 0; i < 40 && contrast(c, bg) < target; i++) {
		c = dark ? lighten(c, 0.05) : darken(c, 0.05);
	}
	return c;
}

function css(c: RGB, alpha = 1): string {
	if (alpha >= 1) return toHex(c);
	const [r, g, b] = c.map((v) => clamp(Math.round(v), 0, 255));
	return `rgb(${r} ${g} ${b} / ${Math.round(alpha * 1000) / 1000})`;
}

const RAMP = {
	inputOff: 0.017,
	row: 0.022,
	window: 0.034,
	group: 0.047,
	btnLo: 0.06,
	btnOff: 0.06,
	raised: 0.069,
	card: 0.077,
	line: 0.103,
	btn: 0.107,
	btnHi: 0.155,
	lineHi: 0.167,
	scrollHi: 0.236,
	faint: 0.28,
	muted: 0.57,
	ink: 0.9
};

const LIGHT: Record<keyof typeof RAMP, number> = {
	inputOff: -0.02,
	row: 0.35,
	window: 0.55,
	group: 0.82,
	btnLo: -0.03,
	btnOff: -0.025,
	raised: 0.95,
	card: 0.9,
	line: -0.08,
	btn: -0.05,
	btnHi: -0.09,
	lineHi: -0.14,
	scrollHi: -0.24,
	faint: 0,
	muted: 0,
	ink: 0
};

export function themeVars(theme: ThemeConfig): Record<string, string> {
	const accent = parseHex(theme.accent) ?? (parseHex(defaultTheme().accent) as RGB);
	const dark = isDarkMode(theme.mode);
	const neutral = parseHex(dark ? NEUTRAL.dark : NEUTRAL.light) as RGB;
	const tint = clamp(theme.tint ?? 0, 0, 1);

	const base = dark
		? mix(neutral, darken(accent, 0.78), tint * 0.5)
		: mix(neutral, lighten(accent, 0.55), tint * 0.22);

	const surface = (key: keyof typeof RAMP) => {
		if (dark) return lighten(base, RAMP[key]);
		const t = LIGHT[key];
		return t >= 0 ? lighten(base, t) : darken(base, -t);
	};
	const text = (light: number, deep: number) => (dark ? lighten(base, light) : darken(base, deep));

	const opacity = theme.background ? clamp(theme.opacity ?? 1, 0.25, 1) : 1;
	const alpha = dark ? opacity : 1 - (1 - opacity) * 0.6;

	const brandInk =
		luminance(accent) > 0.35 ? darken(accent, 0.9) : mix([255, 255, 255], accent, 0.07);

	const blur = clamp(theme.backgroundBlur ?? 0, 0, 40);
	const dim = clamp(theme.backgroundDim ?? 0, 0, 0.9);

	return {
		'--color-backdrop': css(base),
		'--color-shell': theme.background ? 'transparent' : css(surface('window')),

		'--color-deep': css(base, alpha),
		'--color-window': css(surface('window'), alpha),
		'--color-row': css(surface('row'), alpha),

		'--color-group': css(surface('group')),
		'--color-input-off': css(surface('inputOff')),
		'--color-raised': css(surface('raised')),
		'--color-card': css(surface('card')),
		'--color-btn': css(surface('btn')),
		'--color-btn-hi': css(surface('btnHi')),
		'--color-btn-lo': css(surface('btnLo')),
		'--color-btn-off': css(surface('btnOff')),
		'--color-line': css(surface('line')),
		'--color-line-hi': css(surface('lineHi')),
		'--color-scroll-hi': css(surface('scrollHi')),
		'--color-scrim': dark ? css(darken(base, 0.6), 0.6) : css(darken(base, 0.45), 0.35),

		'--color-ink': css(text(RAMP.ink, 0.88)),

		'--color-muted': css(text(RAMP.muted, 0.66)),

		'--color-faint': css(readable(text(RAMP.faint, 0.38), base, dark)),
		'--color-hover': dark ? 'rgb(255 255 255 / 0.05)' : 'rgb(0 0 0 / 0.05)',

		'--color-brand': css(accent),
		'--color-brand-fg': css(readable(accent, base, dark)),
		'--color-brand-hi': css(lighten(accent, 0.18)),
		'--color-brand-lo': css(darken(accent, 0.16)),
		'--color-brand-soft': css(accent, dark ? 0.13 : 0.11),
		'--color-brand-soft-hi': css(accent, dark ? 0.22 : 0.18),
		'--color-brand-line': css(accent, 0.42),
		'--color-brand-ink': css(brandInk),
		'--color-knob-on': dark ? css(darken(base, 0.55)) : '#ffffff',

		'--color-danger': dark ? '#ff6b6b' : '#d33a3a',
		'--color-danger-hi': dark ? '#ff8585' : '#bd2f2f',
		'--color-danger-soft': dark ? '#ff6b6b24' : '#d33a3a1f',
		'--color-danger-ink': dark ? '#2a0b0b' : '#ffffff',
		'--color-warn': dark ? '#ffa347' : '#e8890c',

		'--shadow-sm': dark ? '0 1px 3px #0000004d' : '0 1px 3px rgb(0 0 0 / 0.1)',
		'--shadow-md': dark ? '0 6px 20px #00000066' : '0 6px 20px rgb(0 0 0 / 0.12)',
		'--shadow-group': dark ? 'none' : '0 1px 2px rgb(0 0 0 / 0.04), 0 4px 14px rgb(0 0 0 / 0.04)',
		'--shadow-lg': dark ? '0 24px 64px #000000a6' : '0 24px 64px rgb(0 0 0 / 0.2)',

		'--app-color-scheme': dark ? 'dark' : 'light',

		'--app-bg-blur': `${blur}px`,

		'--app-bg-scale': String(Math.round((1 + (blur / 100) * 1.2) * 1000) / 1000),
		'--app-bg-dim': `${dark ? dim : Math.round((0.3 + dim * 0.6) * 1000) / 1000}`,
		'--app-bg-dim-color': dark ? '0 0 0' : '255 255 255'
	};
}

let bgCache: { name: string; url: string } | null = null;

function setBackground(theme: ThemeConfig, root: HTMLElement) {
	if (!theme.background) {
		bgCache = null;
		root.style.setProperty('--app-bg-image', 'none');
		return Promise.resolve();
	}

	if (bgCache?.name === theme.background) {
		root.style.setProperty('--app-bg-image', `url("${bgCache.url}")`);
		return Promise.resolve();
	}

	return Promise.all([import('./api'), import('@tauri-apps/api/core')])
		.then(([api, tauri]) =>
			api.backgroundImagePath(theme.background).then((path) => tauri.convertFileSrc(path))
		)
		.then((url) => {
			bgCache = { name: theme.background, url };
			root.style.setProperty('--app-bg-image', `url("${url}")`);
		})
		.catch(() => {
			bgCache = null;
			root.style.setProperty('--app-bg-image', 'none');
		});
}

let current: ThemeConfig | null = null;
let watchingSystem = false;

function watchSystemScheme() {
	if (watchingSystem || typeof window === 'undefined' || !window.matchMedia) return;
	watchingSystem = true;
	window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
		if (current?.mode === 'system') applyTheme(current);
	});
}

export function applyTheme(theme?: Partial<ThemeConfig> | null): Promise<void> {
	if (typeof document === 'undefined') return Promise.resolve();

	const full: ThemeConfig = { ...defaultTheme(), ...(theme ?? {}) };
	current = full;
	watchSystemScheme();

	const root = document.documentElement;
	for (const [key, value] of Object.entries(themeVars(full))) {
		root.style.setProperty(key, value);
	}
	return setBackground(full, root);
}

export function backgroundUrl(): string {
	return bgCache?.url ?? '';
}

export async function loadTheme(): Promise<void> {
	try {
		const api = await import('./api');
		const config = await api.getLauncherConfig();
		await applyTheme(config.theme);
	} catch {
		return;
	}
}
