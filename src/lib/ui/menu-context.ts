export const MENU_CTX = Symbol('hexo-menu');

export type MenuContext = {
	close: () => void;
};
