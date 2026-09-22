<script lang="ts">
	let { name, size = 16 }: { name: string; size?: number } = $props();


	const paths: Record<string, string> = {

		plus: 'M12 5v14 M5 12h14',
		minus: 'M5 12h14',
		check: 'm5 12 4 4L19 6',
		close: 'm6 6 12 12 M6 18 18 6',
		search: 'M21 21l-5-5 M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0Z',
		refresh: 'M20 11a8 8 0 1 0-2 6 M20 5v6h-6',
		arrow: 'M5 12h14 m-5-5 5 5-5 5',
		chevron: 'm9 5 7 7-7 7',
		down: 'm6 9 6 6 6-6',
		up: 'm6 15 6-6 6 6',
		menu: 'M4 7h16 M4 12h16 M4 17h16',
		caret: 'm5 9 7 7 7-7Z',
		info: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z M12 11v5 M12 8h.01',


		play: 'm7 4 13 8-13 8Z',
		stop: 'M6 6h12v12H6Z',
		edit: 'm15 4 5 5 M4 16 16 4a2 2 0 0 1 4 4L8 20H4Z',
		tag: 'M3 12V3h9l9 9-9 9Z M7.5 7.5h.01',
		folder: 'M3 6h6l2 2h10v12H3Z',
		export: 'M12 16V4 m-5 5 5-5 5 5 M4 17v3h16v-3',
		copy: 'M8 8h13v13H8Z M16 8V3H3v13h5',
		trash: 'M4 7h16 M9 7V4h6v3 M6 7l1 13h10l1-13 M10 11v6 M14 11v6',
		shortcut: 'M14 4h6v6 M20 4 10 14 M18 14v6H4V6h6',
		download: 'M12 3v12 m-5-5 5 5 5-5 M4 16v5h16v-5',
		upload: 'M12 20V8 m-5 5 5-5 5 5 M4 4h16',


		box: 'm12 3 9 5v9l-9 5-9-5V8Z M3 8l9 5 9-5 M12 13v9',
		package: 'M3 7h18v13H3Z M3 7l2-3h14l2 3 M12 7v13',
		image: 'M3 5h18v14H3Z m3 10 4-4 4 4 3-3 4 4 M8.5 9.5h.01',
		notes: 'M6 3h9l3 3v15H6Z M15 3v3h3 M9 12h6 M9 16h5',
		server: 'M3 5h18v6H3Z M3 13h18v6H3Z M7 8h.01 M7 16h.01',
		globe: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z M3 12h18 M12 3a14 14 0 0 1 0 18 14 14 0 0 1 0-18',
		terminal: 'm5 7 5 5-5 5 M12 17h7',
		sun: 'M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8 M12 2v2 M12 20v2 M2 12h2 M20 12h2 M5 5l1.5 1.5 M17.5 17.5 19 19 M19 5l-1.5 1.5 M6.5 17.5 5 19',
		moon: 'M20.5 14.5A8.5 8.5 0 0 1 9.5 3.5a8.5 8.5 0 1 0 11 11Z',
		monitor: 'M3 5h18v11H3Z M9 20h6 M12 16v4',


		settings:
			'M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8 M9 3h6l1 3 3 1 2 5-2 5-3 1-1 3H9l-1-3-3-1-2-5 2-5 3-1Z',
		coffee: 'M4 8h13v6a5 5 0 0 1-5 5H9a5 5 0 0 1-5-5Z M17 10h2a2 2 0 1 1 0 5h-2 M7 3v2 M11 3v2',
		wrench: 'M17 3a5 5 0 0 0-5 7L4 18v2h2l8-8a5 5 0 0 0 7-5l-3 3-3-1Z',
		shield: 'M12 3l8 3v6c0 5-4 8-8 9-4-1-8-4-8-9V6Z',
		key: 'M15 9a4 4 0 1 1-8 0 4 4 0 0 1 8 0Z M13 11l7 7-2 2-2-2-2 2-3-3',
		palette:
			'M12 3a9 9 0 0 0 0 18c1 0 2-1 2-2s-1-2 0-3 2 0 3 0a4 4 0 0 0 4-4c0-5-4-9-9-9Z M8 9h.01 M7 13h.01 M11 7h.01',
		user: 'M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8 M4 20c0-4 4-6 8-6s8 2 8 6',
		clock: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z M12 7v5l3 2',
		rss: 'M5 19h.01 M4 11a9 9 0 0 1 9 9 M4 4a16 16 0 0 1 16 16',
		list: 'M8 5h13 M8 12h13 M8 19h13 M3 5h.01 M3 12h.01 M3 19h.01',


		'win-min': 'M5 12h14',
		'win-max': 'M5 5h14v14H5Z'
	};


	const filled = new Set(['play', 'caret', 'stop']);
	const isFilled = $derived(filled.has(name));
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill={isFilled ? 'currentColor' : 'none'}
	stroke="currentColor"
	stroke-width={isFilled ? 1 : 1.7}
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
>
	<path d={paths[name] ?? paths.box} />
</svg>
