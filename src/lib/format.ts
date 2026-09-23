import type { LoaderKind } from './api';
import { m } from '$lib/paraglide/messages.js';
import { getLocale } from '$lib/paraglide/runtime.js';

const LOADER_LABELS: Record<LoaderKind, string> = {
	vanilla: 'Vanilla',
	fabric: 'Fabric',
	forge: 'Forge',
	neoforge: 'NeoForge'
};

export function loaderLabel(loader: LoaderKind): string {
	return LOADER_LABELS[loader] ?? loader;
}

export const LOADERS: LoaderKind[] = ['vanilla', 'fabric', 'forge', 'neoforge'];

export function formatTime(unixSeconds: number): string {
	if (!unixSeconds) return m.format_never_played();
	const d = new Date(unixSeconds * 1000);
	const pad = (n: number) => String(n).padStart(2, '0');
	return (
		`${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ` +
		`${pad(d.getHours())}:${pad(d.getMinutes())}`
	);
}

export function formatDate(unixSeconds: number): string {
	return unixSeconds ? formatTime(unixSeconds).split(' ')[0] : m.format_never_played();
}

export function formatDuration(seconds: number): string {
	if (seconds <= 0) return m.format_minutes({ minutes: 0 });
	const hours = Math.floor(seconds / 3600);
	const minutes = Math.floor((seconds % 3600) / 60);
	if (hours === 0) return m.format_minutes({ minutes });
	return m.format_hours_minutes({ hours, minutes: String(minutes).padStart(2, '0') });
}

export function formatSize(bytes: number): string {
	const units = ['B', 'KiB', 'MiB', 'GiB'];
	let value = bytes;
	let unit = 0;
	while (value >= 1024 && unit < units.length - 1) {
		value /= 1024;
		unit += 1;
	}
	return `${value.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

export function formatCount(n: number): string {
	return new Intl.NumberFormat(getLocale(), {
		notation: 'compact',
		maximumFractionDigits: 1
	}).format(n);
}

export function versionTypeLabel(type: string): string {
	switch (type) {
		case 'release':
			return m.version_type_release();
		case 'snapshot':
			return m.version_type_snapshot();
		case 'old_beta':
			return m.version_type_old_beta();
		case 'old_alpha':
			return m.version_type_old_alpha();
		default:
			return type;
	}
}

export function releaseDate(isoTime: string): string {
	return isoTime.split('T')[0] ?? isoTime;
}
