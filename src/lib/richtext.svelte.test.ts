import { describe, expect, it } from 'vitest';
import { renderRichText } from './richtext';

describe('renderRichText', () => {
	it('把 Markdown 轉成 HTML', () => {
		const html = renderRichText('# 標題\n\n看 [說明](https://example.com) 就好。', 'markdown');
		expect(html).toContain('<h1>標題</h1>');
		expect(html).toContain('<a href="https://example.com">說明</a>');
	});

	it('HTML 原樣保留結構', () => {
		expect(renderRichText('<p>Fast &amp; <b>free</b>.</p>', 'html')).toBe(
			'<p>Fast &amp; <b>free</b>.</p>'
		);
	});

	it('拆掉腳本與事件屬性', () => {
		const html = renderRichText(
			'<p onclick="evil()">嗨</p><script>evil()</script><img src=x onerror="evil()">',
			'html'
		);
		expect(html).not.toContain('script');
		expect(html).not.toContain('onclick');
		expect(html).not.toContain('onerror');
		expect(html).toContain('<p>嗨</p>');
	});

	it('擋掉 javascript: 與 data: 連結', () => {
		expect(renderRichText('[點我](javascript:evil())', 'markdown')).not.toContain('javascript:');
		expect(renderRichText('<iframe src="data:text/html,x"></iframe>', 'html')).toBe('');
	});

	it('Markdown 裡夾的 HTML 一樣要過濾', () => {
		const html = renderRichText('說明\n\n<style>body{display:none}</style>', 'markdown');
		expect(html).not.toContain('style');
	});
});
