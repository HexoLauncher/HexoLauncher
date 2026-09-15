import * as api from './api';
import type { AccountView, InstanceView, LauncherConfig, RunningGame } from './api';

export const app = $state({
	ready: false,
	baseDir: '',
	defaultBaseDir: '',
	storeDir: '',
	config: defaultConfig(),
	instances: [] as InstanceView[],
	accounts: [] as AccountView[],

	running: [] as RunningGame[],
	selectedId: null as string | null,
	search: '',
	status: '就緒'
});

export function defaultConfig(): LauncherConfig {
	return {
		formatVersion: 1,
		baseDir: '',
		ui: { instanceToolbar: true, statusBar: true, runningBadge: true, includeSnapshots: false },
		game: {
			minMemoryMb: 512,
			maxMemoryMb: 4096,
			jvmArgs: '',
			recordPlayTime: true,
			showProgressWindow: true,
			closeProgressWhenDone: true,
			keepLogLines: 5000
		},
		java: { path: '' },
		commands: { preLaunch: '', postExit: '' },
		api: { curseforgeApiKey: '' }
	};
}

let statusTimer: ReturnType<typeof setTimeout> | undefined;

export function report(message: string) {
	app.status = message;
	clearTimeout(statusTimer);
	statusTimer = setTimeout(() => (app.status = '就緒'), 5000);
}

export function reportError(prefix: string, error: unknown) {
	app.status = `${prefix}：${api.errorMessage(error)}`;
	clearTimeout(statusTimer);
	statusTimer = setTimeout(() => (app.status = '就緒'), 12000);
}

export async function init() {
	try {
		const [paths, config] = await Promise.all([api.resolvePaths(), api.getLauncherConfig()]);
		app.baseDir = paths.baseDir;
		app.defaultBaseDir = paths.defaultBaseDir;
		app.storeDir = paths.storeDir;
		app.config = config;
	} catch (e) {
		reportError('無法讀取啟動器設定', e);
		return;
	}

	await Promise.all([refreshInstances(), refreshAccounts(), refreshRunning()]);
	app.ready = true;
}

export async function refreshInstances() {
	if (!app.baseDir) return;
	try {
		app.instances = await api.listInstances(app.baseDir);
		if (app.selectedId && !app.instances.some((i) => i.id === app.selectedId)) {
			app.selectedId = null;
		}
	} catch (e) {
		reportError('無法讀取實例清單', e);
	}
}

export async function refreshAccounts() {
	try {
		app.accounts = await api.listAccounts();
	} catch (e) {
		reportError('無法讀取帳號', e);
	}
}

export async function refreshRunning() {
	try {
		app.running = await api.runningInstances();
	} catch (e) {
		reportError('無法取得執行中的實例', e);
	}
}

export async function saveConfig(patch?: Partial<LauncherConfig>) {
	const next = $state.snapshot({ ...app.config, ...patch }) as LauncherConfig;
	try {
		app.config = await api.saveLauncherConfig(next);
	} catch (e) {
		reportError('設定存檔失敗', e);
		return;
	}

	const paths = await api.resolvePaths();
	if (paths.baseDir !== app.baseDir) {
		app.baseDir = paths.baseDir;
		await refreshInstances();
	}
}

let saveTimer: ReturnType<typeof setTimeout> | undefined;

export function saveConfigSoon(delay = 400) {
	clearTimeout(saveTimer);
	saveTimer = setTimeout(() => saveConfig(), delay);
}

export function findInstance(id: string | null): InstanceView | null {
	if (!id) return null;
	return app.instances.find((i) => i.id === id) ?? null;
}

export function isRunning(id: string | null): boolean {
	return runningOf(id) !== null;
}

export function runningOf(id: string | null): RunningGame | null {
	if (!id) return null;
	return app.running.find((r) => r.instanceName === id) ?? null;
}

export function runningIds(): string[] {
	return app.running.map((r) => r.instanceName);
}

export function selectedAccount(): AccountView | null {
	return app.accounts.find((a) => a.selected) ?? null;
}

export function groupsOf(instances: InstanceView[]): string[] {
	const present = instances.map((i) => i.group).filter((g, i, all) => all.indexOf(g) === i);
	const named = present.filter((g) => g !== '').sort((a, b) => a.localeCompare(b, 'zh-Hant'));
	return [...named, ...(present.includes('') ? [''] : [])];
}

export function knownGroups(): string[] {
	return groupsOf(app.instances).filter((g) => g !== '');
}
