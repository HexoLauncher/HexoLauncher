import type { LoaderKind } from './api';

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
	if (!unixSeconds) return '從未遊玩';
	const d = new Date(unixSeconds * 1000);
	const pad = (n: number) => String(n).padStart(2, '0');
	return (
		`${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ` +
		`${pad(d.getHours())}:${pad(d.getMinutes())}`
	);
}

export function formatDate(unixSeconds: number): string {
	return unixSeconds ? formatTime(unixSeconds).split(' ')[0] : '從未遊玩';
}

export function formatDuration(seconds: number): string {
	if (seconds <= 0) return '0 分';
	const hours = Math.floor(seconds / 3600);
	const minutes = Math.floor((seconds % 3600) / 60);
	if (hours === 0) return `${minutes} 分`;
	return `${hours} 小時 ${String(minutes).padStart(2, '0')} 分`;
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

export function versionTypeLabel(type: string): string {
	switch (type) {
		case 'release':
			return '正式版';
		case 'snapshot':
			return '快照';
		case 'old_beta':
			return '舊 Beta';
		case 'old_alpha':
			return '舊 Alpha';
		default:
			return type;
	}
}

export function releaseDate(isoTime: string): string {
	return isoTime.split('T')[0] ?? isoTime;
}
