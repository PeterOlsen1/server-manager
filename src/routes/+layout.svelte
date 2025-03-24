<script lang="ts">
    import theme, { toggleTheme, loadTheme } from "$lib/style.svelte";
    import { onMount } from "svelte";
    let { children } = $props();

    onMount(loadTheme);
</script>

<style>
    header {
        width: calc(100vw - 1em);
        height: 3em;
        border-bottom: 1px solid var(--border-color);
        display: flex;
        justify-content: flex-start;
        align-items: center;
        padding-left: 1em;
        font-size: 1.2em;

        background-color: var(--header-background-color);
        background: blur(10px);
    }

    .body {
        width: 100%;
        min-height: calc(100vh - 10em);
    }

    img {
        width: 2em;
        height: 2em;

        filter: var(--black-image-filter);
    }

    .theme-toggle {
        display: flex;
        justify-content: flex-end;
        align-items: center;
        margin-right: 1em;
        padding: 0.5em;
        border-radius: 0.25em;
        cursor: pointer;

        background-color: transparent;
        transition: background-color 0.2s;

        &:hover {
            background-color: var(--button-background-color);
        }
    }
</style>

<header>
    <div style="align-items: center; display: flex; gap: 1em; flex-grow: 1;">
        <img src="/favicon.ico" alt="favicon">
        csci 4131 server manager
    </div>
    <div class="theme-toggle" onclick={toggleTheme}>
        {#if theme.theme === 'dark'}
            <img src='/sun.svg' alt='sun' style="width: 1.5em; height: 1.5em;">
        {:else}
            <img src='/moon.svg' alt='moon' style="width: 1.5em; height: 1.5em;">
        {/if}
    </div>
</header>

<div class="body">
    {@render children()}
</div>
