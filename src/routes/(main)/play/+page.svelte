<!-- Game type screen -->
<script lang="ts">
	import Icon from "@iconify/svelte";
	import { onMount, onDestroy } from "svelte";
	import Button from "$lib/components/Button.svelte";
	import { goto } from "$app/navigation";
</script>

<div class="flex flex-col h-screen gap-4 justify-center items-center">
	<div class="flex flex-row gap-2">
		<!-- easy, medium, hard, expert -->
		<Button label="Easy" type="primary" onClick={() => goto("/play/game")} />
		<Button label="Medium" type="primary" onClick={() => console.log(`Medium clicked`)} />
		<Button label="Hard" type="primary" onClick={() => console.log(`Hard clicked`)} />
		<Button label="Expert" type="primary" onClick={() => console.log(`Expert clicked`)} />
		<Button label="Custom" type="primary" onClick={() => console.log(`Custom clicked`)} />
	</div>
	<Button label="Back" type="primary" onClick={() => goto("/")} />
</div>
