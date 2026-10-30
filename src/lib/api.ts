import { invoke } from '@tauri-apps/api/core';
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { RichTextFormat } from '$lib/richtext';
import type { ThemeConfig } from '$lib/theme';

export type LoaderKind = 'vanilla' | 'fabric' | 'forge' | 'neoforge';

export interface NolauncherCmdError {
	kind: string;
	message: string;
}

export function errorMessage(error: unknown): string {
	if (error && typeof error === 'object' && 'message' in error) {
		return String((error as NolauncherCmdError).message);
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
	theme: ThemeConfig;
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

export const setBackgroundImage = (source: string) =>
	invoke<string>('set_background_image', { source });

export const backgroundImagePath = (name: string) =>
	invoke<string>('background_image_path', { name });

export const clearBackgroundImage = () => invoke<void>('clear_background_image');

export const emitThemeChanged = (theme: ThemeConfig) => emit('nolauncher://theme-changed', theme);

export const onThemeChanged = (handler: (theme: ThemeConfig) => void): Promise<UnlistenFn> =>
	listen<ThemeConfig>('nolauncher://theme-changed', (event) => handler(event.payload));

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
	iconImage: string | null;

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
	| 'config'
	| 'schematics';

export const readImageFile = (path: string) => invoke<string>('read_image_file', { path });

export const setInstanceIconImage = (baseDir: string, instanceName: string, path: string) =>
	invoke<string>('set_instance_icon_image', { baseDir, instanceName, path });

export const playerSkin = (uuid: string) => invoke<string | null>('player_skin', { uuid });

export const openInstanceFolder = (
	baseDir: string,
	instanceName: string,
	target: InstanceFolder = 'minecraft'
) => invoke<void>('open_instance_folder', { baseDir, instanceName, target });

export interface CreateRequest {
	baseDir: string;

	name: string;
	icon?: string | null;
	iconPath?: string | null;
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

export type ImportMode = 'copy' | 'link';

export interface ForeignLauncher {
	kind: string;
	name: string;
	path: string;
	instanceCount: number;
}

export interface ForeignInstance {
	path: string;
	name: string;
	notes: string;
	mcVersion: string;
	loader: LoaderKind;
	loaderVersion: string;
	modCount: number;
	lastPlayed: number;
	playTimeSeconds: number;
	/** 我們不支援的模組載入器，匯入後要自己換掉。 */
	unsupportedLoader: string | null;
}

export interface ForeignImportRequest {
	baseDir: string;
	paths: string[];
	mode: ImportMode;
	group?: string | null;
}

export interface ForeignImportFailure {
	path: string;
	name: string;
	message: string;
}

export interface ForeignImportResult {
	imported: InstanceView[];
	failed: ForeignImportFailure[];
}

export const detectForeignLaunchers = () => invoke<ForeignLauncher[]>('detect_foreign_launchers');

export const listForeignInstances = (path: string) =>
	invoke<ForeignInstance[]>('list_foreign_instances', { path });

export const importForeignInstances = (request: ForeignImportRequest) =>
	invoke<ForeignImportResult>('import_foreign_instances', { request });

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
	iconPath?: string | null;
	iconUrl?: string | null;
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

export interface ModpackUpdateResult extends ModpackInstallResult {
	/** 舊版模組包帶進來、這次更新之後被刪掉的檔案（相對 .minecraft）。 */
	removedFiles: string[];
}

export const installModpackFile = (target: PackTarget, packPath: string) =>
	invoke<ModpackInstallResult>('install_modpack_file', { target, packPath });

export const updateModpackFile = (baseDir: string, instanceName: string, packPath: string) =>
	invoke<ModpackUpdateResult>('update_modpack_file', { baseDir, instanceName, packPath });

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

export type ReleaseType = 'release' | 'beta' | 'alpha' | 'archived';

export interface RemoteVersion {
	id: string;
	name: string;
	mcVersion: string;
	gameVersions: string[];
	loaders: string[];
	releaseType: ReleaseType;

	published: number;
	fileName: string;
	/** false = 作者不允許第三方下載 */
	downloadable: boolean;
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

export type ContentVersion = RemoteVersion;

export interface InstalledContent {
	fileName: string;
	path: string;
}

export interface ModInstallPlan {
	id: string;
	items: {
		name: string;
		fileName: string;
		version: string;
		dependency: boolean;
		installed: boolean;
		conflict: string | null;
	}[];
}

export const prepareModInstall = (args: {
	baseDir: string;
	instanceName: string;
	platform: ContentPlatform;
	projectId: string;
	versionId: string;
	includeOptional: boolean;
}) => invoke<ModInstallPlan>('prepare_mod_install', args);

export const commitModInstall = (planId: string) =>
	invoke<InstalledContent>('commit_mod_install', { planId });

export const discardModInstall = (planId: string) =>
	invoke<void>('discard_mod_install', { planId });

export interface GalleryImage {
	url: string;
	title: string;
}

export interface ContentLink {
	kind: 'website' | 'source' | 'issues' | 'wiki' | 'discord';
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

export const getModpackDetail = (platform: ModpackPlatform, packId: string) =>
	invoke<ContentDetail>('get_modpack_detail', { platform, packId });

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

export interface ModDependency {
	modId: string;
	version: string | null;
	kind: 'required' | 'optional' | 'incompatible';
}

export interface ModDetails extends ModEntry {
	size: number;
	modified: number;
	authors: string[];
	license: string | null;
	homepage: string | null;
	sources: string | null;
	issues: string | null;
	/** data URL */
	icon: string | null;
	dependencies: ModDependency[];
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
	icon: string | null;
	name: string;
	path: string;
	fileName: string;
	isFolder: boolean;
	size: number;
	modified: number;
	active: boolean;
}

export type WorldGameMode = 'survival' | 'creative' | 'adventure' | 'spectator' | 'unknown';

export interface WorldEntry {
	name: string;
	folder: string;
	path: string;

	icon: string | null;
	gameMode: WorldGameMode | null;
	hardcore: boolean;
	cheats: boolean;
	version: string | null;
	lastPlayed: number;
	size: number;
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

export const modDetails = (baseDir: string, instanceName: string, fileName: string) =>
	invoke<ModDetails>('mod_details', { baseDir, instanceName, fileName });

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

export const setActiveShader = (baseDir: string, instanceName: string, fileName: string | null) =>
	invoke<void>('set_active_shader', { baseDir, instanceName, fileName });

export const deletePack = (
	baseDir: string,
	instanceName: string,
	kind: 'resourcepacks' | 'shaderpacks',
	fileName: string
) => invoke<void>('delete_pack', { baseDir, instanceName, kind, fileName });

export const listWorlds = (baseDir: string, instanceName: string) =>
	invoke<WorldEntry[]>('list_worlds', { baseDir, instanceName });

export const openWorldFolder = (baseDir: string, instanceName: string, folder: string) =>
	invoke<void>('open_world_folder', { baseDir, instanceName, folder });

export const deleteWorld = (baseDir: string, instanceName: string, folder: string) =>
	invoke<void>('delete_world', { baseDir, instanceName, folder });

export const listServers = (baseDir: string, instanceName: string) =>
	invoke<ServerEntry[]>('list_servers', { baseDir, instanceName });

export const listScreenshots = (baseDir: string, instanceName: string) =>
	invoke<FileEntry[]>('list_screenshots', { baseDir, instanceName });

export const listInstanceLogs = (baseDir: string, instanceName: string) =>
	invoke<FileEntry[]>('list_instance_logs', { baseDir, instanceName });

export const readInstanceLog = (baseDir: string, instanceName: string, fileName: string) =>
	invoke<string>('read_instance_log', { baseDir, instanceName, fileName });

export interface BlueprintUser {
	uuid: string;
	username: string;
	createdAt: string;
}

export interface BlueprintSession {
	server: string;
	/** 啟動器目前選的帳號；online = Microsoft 帳號，才能登入藍圖庫。 */
	account: { id: string; playerName: string; online: boolean } | null;
	user: BlueprintUser | null;
}

export interface BlueprintTag {
	slug: string;
	name: string;
}

export type RenderStatus = 'pending' | 'rendering' | 'done' | 'failed' | 'unsupported';

export interface BlueprintImage {
	id: string;
	url: string;
	width: number;
	height: number;
}

export interface Blueprint {
	id: string;
	owner: { uuid: string; username: string };
	name: string;
	description: string;
	format: 'litematic' | 'schem';
	tags: string[];
	/** min = null 表示未知；max = null 表示沒有上限 */
	mcVersions: { min: string | null; max: string | null };
	dataVersion: number | null;
	/** [x, y, z] */
	size: [number, number, number] | null;
	totalBlocks: number | null;
	file: { name: string; size: number; sha256: string };
	cover: { url: string | null; source: 'image' | 'render' | null; imageId: string | null };
	render: {
		status: RenderStatus;
		url: string | null;
		error: string | null;
		mcVersion: string | null;
	};
	images: BlueprintImage[];
	downloadCount: number;
	createdAt: string;
	updatedAt: string;
}

export interface BlueprintList {
	items: Blueprint[];
	page: number;
	perPage: number;
	total: number;
}

export interface BlueprintQuery {
	q?: string;
	owner?: string | null;
	tags?: string[];
	version?: string | null;
	page?: number;
	perPage?: number;
}

export interface InstalledBlueprint {
	fileName: string;
	path: string;
	existed: boolean;
}

export interface BlueprintUpload {
	path: string;
	name: string;
	description: string;
	tags: string[];
	mcVersionMin: string;
	mcVersionMax: string;
}

/** 整份送出；版本欄位空字串代表清除。 */
export interface BlueprintPatch {
	name: string;
	description: string;
	tags: string[];
	mcVersionMin: string;
	mcVersionMax: string;
}

export interface BlueprintTerms {
	version: string;
	content: string;
	accepted: boolean;
}

export const blueprintSession = () => invoke<BlueprintSession>('blueprint_session');

export const blueprintTerms = () => invoke<BlueprintTerms | null>('blueprint_terms');

export const blueprintLogin = (termsVersion?: string) =>
	invoke<BlueprintSession>('blueprint_login', { termsVersion: termsVersion ?? null });

export const blueprintLogout = () => invoke<BlueprintSession>('blueprint_logout');

export const blueprintTags = () => invoke<BlueprintTag[]>('blueprint_tags');

export const blueprintSearch = (query: BlueprintQuery) =>
	invoke<BlueprintList>('blueprint_search', { query });

export const blueprintGet = (id: string) => invoke<Blueprint>('blueprint_get', { id });

export const blueprintInstall = (baseDir: string, instanceName: string, id: string) =>
	invoke<InstalledBlueprint>('blueprint_install', { baseDir, instanceName, id });

export const blueprintSaveAs = (id: string, targetPath: string) =>
	invoke<string>('blueprint_save_as', { id, targetPath });

/** 3D 預覽用的方塊資料（base64），交給 `App.load_preview`。 */
export const blueprintPreview = (id: string, format: string) =>
	invoke<string>('blueprint_preview', { id, format });

export const blueprintUpload = (request: BlueprintUpload) =>
	invoke<Blueprint>('blueprint_upload', { request });

export const blueprintUpdate = (id: string, patch: BlueprintPatch) =>
	invoke<Blueprint>('blueprint_update', { id, patch });

export const blueprintDelete = (id: string) => invoke<void>('blueprint_delete', { id });

export const blueprintRerender = (id: string) => invoke<Blueprint>('blueprint_rerender', { id });

export const blueprintAddImages = (id: string, paths: string[]) =>
	invoke<Blueprint>('blueprint_add_images', { id, paths });

export const blueprintDeleteImage = (id: string, imageId: string) =>
	invoke<Blueprint>('blueprint_delete_image', { id, imageId });

export const blueprintSetCover = (id: string, imageId: string | null) =>
	invoke<Blueprint>('blueprint_set_cover', { id, imageId });

export interface SchematicFile {
	name: string;
	/** 相對 schematics/，用 / 分隔 */
	relPath: string;
	path: string;
	format: 'litematic' | 'schem';
	size: number;
	modified: number;
}

export const listSchematics = (baseDir: string, instanceName: string) =>
	invoke<SchematicFile[]>('list_schematics', { baseDir, instanceName });

export const deleteSchematic = (baseDir: string, instanceName: string, relPath: string) =>
	invoke<void>('delete_schematic', { baseDir, instanceName, relPath });

export const importSchematic = (baseDir: string, instanceName: string, source: string) =>
	invoke<SchematicFile>('import_schematic', { baseDir, instanceName, source });

/** 本機 .litematic 內容（base64），給 3D 預覽用。 */
export const readSchematic = (path: string) => invoke<string>('read_schematic', { path });

/** 從本機已下載的 client jar 抽出方塊材質；沒有就回傳 null。 */
export const localBlockAssets = (baseDir: string, version: string, prefer?: string | null) =>
	invoke<string | null>('local_block_assets', { baseDir, version, prefer });

export const packBlockAssets = (path: string) => invoke<string>('pack_block_assets', { path });

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
	listen<InstallProgress>('nolauncher://install-progress', (event) => handler(event.payload));

export const onGameStart = (handler: (info: RunningGame) => void): Promise<UnlistenFn> =>
	listen<RunningGame>('nolauncher://game-start', (event) => handler(event.payload));

export const onGameExit = (handler: (exit: GameExit) => void): Promise<UnlistenFn> =>
	listen<GameExit>('nolauncher://game-exit', (event) => handler(event.payload));

export const onGameOutput = (handler: (output: GameOutput) => void): Promise<UnlistenFn> =>
	listen<GameOutput>('nolauncher://game-output', (event) => handler(event.payload));

export const onInstancesChanged = (handler: () => void): Promise<UnlistenFn> =>
	listen('nolauncher://instances-changed', () => handler());

export interface JavaProgress {
	version: number;

	done: number;

	total: number;

	message: string;
}

export const onJavaProgress = (handler: (progress: JavaProgress) => void): Promise<UnlistenFn> =>
	listen<JavaProgress>('nolauncher://java-progress', (event) => handler(event.payload));
