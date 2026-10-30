import * as api from './api';

let releases: Promise<string[]> | null = null;

export function releaseVersions(): Promise<string[]> {
	releases ??= api
		.listVersions(false)
		.then((list) => list.versions.filter((v) => v.versionType === 'release').map((v) => v.id))
		.catch((e) => {
			releases = null;
			throw e;
		});
	return releases;
}

export function withCurrent(versions: string[], ...current: string[]): string[] {
	const extra = current.filter((v) => v && !versions.includes(v));
	return [...extra, ...versions];
}
