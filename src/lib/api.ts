/**
 * HexoLauncher 後端（src-tauri）的型別化介面。
 *
 * 實際的 Minecraft 邏輯都在 hexomc-lib（crates.io）裡，這層只負責把
 * 資料夾等資訊傳進去，並把結果 / 進度事件帶回前端。
 *
 * 所有 API 的第一個資訊都是 `baseDir` —— 啟動器根目錄由前端決定並傳入，
 * 後端不會自己記住。用 `defaultBaseDir()` 取得建議值。
 */
import { invoke } from '@tauri-apps/api/tauri';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// ---------- 型別 ----------

export type LoaderKind = 'vanilla' | 'fabric' | 'forge' | 'neoforge';

/** 後端丟出來的錯誤（invoke 的 reject 值） */
export interface HexoCmdError {
	/** network / io / javaNotFound / instanceNotFound / alreadyRunning / authError … */
	kind: string;
	message: string;
}

export interface BaseDirInfo {
	path: string;
	exists: boolean;
}

export interface VersionSummary {
	id: string;
	versionType: string;
	releaseTime: string;
}

export interface VersionList {
	latestRelease: string;
	latestSnapshot: string;
	versions: VersionSummary[];
}

export interface InstanceSummary {
	name: string;
	versionId: string;
	loader: LoaderKind;
	mainClass: string;
	javaVersion: number;
	assetsId: string;
	dir: string;
	minecraftDir: string;
}

export interface JavaInfo {
	path: string;
	version: number;
}

export interface Account {
	playerName: string;
	uuid: string;
	accessToken: string;
	refreshToken: string;
	xuid: string;
	expiresAt: number;
}

/** 啟動用的帳號；只給 playerName 就是離線模式 */
export interface LaunchAccount {
	playerName: string;
	accessToken?: string | null;
	uuid?: string | null;
	xuid?: string | null;
}

export interface LaunchRequest {
	baseDir: string;
	instanceName: string;
	account: LaunchAccount;
	javaPath?: string | null;
	jvmArgs?: string[] | null;
	maxMemoryMb?: number | null;
	minMemoryMb?: number | null;
	autoDownloadJava?: boolean | null;
}

export interface LaunchInfo {
	instanceName: string;
	pid: number;
	javaPath: string;
	javaVersion: number;
	online: boolean;
}

export interface DeviceCode {
	sessionId: string;
	userCode: string;
	verificationUri: string;
	message: string;
	expiresIn: number;
}

export interface ModEntry {
	fileName: string;
	filePath: string;
	modId: string;
	name: string;
	version: string;
	loader: string;
	description: string | null;
}

export interface ModUpdateEntry {
	id: string;
	modId: string;
	name: string;
	currentVersion: string;
	newVersion: string;
	newFileName: string | null;
	source: 'modrinth' | 'curseforge';
}

export interface PackEntry {
	name: string;
	path: string;
	isFolder: boolean;
}

export interface WorldEntry {
	name: string;
	path: string;
	/** icon.png 的 data URI */
	icon: string | null;
}

export interface InstallProgress {
	instanceName: string;
	done: number;
	total: number;
	message: string;
}

export interface GameExit {
	instanceName: string;
	exitCode: number | null;
}

// ---------- 目錄 ----------

export const defaultBaseDir = () => invoke<BaseDirInfo>('default_base_dir');

/** 建立 base_dir 底下的 assets / libraries / instance / java */
export const prepareBaseDir = (baseDir: string) =>
	invoke<BaseDirInfo>('prepare_base_dir', { baseDir });

export const openExternal = (url: string) => invoke<void>('open_external', { url });

// ---------- 版本 ----------

export const listVersions = (includeSnapshots = false) =>
	invoke<VersionList>('list_versions', { includeSnapshots });

/** 某個 MC 版本可用的 loader 版本（vanilla 回空陣列） */
export const listLoaderVersions = (loader: LoaderKind, mcVersion: string) =>
	invoke<string[]>('list_loader_versions', { loader, mcVersion });

// ---------- 安裝 ----------

/**
 * 安裝 instance。過程中持續發出 install-progress 事件，用 `onInstallProgress` 訂閱。
 * `instanceName` 由前端決定（同版本不同 loader 才不會撞資料夾）。
 */
export const installInstance = (args: {
	baseDir: string;
	mcVersion: string;
	instanceName: string;
	loader: LoaderKind;
	loaderVersion?: string | null;
	javaPath?: string | null;
}) => invoke<InstanceSummary>('install_instance', args);

export const onInstallProgress = (handler: (progress: InstallProgress) => void): Promise<UnlistenFn> =>
	listen<InstallProgress>('hexo://install-progress', (event) => handler(event.payload));

// ---------- instance ----------

export const listInstances = (baseDir: string) =>
	invoke<InstanceSummary[]>('list_instances', { baseDir });

export const getInstance = (baseDir: string, instanceName: string) =>
	invoke<InstanceSummary>('get_instance', { baseDir, instanceName });

export const deleteInstance = (baseDir: string, instanceName: string) =>
	invoke<void>('delete_instance', { baseDir, instanceName });

export const openInstanceFolder = (baseDir: string, instanceName: string) =>
	invoke<void>('open_instance_folder', { baseDir, instanceName });

// ---------- Java ----------

export const detectJava = (requiredVersion = 21) =>
	invoke<JavaInfo | null>('detect_java', { requiredVersion });

export const installJava = (baseDir: string, version: number) =>
	invoke<JavaInfo>('install_java', { baseDir, version });

/** 先找系統的，找不到再下載 */
export const ensureJava = (baseDir: string, requiredVersion: number, allowDownload = true) =>
	invoke<JavaInfo>('ensure_java', { baseDir, requiredVersion, allowDownload });

// ---------- 啟動 ----------

export const launchInstance = (options: LaunchRequest) =>
	invoke<LaunchInfo>('launch_instance', { options });

export const stopInstance = (instanceName: string) =>
	invoke<boolean>('stop_instance', { instanceName });

export const runningInstances = () => invoke<string[]>('running_instances');

export const onGameExit = (handler: (exit: GameExit) => void): Promise<UnlistenFn> =>
	listen<GameExit>('hexo://game-exit', (event) => handler(event.payload));

// ---------- Microsoft 登入 ----------

export const msLoginStart = () => invoke<DeviceCode>('ms_login_start');

/** 會一直等到使用者在瀏覽器完成授權才 resolve */
export const msLoginPoll = (sessionId: string) => invoke<Account>('ms_login_poll', { sessionId });

export const msLoginCancel = (sessionId: string) => invoke<void>('ms_login_cancel', { sessionId });

export const msLoginRefresh = (refresh: string) => invoke<Account>('ms_login_refresh', { refresh });

// ---------- 模組 ----------

export const listMods = (baseDir: string, instanceName: string) =>
	invoke<ModEntry[]>('list_mods', { baseDir, instanceName });

export const checkModUpdates = (args: {
	baseDir: string;
	instanceName: string;
	mcVersion?: string | null;
	curseforgeApiKey?: string | null;
	modIdMap?: Record<string, number> | null;
}) => invoke<ModUpdateEntry[]>('check_mod_updates', args);

export const applyModUpdate = (baseDir: string, instanceName: string, updateId: string) =>
	invoke<void>('apply_mod_update', { baseDir, instanceName, updateId });

export const deleteMod = (baseDir: string, instanceName: string, fileName: string) =>
	invoke<void>('delete_mod', { baseDir, instanceName, fileName });

// ---------- 資源包 / 光影 / 存檔 ----------

export const listResourcePacks = (baseDir: string, instanceName: string) =>
	invoke<PackEntry[]>('list_resource_packs', { baseDir, instanceName });

export const listShaderPacks = (baseDir: string, instanceName: string) =>
	invoke<PackEntry[]>('list_shader_packs', { baseDir, instanceName });

export const listWorlds = (baseDir: string, instanceName: string) =>
	invoke<WorldEntry[]>('list_worlds', { baseDir, instanceName });
