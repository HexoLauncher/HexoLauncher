import { getLocale, locales, localStorageKey, setLocale } from '$lib/paraglide/runtime.js';
import type { Locale } from '$lib/paraglide/runtime.js';

export const LOCALE_NAMES: Record<Locale, string> = {
	en: 'English',
	'zh-TW': '繁體中文'
};

export const LOCALE_OPTIONS = locales.map((id) => ({ id, label: LOCALE_NAMES[id] ?? id }));

export function storedLocale(): Locale | '' {
	try {
		const value = localStorage.getItem(localStorageKey);
		return locales.includes(value as Locale) ? (value as Locale) : '';
	} catch {
		return '';
	}
}

export function useLocale(value: Locale | '') {
	if (value === '') {
		try {
			localStorage.removeItem(localStorageKey);
		} catch {
			return;
		}
		window.location.reload();
		return;
	}
	setLocale(value);
}

export function applyDocumentLocale() {
	document.documentElement.lang = getLocale();
}
