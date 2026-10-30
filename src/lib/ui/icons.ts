import type { Component } from 'svelte';
import LuArrowRight from '~icons/lucide/arrow-right';
import LuArrowUpFromLine from '~icons/lucide/arrow-up-from-line';
import LuBlocks from '~icons/lucide/blocks';
import LuBox from '~icons/lucide/box';
import LuCheck from '~icons/lucide/check';
import LuChevronDown from '~icons/lucide/chevron-down';
import LuChevronRight from '~icons/lucide/chevron-right';
import LuChevronUp from '~icons/lucide/chevron-up';
import LuClock from '~icons/lucide/clock';
import LuCoffee from '~icons/lucide/coffee';
import LuCopy from '~icons/lucide/copy';
import LuShirt from '~icons/lucide/shirt';
import LuDownload from '~icons/lucide/download';
import LuExternalLink from '~icons/lucide/external-link';
import LuEye from '~icons/lucide/eye';
import LuFileText from '~icons/lucide/file-text';
import LuFolder from '~icons/lucide/folder';
import LuGlobe from '~icons/lucide/globe';
import LuImage from '~icons/lucide/image';
import LuInfo from '~icons/lucide/info';
import LuKey from '~icons/lucide/key';
import LuLayoutGrid from '~icons/lucide/layout-grid';
import LuList from '~icons/lucide/list';
import LuLogIn from '~icons/lucide/log-in';
import LuLogOut from '~icons/lucide/log-out';
import LuMegaphone from '~icons/lucide/megaphone';
import LuMenu from '~icons/lucide/menu';
import LuMinus from '~icons/lucide/minus';
import LuMonitor from '~icons/lucide/monitor';
import LuMoon from '~icons/lucide/moon';
import LuPackage from '~icons/lucide/package';
import LuPalette from '~icons/lucide/palette';
import LuPencil from '~icons/lucide/pencil';
import LuPlay from '~icons/lucide/play';
import LuPlus from '~icons/lucide/plus';
import LuRefreshCw from '~icons/lucide/refresh-cw';
import LuRotateCcw from '~icons/lucide/rotate-ccw';
import LuRss from '~icons/lucide/rss';
import LuSearch from '~icons/lucide/search';
import LuServer from '~icons/lucide/server';
import LuSettings from '~icons/lucide/settings';
import LuShield from '~icons/lucide/shield';
import LuSquare from '~icons/lucide/square';
import LuSun from '~icons/lucide/sun';
import LuTag from '~icons/lucide/tag';
import LuTerminal from '~icons/lucide/terminal';
import LuTrash2 from '~icons/lucide/trash-2';
import LuUpload from '~icons/lucide/upload';
import LuUser from '~icons/lucide/user';
import LuWrench from '~icons/lucide/wrench';
import LuX from '~icons/lucide/x';

export const ICONS: Record<string, Component> = {
	plus: LuPlus,
	minus: LuMinus,
	check: LuCheck,
	close: LuX,
	search: LuSearch,
	refresh: LuRefreshCw,
	arrow: LuArrowRight,
	chevron: LuChevronRight,
	down: LuChevronDown,
	up: LuChevronUp,
	menu: LuMenu,
	caret: LuChevronDown,
	info: LuInfo,
	play: LuPlay,
	stop: LuSquare,
	edit: LuPencil,
	tag: LuTag,
	folder: LuFolder,
	export: LuUpload,
	copy: LuCopy,
	trash: LuTrash2,
	shortcut: LuExternalLink,
	download: LuDownload,
	upload: LuArrowUpFromLine,
	box: LuBox,
	grid: LuLayoutGrid,
	package: LuPackage,
	image: LuImage,
	notes: LuFileText,
	server: LuServer,
	globe: LuGlobe,
	terminal: LuTerminal,
	sun: LuSun,
	moon: LuMoon,
	monitor: LuMonitor,
	settings: LuSettings,
	coffee: LuCoffee,
	wrench: LuWrench,
	shield: LuShield,
	key: LuKey,
	palette: LuPalette,
	user: LuUser,
	shirt: LuShirt,
	clock: LuClock,
	rss: LuRss,
	megaphone: LuMegaphone,
	list: LuList,
	blocks: LuBlocks,
	eye: LuEye,
	login: LuLogIn,
	logout: LuLogOut,
	rerender: LuRotateCcw,
	'win-min': LuMinus,
	'win-max': LuSquare
};

export type IconName = keyof typeof ICONS;
