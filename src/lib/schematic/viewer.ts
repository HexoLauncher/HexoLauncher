import init, { App, VanillaAssets, load_vanilla, minecraft_versions } from './pkg/web.js';
import wasmUrl from './pkg/web_bg.wasm?url';

let ready: Promise<unknown> | undefined;

export function loadViewer() {
	return (ready ??= init({ module_or_path: wasmUrl }));
}

export { App, VanillaAssets };

let teardown: Promise<unknown> = Promise.resolve();

export async function createApp(canvas: HTMLCanvasElement): Promise<App> {
	await loadViewer();
	await teardown;
	return new App(canvas);
}

export function destroyApp(app: App) {
	teardown = app
		.destroy()
		.catch(() => {})
		.finally(() => app.free());
}

export function loadVanilla(version: string, progress: (message: string) => void) {
	return load_vanilla(version, progress) as Promise<VanillaAssets>;
}

export function minecraftVersions() {
	return minecraft_versions() as Promise<string[]>;
}

export function base64ToBytes(base64: string): Uint8Array {
	const binary = atob(base64);
	const bytes = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
	return bytes;
}

export async function fetchLocal(path: string): Promise<Uint8Array> {
	const { convertFileSrc } = await import('@tauri-apps/api/tauri');
	const response = await fetch(convertFileSrc(path));
	if (!response.ok) throw new Error(`HTTP ${response.status}: ${path}`);
	return new Uint8Array(await response.arrayBuffer());
}

export interface MeshReport {
	summary: string;
	warnings: string[];
}

export function parseReport(report: string | undefined | null): MeshReport | null {
	if (!report) return null;
	const [summary = '', ...rest] = report.split('\n');
	return {
		summary,
		warnings: rest.filter((line) => line.trim()).map((l) => l.replace(/^⚠\s*/, ''))
	};
}
