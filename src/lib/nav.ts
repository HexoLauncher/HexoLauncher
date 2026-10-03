import { goto } from '$app/navigation';

export const routes = {
	instances: '/',
	settings: '/settings',
	blueprints: '/blueprints'
};

export function navigate(path: string) {
	return goto(`#${path}`);
}

export function currentRoute(url: URL) {
	const raw = url.hash.replace(/^#/, '').split('?')[0];
	return raw || '/';
}
