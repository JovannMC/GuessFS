<script lang="ts">
	import { goto } from "$app/navigation";
	import { page } from "$app/state";
	import Button from "$lib/components/Button.svelte";
	import type { GameDifficulty, GameType } from "$lib/types/game";
	import Icon from "@iconify/svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { onMount, onDestroy } from "svelte";

	let difficulty: GameDifficulty = $state("easy");
	let gameType: GameType = $state("directory");
	let isIndexing = false;

	onMount(() => {
		const diff = page.url.searchParams.get("difficulty");
		if (diff) {
			difficulty = diff as GameDifficulty;

			if (difficulty === "custom") {
				// get custom game type stuff, store in store for next time
				// hint count, time limit, etc.
			}
		}

		if (gameType === "directory") {
			// do something
		}

		const handleKeydown = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				goto("/play");
			}
		};

		window.addEventListener("keydown", handleKeydown);

		return () => {
			window.removeEventListener("keydown", handleKeydown);
		};
	});

	function test() {
		if (!isIndexing) {
			console.log("Starting indexing")
			isIndexing = true;
			// start indexing
			invoke("start_indexing", { pathString: "C:\\", indexFiles: true })
				.then((result) => {
					console.log("Result:", result);
				})
				.catch((error) => {
					console.error("Error:", error);
				});
		} else {
			console.log("Stopping indexing")
			isIndexing = false;
			// stop indexing
			invoke("stop_indexing", { pathString: "C:\\"})
				.then((result) => {
					console.log("Result:", result);
				})
				.catch((error) => {
					console.error("Error:", error);
				});
		}
	}
</script>

<div class="flex flex-col h-screen gap-4 justify-center items-center">
	<Button label="Test" type="primary" onClick={() => test()} />
	<Button label="Back" type="primary" onClick={() => goto("/play")} />
</div>
