import { describe, expect, it } from 'vitest';
import { parseMcText, stripMcText } from './mctext';

describe('parseMcText', () => {
	it('沒有格式碼就是一段純文字', () => {
		expect(parseMcText('Hello')).toEqual([{ text: 'Hello' }]);
	});

	it('顏色碼會重設粗體之類的樣式', () => {
		expect(parseMcText('§6§lGold§r plain§cRed')).toEqual([
			{ text: 'Gold', color: '#ffaa00', bold: true },
			{ text: ' plain' },
			{ text: 'Red', color: '#ff5555' }
		]);
	});

	it('白色當預設色，保留換行', () => {
		expect(parseMcText('§fCreate\n§8by me')).toEqual([
			{ text: 'Create\n', color: undefined },
			{ text: 'by me', color: '#555555' }
		]);
	});

	it('結尾落單的 § 原樣保留', () => {
		expect(parseMcText('a§')).toEqual([{ text: 'a§' }]);
	});

	it('stripMcText 只留下文字', () => {
		expect(stripMcText('§6§lGold§r plain')).toBe('Gold plain');
	});
});
