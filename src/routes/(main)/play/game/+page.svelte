<script lang="ts">
	import { goto } from "$app/navigation";
	import { page } from "$app/state";
	import Button from "$lib/components/Button.svelte";
	import type { GameDifficulty, GameType } from "$lib/types/game";
	import Icon from "@iconify/svelte";
	import { invoke } from "@tauri-apps/api/core";
	import { onMount, onDestroy } from "svelte";
	import type { IndexOptions } from "$lib/types/database";

	let difficulty: GameDifficulty = $state("easy");
	let gameType: GameType = $state("directory");
	let indexOptions: IndexOptions = {
		path: "C:\\",
		index_files: true,
		index_directories: true,
		exclude_temporary: true,
		exclude_system: true,
		exclude_empty: true,
		exclude_hidden: true,
	};
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

	function index() {
		if (!isIndexing) {
			console.log("Starting indexing")
			isIndexing = true;
			// start indexing
			invoke("start_indexing", { indexOptions })
				.catch((error) => {
					console.error("Error while starting indexing:", error);
				});
		} else {
			console.log("Stopping indexing")
			isIndexing = false;
			// stop indexing
			invoke("stop_indexing", { pathString: "C:\\"})
				.catch((error) => {
					console.error("Error while stopping indexing:", error);
				});
		}
	}

	function randomDir() {
		invoke("get_random_dir", { pathString: "C:\\" })
			.then((result) => {
				console.log("Random directory:", result);
			})
			.catch((error) => {
				console.error("Error while getting random directory:", error);
			});
	}

	function randomFile() {
		invoke("get_random_file", { pathString: "C:\\" })
			.then((result) => {
				console.log("Random file:", result);
			})
			.catch((error) => {
				console.error("Error while getting random file:", error);
			});
	}
</script>

<div class="flex flex-col h-screen gap-4 justify-center items-center">
	<Button label="Index" type="primary" onClick={() => index()} />
	<Button label="Random dir" type="primary" onClick={() => randomDir()} />
	<Button label="Random file" type="primary" onClick={() => randomFile()} />
	<Button label="Back" type="primary" onClick={() => goto("/play")} />
</div>
