import * as api from './api';
import type { Blueprint, BlueprintSession, BlueprintTag, BlueprintTerms, InstanceView } from './api';

export const hub = $state({
	session: null as BlueprintSession | null,
	terms: null as BlueprintTerms | null,
	tags: [] as BlueprintTag[],
	/** 連不上藍圖庫時的訊息。 */
	offline: '',
	signingIn: false
});

let sessionSeq = 0;

export async function refreshSession() {
	const seq = ++sessionSeq;
	try {
		const session = await api.blueprintSession();
		if (seq === sessionSeq) hub.session = session;
	} catch (e) {
		if (seq === sessionSeq) hub.offline = api.errorMessage(e);
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

function isTermsError(error: unknown) {
	return !!error && typeof error === 'object' && (error as api.NolauncherCmdError).kind === 'blueprintTerms';
}

export async function signIn(termsVersion?: string): Promise<boolean> {
	hub.signingIn = true;
	try {
		if (!termsVersion) {
			const terms = await api.blueprintTerms();
			if (terms && !terms.accepted) {
				hub.terms = terms;
				return false;
			}
		}
		const seq = ++sessionSeq;
		let session: api.BlueprintSession;
		try {
			session = await api.blueprintLogin(termsVersion);
		} catch (e) {
			if (!isTermsError(e)) throw e;
			const terms = await api.blueprintTerms();
			if (!terms) throw e;
			hub.terms = { ...terms, accepted: false };
			return false;
		}
		if (seq === sessionSeq) hub.session = session;
		else await refreshSession();
		return true;
	} finally {
		hub.signingIn = false;
	}
}

export async function signOut() {
	const seq = ++sessionSeq;
	const session = await api.blueprintLogout();
	if (seq === sessionSeq) hub.session = session;
}

/** 伺服器說 token 失效時，同步一下畫面上的登入狀態。 */
export function handleAuthError(error: unknown) {
	if (error && typeof error === 'object' && (error as api.NolauncherCmdError).kind === 'blueprintAuth') {
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
