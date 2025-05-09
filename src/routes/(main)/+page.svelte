<script lang="ts">
	import Icon from "@iconify/svelte";
	import Button from "$lib/components/Button.svelte";
	import { goto } from "$app/navigation";
</script>

<div class="flex flex-col h-screen gap-4 justify-center items-center">
	<!-- placeholder logo -->
	<div class="w-24 h-24 bg-primary rounded-full mx-auto"></div>

	<div class="flex flex-row gap-2">
		<Button label="Shop" type="tertiary" onClick={() => goto("/shop")} />
		<Button label="Play" type="primary" onClick={() => goto("/play")} />
		<Button
			label="Statistics"
			type="tertiary"
			onClick={() => goto("/statistics")}
		/>
	</div>

	<div class="absolute right-3 bottom-3 flex flex-row gap-2">
		<Icon
			icon="material-symbols:info"
			class="text-3xl text-primary cursor-pointer"
			onclick={() => goto("/about")}
		/>
		<Icon
			icon="material-symbols:settings"
			class="text-3xl text-primary cursor-pointer"
			onclick={() => goto("/settings")}
		/>
	</div>
</div>
