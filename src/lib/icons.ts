const files = import.meta.glob<string>('./assets/blocks/*.png', {
	eager: true,
	import: 'default'
});

const BLOCKS: Record<string, string> = Object.fromEntries(
	Object.entries(files).map(([path, url]) => [path.slice(path.lastIndexOf('/') + 1, -4), url])
);

export const ICON_CHOICES = ['grass'];

export function blockIcon(name: string): string {
	return BLOCKS[name] ?? BLOCKS.grass;
}
