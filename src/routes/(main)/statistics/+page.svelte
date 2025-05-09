<script lang="ts">
	import { goto } from "$app/navigation";
	import Button from "$lib/components/Button.svelte";
    import Icon from "@iconify/svelte";
    import { onMount, onDestroy } from "svelte";
</script>

<div class="flex flex-col h-screen gap-4 justify-center items-center">
    	<Button label="Back" type="primary" onClick={() => goto("/")} />
</div>