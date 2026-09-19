import DOMPurify from 'dompurify';
import { marked } from 'marked';

export type RichTextFormat = 'markdown' | 'html';

const ALLOWED_TAGS = [
	'p',
	'br',
	'hr',
	'div',
	'span',
	'center',
	'h1',
	'h2',
	'h3',
	'h4',
	'h5',
	'h6',
	'strong',
	'b',
	'em',
	'i',
	'u',
	's',
	'del',
	'ins',
	'mark',
	'small',
	'sub',
	'sup',
	'abbr',
	'blockquote',
	'q',
	'cite',
	'code',
	'pre',
	'kbd',
	'samp',
	'var',
	'ul',
	'ol',
	'li',
	'dl',
	'dt',
	'dd',
	'a',
	'img',
	'figure',
	'figcaption',
	'table',
	'caption',
	'colgroup',
	'col',
	'thead',
	'tbody',
	'tfoot',
	'tr',
	'th',
	'td',
	'details',
	'summary'
];

const ALLOWED_ATTR = [
	'href',
	'title',
	'alt',
	'src',
	'width',
	'height',
	'align',
	'colspan',
	'rowspan',
	'span',
	'start',
	'reversed',
	'open',
	'lang',
	'dir'
];

const ALLOWED_URI_REGEXP = /^(?:https?:\/\/|mailto:|#)/i;

export function renderRichText(source: string, format: RichTextFormat): string {
	const html =
		format === 'markdown'
			? (marked.parse(source, { async: false, gfm: true, breaks: false }) as string)
			: source;

	if (!DOMPurify.isSupported) return escapeHtml(source);

	return DOMPurify.sanitize(html, {
		ALLOWED_TAGS,
		ALLOWED_ATTR,
		ALLOWED_URI_REGEXP,
		ALLOW_DATA_ATTR: false,
		ALLOW_ARIA_ATTR: false
	});
}

function escapeHtml(text: string) {
	return text.replace(
		/[&<>"']/g,
		(c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c] as string
	);
}
