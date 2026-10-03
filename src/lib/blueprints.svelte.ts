import * as api from './api';
import type { Blueprint, BlueprintSession, BlueprintTag, InstanceView } from './api';

export const hub = $state({
	session: null as BlueprintSession | null,
	tags: [] as BlueprintTag[],
	/** 連不上藍圖庫時的訊息。 */
	offline: '',
	signingIn: false
});

export async function refreshSession() {
	try {
		hub.session = await api.blueprintSession();
	} catch (e) {
		hub.offline = api.errorMessage(e);
	}
}

export async function refreshTags() {
	try {
		hub.tags = await api.blueprintTags();
		hub.offline = '';
	} catch (e) {
		hub.offline = api.errorMessage(e);
	}
}

export async function signIn() {
	hub.signingIn = true;
	try {
		hub.session = await api.blueprintLogin();
	} finally {
		hub.signingIn = false;
	}
}

export async function signOut() {
	hub.session = await api.blueprintLogout();
}

/** 伺服器說 token 失效時，同步一下畫面上的登入狀態。 */
export function handleAuthError(error: unknown) {
	if (error && typeof error === 'object' && (error as api.HexoCmdError).kind === 'blueprintAuth') {
		refreshSession();
	}
}

export function tagName(slug: string): string {
	return hub.tags.find((t) => t.slug === slug)?.name ?? slug;
}

export function isOwner(blueprint: Blueprint | null): boolean {
	const me = hub.session?.user?.uuid;
	if (!me || !blueprint) return false;
	const plain = (uuid: string) => uuid.replace(/-/g, '').toLowerCase();
	return plain(me) === plain(blueprint.owner.uuid);
}

const RELEASE = /^\d+\.\d+(\.\d+)?$/;

/** 藍圖庫只接受正式版版本號。 */
export function isRelease(version: string): boolean {
	return RELEASE.test(version);
}

export function compareVersions(a: string, b: string): number {
	const pa = a.split('.').map(Number);
	const pb = b.split('.').map(Number);
	for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
		const diff = (pa[i] ?? 0) - (pb[i] ?? 0);
		if (diff !== 0) return diff;
	}
	return 0;
}

/** 實例用到的正式版版本，新到舊。 */
export function instanceVersions(instances: InstanceView[]): string[] {
	return instances
		.map((i) => i.versionId)
		.filter((v, i, all) => isRelease(v) && all.indexOf(v) === i)
		.sort((a, b) => compareVersions(b, a));
}

/** 藍圖支援 `version` 嗎？min 未知時回傳 null。 */
export function supports(blueprint: Blueprint, version: string): boolean | null {
	const { min, max } = blueprint.mcVersions;
	if (!isRelease(version)) return null;
	if (max && compareVersions(version, max) > 0) return false;
	if (!min) return null;
	return compareVersions(version, min) >= 0;
}

export function versionRange(blueprint: Blueprint): string {
	const { min, max } = blueprint.mcVersions;
	if (!min && !max) return '';
	if (min && max) return min === max ? min : `${min} – ${max}`;
	if (min) return `${min}+`;
	return `≤ ${max}`;
}
