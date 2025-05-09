import { writable } from "svelte/store";

export const CustomSettings = writable({
    hintCount: 3,
    timeLimit: 60,
});