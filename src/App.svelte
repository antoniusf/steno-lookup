<!-- This Source Code Form is subject to the terms of the Mozilla Public
   - License, v. 2.0. If a copy of the MPL was not distributed with this
   - file, You can obtain one at https://mozilla.org/MPL/2.0/. -->
<script>
    import FileLoader from './FileLoader.svelte';
    import Lookup from './Lookup.svelte';
    import FindStroke from './FindStroke.svelte';

    let serviceworker_version = "(unknown)";

    if ('serviceWorker' in navigator) {
	navigator.serviceWorker.register('serviceworker.js')
	    .then((registration) => {
		console.log("Service worker registration successful.");
	    }).catch((error) => {
		console.log("Service worker registration failed: " + error);
	    });

	navigator.serviceWorker.addEventListener("message", (event => {
	    if (event.data.type == "version-info") {
		serviceworker_version = event.data.serviceworker_version;
	    }
            else if (event.data.type == "update-info") {
                if (event.data.status == "up-to-date") {
                    update_info.update_available = false;
                }
                else if (event.data.status == "available") {
                    update_info.update_available = true;
                }
                else if (event.data.status == "installed") {
                    update_info.update_available = false;
                    window.location.reload();
                }

		update_info.date_checked = event.data.date_checked;
                update_info.update_size = event.data.update_size;
		console.log(JSON.stringify(update_info, null, 2));
	    }
	    else if (event.data.type == "serviceworker-info") {
		update_info.serviceworker_info = event.data.text;
		console.log(update_info.serviceworker_info);
	    }
	}));

	if (navigator.serviceWorker.controller) {
	    navigator.serviceWorker.controller.postMessage("get-version");
	    navigator.serviceWorker.controller.postMessage("get-update-info");
	    navigator.serviceWorker.controller.postMessage("check-for-updates");
	}
    }

    let status = 'load-dict';
    let loader_status = 'initializing';
    let update_info = {};
    let dictionary = null;
    let titles = {
        "load-dict": "Load",
        "query": "Lookup",
	"find-stroke": "Find Stroke"
    }
    

    function toggleLoad (event) {
        if (status == 'load-dict') {
            status = 'query';
        }
        else if (status == 'query' || status == 'find-stroke') {
            status = 'load-dict';
        }
    }
</script>

<div id="container">
  <header>
    <h1>{titles[status]}</h1>
    <button id="switch" on:click={e => { if (status == "query") { status = "find-stroke" } else { status = "query"} }} disabled={dictionary == null}>
      {#if (status == "query")}
        <img src="STK-icon.svg" alt="find stroke"/>
      {:else}
        <img src="abc-icon.svg" alt="lookup"/>
      {/if}
    </button>
    <button id="load" class={(status == 'load-dict') ? 'selected' : ''} on:click={toggleLoad} disabled={dictionary == null}>
      <img src="load-icon.svg" alt="load"/>
    </button>
  </header>

  <main>
    {#if status == "load-dict"}
      <FileLoader on:message="{e => status = 'query'}" bind:dictionary bind:update_info bind:status={loader_status}/>
    {:else if status == "query"}
      {#if dictionary === null}
        <p id="nodict">No dictionary loaded.</p>
      {:else}
        <!-- TODO: does this have to be a bind? I mean, we shouldn't really get any data out this way.-->
        <Lookup bind:dictionary={dictionary}/>
      {/if}
    {:else if status == "find-stroke"}
      {#if dictionary === null} // TODO: un-duplicate this code
        <p id="nodict">No dictionary loaded.</p>
      {:else}
        <FindStroke bind:dictionary={dictionary}/>
      {/if}
    {/if}
  </main>

  <p id="version-info">App version: __version__<br>Service worker version: {serviceworker_version}<br>File a bug or contribute to development on <a href="https://github.com/antoniusf/steno-lookup" target="_blank">github</a></p>
</div>

<style>
    div#container {
        max-width: 25em;
        width: 100%;
        font-family: Arial;
        text-align: center;
        margin: 0 auto;
        padding: 0;
    }

    header {
        width: 100%;
        margin: 0;
        margin-bottom: 0.5em;
        padding: 0;
        /* I know I could use flexbox for this,
         * but grid just seems easier */
        display: grid;
        grid-template-columns: minmax(0, 1fr) 2em 2em;
        grid-template-rows: 2em;
        grid-template-areas: "mode switch load";
        grid-column-gap: 0.3em;
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

    button:disabled {
      background-color: #aaa;
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
      line-height: 1.35;
    }
</style>
