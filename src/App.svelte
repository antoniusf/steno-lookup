<script>
	import FileLoader from './FileLoader.svelte';
    import ReverseLookup from './ReverseLookup.svelte';
	export let name;

	let status = 'load-dict';
    let loader_status = 'initializing';
	let dictionary = null;

    function toggleLoad (event) {
        if (status == 'load-dict') {
            status = 'query';
        }
        else if (status == 'query') {
            status = 'load-dict';
        }
    }
</script>

<main>
    <button id="switch">
    a
    </button>
    <button id="load" class={(status == 'load-dict') ? 'selected' : ''} on:click={toggleLoad}>
    b
    </button>
	{#if status == "load-dict"}
        <h1>Load</h1>
	    <FileLoader on:message="{e => status = 'query'}" bind:dictionary bind:status={loader_status}/>
	{:else if status == "query"}
        <h1>Lookup</h1>
        {#if dictionary === null}
        <p id="nodict">No dictionary loaded.</p>
        {:else}
        <ReverseLookup bind:dictionary={dictionary.data}/>
        {/if}
	{/if}
</main>

<style>
    main {
        max-width: 25em;
        width: 100%;
        font-family: Arial;
        text-align: center;
        margin: 0 auto;
        
        display: grid;
        grid-template-columns: minmax(0, 1fr) 2em 2em;
        grid-template-rows: 2em auto auto;
        grid-template-areas: "mode switch load"
        "query query query"
                             "main main main";
        grid-column-gap: 0.3em;
        grid-row-gap: 0.5em;
    }

    h1 {
      grid-area: mode;
      text-align: left;
      margin: auto 0 auto 0.2em;
      padding: 0;
      font-size: 1.5rem;
    }
    
    button {
      width: 100%;
      height: 100%;
      background-color: green;
      border: none;
      color: white;
      cursor: pointer;
    }

    button.selected {
      background-color: black;
    }
    
    button#switch {
      grid-area: switch;
    }
    
    button#load {
      grid-area: load;
    }

    p#nodict {
      grid-area: query;
    }
</style>
