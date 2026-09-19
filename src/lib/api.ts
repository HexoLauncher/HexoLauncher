import { invoke } from '@tauri-apps/api/tauri';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { RichTextFormat } from '$lib/richtext';

export type LoaderKind = 'vanilla' | 'fabric' | 'forge' | 'neoforge';

export interface HexoCmdError {
	kind: string;
	message: string;
}

export function errorMessage(error: unknown): string {
	if (error && typeof error === 'object' && 'message' in error) {
		return String((error as HexoCmdError).message);
	}
	return String(error);
}

export interface UiConfig {
	instanceToolbar: boolean;
	statusBar: boolean;
	runningBadge: boolean;
	includeSnapshots: boolean;
}

export interface GameConfig {
	minMemoryMb: number;
	maxMemoryMb: number;
	jvmArgs: string;
	recordPlayTime: boolean;

	showProgressWindow: boolean;

	closeProgressWhenDone: boolean;

	keepLogLines: number;
}

export interface JavaConfig {
	path: string;
}

export interface CommandsConfig {
	preLaunch: string;
	postExit: string;
}

export interface ApiConfig {
	curseforgeApiKey: string;
}

export interface LauncherConfig {
	formatVersion: number;

	baseDir: string;
	ui: UiConfig;
	game: GameConfig;
	java: JavaConfig;
	commands: CommandsConfig;
	api: ApiConfig;
}

export interface Paths {
	baseDir: string;
	defaultBaseDir: string;
	storeDir: string;
}

export const resolvePaths = () => invoke<Paths>('resolve_paths');

export const getLauncherConfig = () => invoke<LauncherConfig>('get_launcher_config');

export const saveLauncherConfig = (config: LauncherConfig) =>
	invoke<LauncherConfig>('save_launcher_config', { config });

export const prepareBaseDir = (baseDir: string) => invoke<void>('prepare_base_dir', { baseDir });

export type LauncherFolder = 'base' | 'instances' | 'libraries' | 'assets' | 'java' | 'store';

export const openLauncherFolder = (target: LauncherFolder) =>
	invoke<void>('open_launcher_folder', { target });

export const openExternal = (url: string) => invoke<void>('open_external', { url });

export type AccountKind = 'microsoft' | 'offline';

export interface AccountView {
	id: string;
	kind: AccountKind;
	playerName: string;
	uuid: string;

	expiresAt: number;
	expired: boolean;
	selected: boolean;
}

export const listAccounts = () => invoke<AccountView[]>('list_accounts');

export const addOfflineAccount = (playerName: string) =>
	invoke<AccountView[]>('add_offline_account', { playerName });

export const removeAccount = (accountId: string) =>
	invoke<AccountView[]>('remove_account', { accountId });

export const selectAccount = (accountId: string) =>
	invoke<AccountView[]>('select_account', { accountId });

export const refreshAccount = (accountId: string) =>
	invoke<AccountView[]>('refresh_account', { accountId });

export interface DeviceCode {
	sessionId: string;
	userCode: string;
	verificationUri: string;
	message: string;
	expiresIn: number;
}

export const msLoginStart = () => invoke<DeviceCode>('ms_login_start');

export const msLoginPoll = (sessionId: string) =>
	invoke<AccountView[]>('ms_login_poll', { sessionId });

export const msLoginCancel = (sessionId: string) => invoke<void>('ms_login_cancel', { sessionId });

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

export const listVersions = (includeSnapshots = false) =>
	invoke<VersionList>('list_versions', { includeSnapshots });

export const listLoaderVersions = (loader: LoaderKind, mcVersion: string) =>
	invoke<string[]>('list_loader_versions', { loader, mcVersion });

export interface InstanceSettings {
	overrideMemory: boolean;
	minMemoryMb: number;
	maxMemoryMb: number;
	overrideJvmArgs: boolean;
	jvmArgs: string;
	overrideCommands: boolean;
	preLaunch: string;
	postExit: string;
}

export interface InstanceView {
	id: string;
	name: string;
	icon: string;

	group: string;
	notes: string;
	versionId: string;
	loader: LoaderKind;
	loaderVersion: string;
	javaVersion: number;
	assetsId: string;
	mainClass: string;
	dir: string;
	minecraftDir: string;
	modCount: number;

	lastPlayed: number;
	playTimeSeconds: number;
	launchCount: number;
	installed: boolean;

	healthy: boolean;
	javaPath: string;
	settings: InstanceSettings;
}

export interface MetaPatch {
	name?: string;
	icon?: string;
	group?: string;
	notes?: string;
	settings?: InstanceSettings;

	javaPath?: string;
}

export const listInstances = (baseDir: string) =>
	invoke<InstanceView[]>('list_instances', { baseDir });

export const getInstance = (baseDir: string, instanceName: string) =>
	invoke<InstanceView>('get_instance', { baseDir, instanceName });

export const saveInstanceMeta = (baseDir: string, instanceName: string, patch: MetaPatch) =>
	invoke<InstanceView>('save_instance_meta', { baseDir, instanceName, patch });

export const deleteInstance = (baseDir: string, instanceName: string) =>
	invoke<void>('delete_instance', { baseDir, instanceName });

export const duplicateInstance = (baseDir: string, instanceName: string, newName: string) =>
	invoke<InstanceView>('duplicate_instance', { baseDir, instanceName, newName });

export const suggestInstanceId = (baseDir: string, name: string) =>
	invoke<string>('suggest_instance_id', { baseDir, name });

export type InstanceFolder =
	| 'instance'
	| 'minecraft'
	| 'mods'
	| 'resourcepacks'
	| 'shaderpacks'
	| 'saves'
	| 'screenshots'
	| 'logs'
	| 'config';

export const openInstanceFolder = (
	baseDir: string,
	instanceName: string,
	target: InstanceFolder = 'minecraft'
) => invoke<void>('open_instance_folder', { baseDir, instanceName, target });

export interface CreateRequest {
	baseDir: string;

	name: string;
	icon?: string | null;
	group?: string | null;
	mcVersion: string;
	loader: LoaderKind;
	loaderVersion?: string | null;
}

export const createInstance = (request: CreateRequest) =>
	invoke<InstanceView>('create_instance', { request });

export interface ExportRequest {
	baseDir: string;
	instanceName: string;

	targetPath: string;
	includeSaves: boolean;
	includeScreenshots: boolean;
}

export interface ExportResult {
	path: string;
	fileCount: number;
	bytes: number;
}

export const exportInstance = (request: ExportRequest) =>
	invoke<ExportResult>('export_instance', { request });

export const importInstance = (baseDir: string, zipPath: string, name?: string | null) =>
	invoke<InstanceView>('import_instance', { baseDir, zipPath, name });

export type ModpackFormat = 'modrinth' | 'curseforge' | 'atlauncher' | 'ftb';

export interface ModpackPreview {
	format: ModpackFormat;
	name: string;
	version: string | null;
	mcVersion: string;
	loader: LoaderKind;

	loaderVersion: string | null;
	fileCount: number;
}

export interface PackTarget {
	baseDir: string;

	name?: string | null;
	icon?: string | null;
	group?: string | null;
}

export interface ManualDownload {
	name: string;
	fileName: string;
	website: string | null;

	dest: string;
	folder: InstanceFolder;
}

export interface ModpackInstallResult {
	instance: InstanceView;
	manualDownloads: ManualDownload[];
	skipped: string[];
}

export const inspectModpack = (packPath: string) =>
	invoke<ModpackPreview>('inspect_modpack', { packPath });

export const installModpackFile = (target: PackTarget, packPath: string) =>
	invoke<ModpackInstallResult>('install_modpack_file', { target, packPath });

export type ModpackPlatform = 'modrinth' | 'curseforge' | 'atlauncher' | 'ftb';

export type ModpackSort = 'relevance' | 'downloads' | 'updated' | 'name';

export interface ModpackSearchRequest {
	platform: ModpackPlatform;
	query: string;
	sort: ModpackSort;
	mcVersion?: string | null;
	offset: number;
}

export interface PackHit {
	id: string;
	name: string;
	author: string;
	summary: string;
	iconUrl: string | null;

	downloads: number;
	website: string | null;
}

export interface SearchPage {
	hits: PackHit[];
	total: number;
	nextOffset: number | null;
}

export interface RemoteVersion {
	id: string;
	name: string;
	mcVersion: string;

	kind: string;

	published: number;
}

export const searchModpacks = (request: ModpackSearchRequest) =>
	invoke<SearchPage>('search_modpacks', { request });

export const listModpackVersions = (platform: ModpackPlatform, packId: string) =>
	invoke<RemoteVersion[]>('list_modpack_versions', { platform, packId });

export const installRemoteModpack = (
	target: PackTarget,
	platform: ModpackPlatform,
	packId: string,
	versionId: string,
	includeOptional: boolean
) =>
	invoke<ModpackInstallResult>('install_remote_modpack', {
		target,
		platform,
		packId,
		versionId,
		includeOptional
	});

export type ContentKind = 'mod' | 'resourcepack' | 'shader';

export type ContentPlatform = 'modrinth' | 'curseforge';

export interface ContentSearchRequest {
	platform: ContentPlatform;
	kind: ContentKind;
	query: string;
	sort: ModpackSort;
	mcVersion?: string | null;
	loader?: LoaderKind | null;
	offset: number;
}

export interface ContentVersion {
	id: string;
	name: string;
	mcVersion: string;

	kind: string;

	published: number;
	fileName: string;
	loaders: string[];
	/** false = 作者不允許第三方下載 */
	downloadable: boolean;
}

export interface InstalledContent {
	fileName: string;
	path: string;
}

export interface GalleryImage {
	url: string;
	title: string;
}

export interface ContentLink {
	label: string;
	url: string;
}

export interface ContentDetail {
	id: string;
	name: string;
	summary: string;
	description: string;
	descriptionFormat: RichTextFormat;
	iconUrl: string | null;

	downloads: number;
	follows: number;
	categories: string[];
	gameVersions: string[];
	loaders: string[];
	license: string | null;

	updated: number;
	gallery: GalleryImage[];
	links: ContentLink[];
}

export const getContentDetail = (args: {
	platform: ContentPlatform;
	kind: ContentKind;
	projectId: string;
}) => invoke<ContentDetail>('get_content_detail', args);

export const searchContent = (request: ContentSearchRequest) =>
	invoke<SearchPage>('search_content', { request });

export const listContentVersions = (args: {
	platform: ContentPlatform;
	kind: ContentKind;
	projectId: string;
	mcVersion?: string | null;
	loader?: LoaderKind | null;
}) => invoke<ContentVersion[]>('list_content_versions', args);

export const installContent = (args: {
	baseDir: string;
	instanceName: string;
	platform: ContentPlatform;
	kind: ContentKind;
	projectId: string;
	versionId: string;
}) => invoke<InstalledContent>('install_content', args);

export interface InstallRequest {
	baseDir: string;

	instanceName: string;

	name?: string | null;
	icon?: string | null;
	group?: string | null;
	mcVersion: string;
	loader: LoaderKind;
	loaderVersion?: string | null;
	javaPath?: string | null;
}

export const installInstance = (request: InstallRequest) =>
	invoke<InstanceView>('install_instance', { request });

export const repairInstance = (baseDir: string, instanceName: string) =>
	invoke<InstanceView>('repair_instance', { baseDir, instanceName });

export interface JavaInfo {
	path: string;
	version: number;
}

export const detectJava = (requiredVersion = 21) =>
	invoke<JavaInfo | null>('detect_java', { requiredVersion });

export interface JavaCandidate extends JavaInfo {
	fullVersion: string;
	vendor: string;

	arch: string;
}

export const detectJavaCandidates = () => invoke<JavaCandidate[]>('detect_java_candidates');

export const testJava = (path: string) => invoke<JavaInfo>('test_java', { path });

export const installJava = (baseDir: string, version: number) =>
	invoke<JavaInfo>('install_java', { baseDir, version });

export const ensureJava = (baseDir: string, requiredVersion: number, allowDownload = false) =>
	invoke<JavaInfo>('ensure_java', { baseDir, requiredVersion, allowDownload });

export interface JavaCheck {
	required: number;

	java: JavaInfo | null;
}

export const checkInstanceJava = (baseDir: string, instanceName: string) =>
	invoke<JavaCheck>('check_instance_java', { baseDir, instanceName });

export interface LaunchRequest {
	baseDir: string;
	instanceName: string;

	accountId?: string | null;
}

export interface LaunchInfo {
	instanceName: string;
	pid: number;
	javaPath: string;
	javaVersion: number;
	playerName: string;
	online: boolean;
}

export interface RunningGame {
	instanceName: string;
	pid: number;

	startedAt: number;
	playerName: string;
	online: boolean;
}

export interface GameLogLine {
	seq: number;
	kind: 'stdout' | 'stderr';
	line: string;
}

export const launchInstance = (options: LaunchRequest) =>
	invoke<LaunchInfo>('launch_instance', { options });

export const stopInstance = (instanceName: string) =>
	invoke<boolean>('stop_instance', { instanceName });

export const runningInstances = () => invoke<RunningGame[]>('running_instances');

export const gameLog = (instanceName: string) =>
	invoke<GameLogLine[]>('game_log', { instanceName });

export const clearGameLog = (instanceName: string) =>
	invoke<void>('clear_game_log', { instanceName });

export interface ModEntry {
	fileName: string;
	filePath: string;
	modId: string;
	name: string;
	version: string;
	loader: string;
	description: string | null;
	enabled: boolean;
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

	icon: string | null;
}

export interface ServerEntry {
	name: string;
	address: string;

	icon: string | null;
	acceptTextures: boolean;
}

export interface FileEntry {
	name: string;
	path: string;
	size: number;

	modified: number;
}

export const listMods = (baseDir: string, instanceName: string) =>
	invoke<ModEntry[]>('list_mods', { baseDir, instanceName });

export const setModEnabled = (
	baseDir: string,
	instanceName: string,
	fileName: string,
	enabled: boolean
) => invoke<void>('set_mod_enabled', { baseDir, instanceName, fileName, enabled });

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

export const listResourcePacks = (baseDir: string, instanceName: string) =>
	invoke<PackEntry[]>('list_resource_packs', { baseDir, instanceName });

export const listShaderPacks = (baseDir: string, instanceName: string) =>
	invoke<PackEntry[]>('list_shader_packs', { baseDir, instanceName });

export const listWorlds = (baseDir: string, instanceName: string) =>
	invoke<WorldEntry[]>('list_worlds', { baseDir, instanceName });

export const listServers = (baseDir: string, instanceName: string) =>
	invoke<ServerEntry[]>('list_servers', { baseDir, instanceName });

export const listScreenshots = (baseDir: string, instanceName: string) =>
	invoke<FileEntry[]>('list_screenshots', { baseDir, instanceName });

export const listInstanceLogs = (baseDir: string, instanceName: string) =>
	invoke<FileEntry[]>('list_instance_logs', { baseDir, instanceName });

export const readInstanceLog = (baseDir: string, instanceName: string, fileName: string) =>
	invoke<string>('read_instance_log', { baseDir, instanceName, fileName });

export interface InstallProgress {
	instanceName: string;
	done: number;
	total: number;
	message: string;
}

export interface GameExit {
	instanceName: string;
	exitCode: number | null;
	seconds: number;
}

export interface GameOutput extends GameLogLine {
	instanceName: string;
}

export const onInstallProgress = (
	handler: (progress: InstallProgress) => void
): Promise<UnlistenFn> =>
	listen<InstallProgress>('hexo://install-progress', (event) => handler(event.payload));

export const onGameStart = (handler: (info: RunningGame) => void): Promise<UnlistenFn> =>
	listen<RunningGame>('hexo://game-start', (event) => handler(event.payload));

export const onGameExit = (handler: (exit: GameExit) => void): Promise<UnlistenFn> =>
	listen<GameExit>('hexo://game-exit', (event) => handler(event.payload));

export const onGameOutput = (handler: (output: GameOutput) => void): Promise<UnlistenFn> =>
	listen<GameOutput>('hexo://game-output', (event) => handler(event.payload));

export const onInstancesChanged = (handler: () => void): Promise<UnlistenFn> =>
	listen('hexo://instances-changed', () => handler());

export interface JavaProgress {
	version: number;

	done: number;

	total: number;

	message: string;
}

export const onJavaProgress = (handler: (progress: JavaProgress) => void): Promise<UnlistenFn> =>
	listen<JavaProgress>('hexo://java-progress', (event) => handler(event.payload));
