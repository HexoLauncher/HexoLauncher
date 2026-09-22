<script lang="ts">
	let { size = 14, class: klass = '' }: { size?: number; class?: string } = $props();
</script>

<span
	class="inline-block shrink-0 animate-spin rounded-full border-2 border-line border-t-brand {klass}"
	style:width="{size}px"
	style:height="{size}px"
></span>
