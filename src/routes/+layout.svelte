<script lang="ts">
    let { children } = $props();
</script>

<style>
    header {
        width: 100%;
        height: 3em;
        border-bottom: 1px solid rgb(255, 255, 255);
        display: flex;
        justify-content: flex-start;
        align-items: center;
        padding-left: 1em;
        /* font-weight: bold; */
        font-size: 1.2em;

        background-color: rgba(255, 255, 255, 0.1);
    }

    .body {
        margin-top: 2em;
        width: 100%;
        min-height: calc(100vh - 10em);
        /* display: grid;
        place-items: center; */
    }

    img {
        width: 2em;
        height: 2em;

        filter: invert(1);
    }
</style>

<header>
    <div style="align-items: center; display: flex; gap: 1em;">
        <img src="/favicon.ico" alt="favicon">
        csci 4131 server manager
    </div>
</header>

<div class="body">
    {@render children()}
</div>
