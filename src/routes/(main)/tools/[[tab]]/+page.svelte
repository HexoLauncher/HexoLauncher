<script lang="ts">
	import { page } from '$app/state';
	import NavList from '$lib/ui/NavList.svelte';
	import type { Page } from '$lib/ui';
	import ClusterFinder from '$lib/components/tools/ClusterFinder.svelte';
	import SeedMapView from '$lib/components/tools/SeedMapView.svelte';
	import SeedSource from '$lib/components/tools/SeedSource.svelte';
	import SlimeFinder from '$lib/components/tools/SlimeFinder.svelte';
	import WorldMapView from '$lib/components/tools/WorldMapView.svelte';
	import { navigate } from '$lib/nav';
	import { persist, tools } from '$lib/tools/state.svelte';
	import { m } from '$lib/paraglide/messages.js';

	const pages: Page[] = [
		{ id: 'map', label: m.tools_tab_map(), icon: 'map' },
		{ id: 'world', label: m.tools_tab_world(), icon: 'compass' },
		{ id: 'slime', label: m.tools_tab_slime(), icon: 'crosshair' },
		{ id: 'clusters', label: m.tools_tab_clusters(), icon: 'castle' }
	];

	const tab = $derived(pages.some((p) => p.id === page.params.tab) ? page.params.tab! : 'map');

	// 設定與地圖視角存到 localStorage；拖動地圖時每幀都會變，所以延遲寫入。
	let timer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		void [tools.mode, tools.instanceId, tools.worldFolder, tools.manualSeed, tools.manualVersion];
		void [tools.dimension, tools.slime, tools.terrain, tools.sets, tools.strongholds];
		void [tools.seedView.x, tools.seedView.z, tools.seedView.zoom];
		void [tools.worldView.x, tools.worldView.z, tools.worldView.zoom, tools.worldDimension, tools.worldOverlay];
		void [tools.prune.useInhabited, tools.prune.minutes, tools.prune.useAge, tools.prune.days];
		clearTimeout(timer);
		timer = setTimeout(persist, 400);
		return () => clearTimeout(timer);
	});
</script>

<svelte:head><title>{m.nav_tools()} · Nolauncher</title></svelte:head>

<div class="flex min-h-0 flex-1 bg-window">
	<NavList {pages} selected={tab} onselect={(id) => navigate(id === 'map' ? '/tools' : `/tools/${id}`)} width="172px" />

	<div class="flex min-w-0 flex-1 flex-col gap-2.5 px-4 pt-3 pb-3.5">
		<header class="flex shrink-0 items-center gap-2.5">
			<h2 class="text-[15px] font-semibold">{pages.find((p) => p.id === tab)?.label}</h2>
		</header>

		<SeedSource worldOnly={tab === 'world'} />

		{#if tab === 'map'}
			<SeedMapView />
		{:else if tab === 'world'}
			<WorldMapView />
		{:else if tab === 'slime'}
			<SlimeFinder />
		{:else}
			<ClusterFinder />
		{/if}
	</div>
</div>
