<script lang="ts" module>
	const skins = new Map<string, Promise<string | null>>();
</script>

<script lang="ts">
	import * as api from '$lib/api';
	import type { AccountView } from '$lib/api';
	import STEVE from '$lib/assets/steve.png';

	let { account = null, size = 24 }: { account?: AccountView | null; size?: number } = $props();

	let skin = $state(STEVE);

	$effect(() => {
		const uuid = account?.kind === 'microsoft' ? account.uuid : '';
		skin = STEVE;
		if (!uuid) return;

		let cached = skins.get(uuid);
		if (!cached) {
			cached = api.playerSkin(uuid).catch(() => null);
			skins.set(uuid, cached);
		}
		let alive = true;
		cached.then((url) => {
			if (alive && url) skin = url;
		});
		return () => (alive = false);
	});
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 8 8"
	aria-hidden="true"
	class="block shrink-0 [image-rendering:pixelated]"
>
	<svg x="0" y="0" width="8" height="8" viewBox="8 8 8 8">
		<image href={skin} width="64" height="64" preserveAspectRatio="xMinYMin meet" />
	</svg>
	<svg x="0" y="0" width="8" height="8" viewBox="40 8 8 8">
		<image href={skin} width="64" height="64" preserveAspectRatio="xMinYMin meet" />
	</svg>
</svg>
