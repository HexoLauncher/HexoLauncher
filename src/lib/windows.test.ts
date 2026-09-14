import { describe, expect, it } from 'vitest';
import { windowSpecs, windowUrl } from './windows';

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
