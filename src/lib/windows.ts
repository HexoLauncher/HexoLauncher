import { m } from '$lib/paraglide/messages.js';

export type WindowSpec = {
	path: string;
	title: string;
	width: number;
	height: number;
};

export const windowSpecs = {
	instance: (id: string, name: string): WindowSpec => ({
		path: `/instance/${encodeURIComponent(id)}`,
		title: m.window_edit_instance({ name }),
		width: 1040,
		height: 690
	}),
	launch: (id: string, name: string): WindowSpec => ({
		path: `/launch/${encodeURIComponent(id)}`,
		title: m.window_console({ name }),
		width: 760,
		height: 560
	}),
	crash: (id: string, name: string): WindowSpec => ({
		path: `/crash/${encodeURIComponent(id)}`,
		title: m.window_crash({ name }),
		width: 760,
		height: 600
	}),
	newInstance: (): WindowSpec => ({
		path: '/new-instance',
		title: m.window_new_instance(),
		width: 1040,
		height: 690
	}),

	viewer: (
		source:
			| { kind: 'remote'; id: string }
			| { kind: 'local'; path: string; instance?: string | null },
		name: string
	): WindowSpec => {
		const ref = source.kind === 'remote' ? source.id : encodeRef(source.path);
		const instance =
			source.kind === 'local' && source.instance ? `/${encodeURIComponent(source.instance)}` : '';
		return {
			path: `/viewer/${source.kind}/${ref}${instance}`,
			title: m.window_viewer({ name }),
			width: 1100,
			height: 720
		};
	},
	about: (): WindowSpec => ({
		path: '/about',
		title: m.window_about(),
		width: 460,
		height: 400
	})
};

export function encodeRef(value: string) {
	const bytes = new TextEncoder().encode(value);
	let binary = '';
	bytes.forEach((b) => (binary += String.fromCharCode(b)));
	return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

export function decodeRef(ref: string) {
	const base64 = ref.replace(/-/g, '+').replace(/_/g, '/');
	const binary = atob(base64 + '='.repeat((4 - (base64.length % 4)) % 4));
	return new TextDecoder().decode(Uint8Array.from(binary, (c) => c.charCodeAt(0)));
}

function inTauri() {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

function labelFor(path: string) {
	return path.replace(/[^a-zA-Z0-9\-/:_]/g, '_');
}

export function windowUrl(path: string, href = window.location.href) {
	const url = new URL(href);
	url.search = '';
	url.hash = path;
	return url.href;
}

export async function openAppWindow(spec: WindowSpec) {
	const url = windowUrl(spec.path);
	const label = labelFor(spec.path);

	if (inTauri()) {
		const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');

		const existing = await WebviewWindow.getByLabel(label);
		if (existing) {
			await existing.unminimize();
			await existing.setFocus();
			return;
		}

		new WebviewWindow(label, {
			url,
			title: spec.title,
			width: spec.width,
			height: spec.height,
			resizable: true,
			focus: true,
			useHttpsScheme: true
		});
		return;
	}

	window.open(url, label, `popup=yes,width=${spec.width},height=${spec.height}`);
}

export async function closeSelf() {
	if (inTauri()) {
		const { getCurrentWindow } = await import('@tauri-apps/api/window');
		await getCurrentWindow().close();
		return;
	}
	window.close();
}
