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
	newInstance: (): WindowSpec => ({
		path: '/new-instance',
		title: m.window_new_instance(),
		width: 800,
		height: 620
	}),
	about: (): WindowSpec => ({
		path: '/about',
		title: m.window_about(),
		width: 460,
		height: 400
	})
};

function inTauri() {
	return typeof window !== 'undefined' && '__TAURI_IPC__' in window;
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
		const { WebviewWindow } = await import('@tauri-apps/api/window');

		const existing = WebviewWindow.getByLabel(label);
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
			focus: true
		});
		return;
	}

	window.open(url, label, `popup=yes,width=${spec.width},height=${spec.height}`);
}

export async function closeSelf() {
	if (inTauri()) {
		const { appWindow } = await import('@tauri-apps/api/window');
		await appWindow.close();
		return;
	}
	window.close();
}
