<script>
    import FileLoader from './FileLoader.svelte';
    import Lookup from './Lookup.svelte';

    let serviceworker_version = "(unknown)";

    if ('serviceWorker' in navigator) {
	navigator.serviceWorker.register('serviceworker.js')
	    .then((registration) => {
		registration.onupdatefound = (e => alert("update!!"));
		console.log("Service worker registration successful.");
	    }).catch((error) => {
		console.log("Service worker registration failed: " + error);
	    });

	navigator.serviceWorker.addEventListener("message", (event => serviceworker_version = event.data));
	navigator.serviceWorker.controller.postMessage("getversion");
	navigator.serviceWorker.controller.postMessage("checkforupdates");
    }

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
    <button id="switch" on:click={e => { if (status == "find-stroke") { status = "query" } else { status = "find-stroke"} }}>
      {#if (status == "find-stroke")}
        <img src="abc-icon.svg" alt="lookup steno"/>
      {:else}
	<img src="STK-icon.svg" alt="lookup english"/>
      {/if}
    </button>
    <button id="load" class={(status == 'load-dict') ? 'selected' : ''} on:click={toggleLoad}>
      <img src="load-icon.svg" alt="load"/>
    </button>
	{#if status == "load-dict"}
        <h1>Load</h1>
	    <FileLoader on:message="{e => status = 'query'}" bind:dictionary bind:status={loader_status}/>
	{:else if status == "query"}
        <h1>Lookup</h1>
        {#if dictionary === null}
        <p id="nodict">No dictionary loaded.</p>
        {:else}
        <Lookup bind:dictionary={dictionary.data}/>
        {/if}
	{/if}

    <p id="version-info">Service worker version: {serviceworker_version}</p>
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
        grid-template-rows: 2em auto auto auto;
        grid-template-areas: "mode switch load"
                             "query query query"
                             "main main main"
			     "info info info";
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
      padding: 0;
    }

    button > img {
      width: 100%;
      height: 100%;
    }

    button.selected {
      background-color: black;
    }
    
    button#switch {
      grid-area: switch;
    }
    
    button#load {
      grid-area: load;
      padding: 0; 
    }

    p#nodict {
      grid-area: query;
    }

    p#version-info {
      grid-area: info;
      font-size: 14px;
    }
</style>
