import type { Component } from 'svelte';
import LuAtSign from '~icons/lucide/at-sign';
import LuBell from '~icons/lucide/bell';
import LuBookmark from '~icons/lucide/bookmark';
import LuCheckCheck from '~icons/lucide/check-check';
import LuCode from '~icons/lucide/code';
import LuCrown from '~icons/lucide/crown';
import LuHeart from '~icons/lucide/heart';
import LuLink from '~icons/lucide/link';
import LuMessageCircle from '~icons/lucide/message-circle';
import LuMessageSquare from '~icons/lucide/message-square';
import LuRadio from '~icons/lucide/radio';
import LuTv from '~icons/lucide/tv';
import LuUsers from '~icons/lucide/users';
import LuArrowLeft from '~icons/lucide/arrow-left';
import LuArrowRight from '~icons/lucide/arrow-right';
import LuArrowUpFromLine from '~icons/lucide/arrow-up-from-line';
import LuBlocks from '~icons/lucide/blocks';
import LuBox from '~icons/lucide/box';
import LuCastle from '~icons/lucide/castle';
import LuCheck from '~icons/lucide/check';
import LuChevronDown from '~icons/lucide/chevron-down';
import LuChevronRight from '~icons/lucide/chevron-right';
import LuChevronUp from '~icons/lucide/chevron-up';
import LuClock from '~icons/lucide/clock';
import LuCoffee from '~icons/lucide/coffee';
import LuCompass from '~icons/lucide/compass';
import LuCopy from '~icons/lucide/copy';
import LuCrosshair from '~icons/lucide/crosshair';
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
import LuLayers from '~icons/lucide/layers';
import LuList from '~icons/lucide/list';
import LuLogIn from '~icons/lucide/log-in';
import LuLogOut from '~icons/lucide/log-out';
import LuMap from '~icons/lucide/map';
import LuMegaphone from '~icons/lucide/megaphone';
import LuMenu from '~icons/lucide/menu';
import LuMinus from '~icons/lucide/minus';
import LuMonitor from '~icons/lucide/monitor';
import LuMoon from '~icons/lucide/moon';
import LuPackage from '~icons/lucide/package';
import LuPalette from '~icons/lucide/palette';
import LuPencil from '~icons/lucide/pencil';
import LuPickaxe from '~icons/lucide/pickaxe';
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
	back: LuArrowLeft,
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
	map: LuMap,
	crosshair: LuCrosshair,
	castle: LuCastle,
	compass: LuCompass,
	pickaxe: LuPickaxe,
	layers: LuLayers,
	'win-min': LuMinus,
	'win-max': LuSquare,
	heart: LuHeart,
	bookmark: LuBookmark,
	bell: LuBell,
	users: LuUsers,
	crown: LuCrown,
	'check-all': LuCheckCheck,
	link: LuLink,
	'link-discord': LuMessageCircle,
	'link-bilibili': LuTv,
	'link-twitch': LuRadio,
	'link-x': LuAtSign,
	'link-github': LuCode,
	'link-qq': LuMessageSquare
};

export type IconName = keyof typeof ICONS;
