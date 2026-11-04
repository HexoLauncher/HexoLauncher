export interface McSegment {
	text: string;
	color?: string;
	bold?: boolean;
	italic?: boolean;
	underline?: boolean;
	strike?: boolean;
}

const COLORS: Record<string, string | undefined> = {
	'0': undefined,
	'1': '#0000aa',
	'2': '#00aa00',
	'3': '#00aaaa',
	'4': '#aa0000',
	'5': '#aa00aa',
	'6': '#ffaa00',
	'7': '#aaaaaa',
	'8': '#555555',
	'9': '#5555ff',
	a: '#55ff55',
	b: '#55ffff',
	c: '#ff5555',
	d: '#ff55ff',
	e: '#ffff55',
	f: undefined
};

export function parseMcText(source: string): McSegment[] {
	const segments: McSegment[] = [];
	let style: Omit<McSegment, 'text'> = {};
	let text = '';

	const flush = () => {
		if (text) segments.push({ text, ...style });
		text = '';
	};

	for (let i = 0; i < source.length; i++) {
		const ch = source[i];
		if (ch !== '§' || i + 1 >= source.length) {
			text += ch;
			continue;
		}
		const code = source[++i].toLowerCase();
		flush();
		if (code in COLORS) style = { color: COLORS[code] };
		else if (code === 'l') style = { ...style, bold: true };
		else if (code === 'o') style = { ...style, italic: true };
		else if (code === 'n') style = { ...style, underline: true };
		else if (code === 'm') style = { ...style, strike: true };
		else if (code === 'r') style = {};
	}
	flush();
	return segments;
}

export function stripMcText(source: string): string {
	return parseMcText(source)
		.map((segment) => segment.text)
		.join('');
}
