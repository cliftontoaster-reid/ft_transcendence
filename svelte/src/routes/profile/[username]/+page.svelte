<script lang="ts">
	import ProfileCard from '$lib/components/ProfileCard.svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();
</script>

<svelte:head>
	<title>{data.profile.displayName} | Profile</title>
</svelte:head>

<ProfileCard profile={data.profile} />
