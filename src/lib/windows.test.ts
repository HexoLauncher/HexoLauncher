import { describe, expect, it } from 'vitest';
import { decodeRef, encodeRef, windowSpecs, windowUrl } from './windows';

describe('windowUrl', () => {
	it('在 tauri dev（SvelteKit dev server）下指向根路徑，不是 /index.html', () => {
		// dev server 對 /index.html 會回 404，二級視窗因此開不起來
		expect(windowUrl('/instance/survival', 'http://localhost:5173/')).toBe(
			'http://localhost:5173/#/instance/survival'
		);
	});

	it('打包後從 index.html 開，也維持同一份文件', () => {
		expect(windowUrl('/launch/newage', 'https://tauri.localhost/index.html')).toBe(
			'https://tauri.localhost/index.html#/launch/newage'
		);
	});

	it('主視窗已經在別的路由時，只換掉 hash', () => {
		expect(windowUrl('/about', 'http://localhost:5173/#/settings/accounts')).toBe(
			'http://localhost:5173/#/about'
		);
	});

	it('不會把主視窗的 query 帶進二級視窗', () => {
		expect(windowUrl('/new-instance', 'http://localhost:5173/?foo=1#/settings')).toBe(
			'http://localhost:5173/#/new-instance'
		);
	});
});

describe('windowSpecs', () => {
	it('編輯與啟動進度用路由參數帶 id', () => {
		expect(windowSpecs.instance('survival', '我的生存世界').path).toBe('/instance/survival');
		expect(windowSpecs.launch('newage', 'New Age Science').path).toBe('/launch/newage');
	});

	it('id 會被跳脫，不會破壞路由', () => {
		expect(windowSpecs.instance('a/b', 'x').path).toBe('/instance/a%2Fb');
	});
});

describe('viewer', () => {
	it('本機路徑轉成單一路由分段，可以原樣還原', () => {
		const path = 'C:\\Users\\玩家\\.minecraft/schematics/農場 (2).litematic';
		const spec = windowSpecs.viewer({ kind: 'local', path, instance: 'survival' }, '農場');
		const [, , kind, ref, instance] = spec.path.split('/');

		expect(kind).toBe('local');
		expect(ref).toMatch(/^[A-Za-z0-9_-]+$/);
		expect(decodeRef(ref)).toBe(path);
		expect(instance).toBe('survival');
	});

	it('藍圖庫的藍圖直接用 id', () => {
		const id = '0b6f6a1e-2c2f-4a57-9b7e-3f7c1d2a9e10';
		expect(windowSpecs.viewer({ kind: 'remote', id }, 'x').path).toBe(`/viewer/remote/${id}`);
	});

	it('encodeRef 與 decodeRef 互為反函數', () => {
		for (const value of ['', 'a', 'ab', 'abc', '/tmp/x?y#z', '中文 🧱']) {
			expect(decodeRef(encodeRef(value))).toBe(value);
		}
	});
});
