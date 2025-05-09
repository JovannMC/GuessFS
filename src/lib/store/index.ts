import { writable } from "svelte/store";

export const statistics = writable({
    total: 0,
    win: 0,
    close: 0,
    lose: 0,
});

export const currentPath = writable("/");