<!-- This Source Code Form is subject to the terms of the Mozilla Public
   - License, v. 2.0. If a copy of the MPL was not distributed with this
   - file, You can obtain one at https://mozilla.org/MPL/2.0/. -->
<script>
    import FileLoader from './FileLoader.svelte';
    import Lookup from './Lookup.svelte';
    import FindStroke from './FindStroke.svelte';

    import { set, get } from 'idb-keyval';

    let status = 'load-dict';
    let loader_status = 'initializing';
    let update_info = {};
    let serviceworker_version = "(unknown)";
    let dictionary = null;
    let titles = {
        "load-dict": "Load",
        "query": "Lookup",
	"find-stroke": "Find Stroke"
    }

    const VERSION_UNKNOWN = Symbol("version_unknown");

    get("user-knows-about-this-version").then(result => {
	if (result) {
	    update_info.user_knows_about_this_version = result;
	}
	else {
	    update_info.user_knows_about_this_version = VERSION_UNKNOWN;
	}
    });

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
		
		if (update_info.user_knows_about_this_version == VERSION_UNKNOWN) {
		    // user-knows-about-this-version is not initialized yet (this only happens once,
		    // the first time the app is loaded), so we want to initialize it properly.
		    update_info.user_knows_about_this_version = event.data.current_version;
		    set("user-knows-about-this-version", event.data.current_version);
		}
		
                if (event.data.status == "up-to-date") {
                    update_info.update_available = false;
                }
                else if (event.data.status == "available") {
                    update_info.update_available = true;
		    update_info.new_version = event.data.new_version;

		    // if the load-dict tab is open and not in a state where it can close automatically,
		    // we want to assume that the user knows about the new update
		    if (status == "load-dict" && loader_status != "initializing" && loader_status != "reading") {
			update_info.user_knows_about_this_version = event.data.new_version;
			set("user-knows-about-this-version", event.data.new_version);
		    }
                }
                else if (event.data.status == "installed") {
                    update_info.update_available = false;
		    update_info.user_knows_about_this_version = event.data.version;
		    set("user-knows-about-this-version", event.data.version);

                    window.location.reload();
                }

		update_info.date_checked = event.data.date_checked;
                update_info.update_size = event.data.update_size;
		update_info.new_version = event.data.new_version;

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

    async function toggleLoad (event) {
        if (status == 'load-dict') {
            status = 'query';
        }
        else if (status == 'query' || status == 'find-stroke') {
            status = 'load-dict';
	    if (update_info.new_version) {
		if (update_info.user_knows_about_this_version != update_info.new_version) {
		    // well, they know now
		    update_info.user_knows_about_this_version = update_info.new_version;
		    await set("user-knows-about-this-version", update_info.new_version);
		}
	    }
        }
    }
</script>

<div id="container">
  <header>
    <h1 id="mode-label" aria-live="polite">{titles[status]}</h1>
    <nav>
      <button id="switch" on:click={e => { if (status == "query") { status = "find-stroke" } else { status = "query"} }} disabled={dictionary == null}>
        {#if (status == "query")}
          <img src="STK-icon.svg" alt="find stroke"/>
        {:else}
          <img src="abc-icon.svg" alt="lookup"/>
        {/if}
      </button>
      <button id="load" class={(status == 'load-dict') ? 'selected' : ''} on:click={toggleLoad} disabled={dictionary == null}>
        <img src="load-icon.svg" alt="load"/>
         {#if update_info.new_version
	  && update_info.user_knows_about_this_version != "unknown"
	  && update_info.user_knows_about_this_version != update_info.new_version}
	  <div id="notification-marker"></div>
        {/if}
      </button>
    </nav>
  </header>

  <main>
    {#if status == "load-dict"}
      <FileLoader bind:dictionary bind:update_info bind:status={loader_status} bind:app_status={status}/>
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

  <p id="version-info">App version: __version__<br>Updater version: {serviceworker_version}<br>File a bug or contribute to development on <a href="https://github.com/antoniusf/steno-lookup" target="_blank">github</a></p>
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
        height: 2em;
        margin: 0;
        margin-bottom: 0.5em;
        padding: 0;
        display: flex;
    }

    h1 {
      grid-area: mode;
      text-align: left;
      margin: auto 0 auto 0.2em;
      padding: 0;
      font-size: 1.5rem;
      flex-grow: 1;
      flex-shrink: 1;
    }

    nav {
        width: auto;
        height: auto;
        padding: 0;
        margin: 0;
        display: flex;
        column-gap: 0.3em;
    }
    
    button {
      width: 2em;
      height: 2em;
      background-color: green;
      border: none;
      color: white;
      cursor: pointer;
      padding: 0;
    }

    button:focus {
      background-color: white;
      border: 0.1em solid black;
      color: black;
    }

    button:focus > img {
      filter: brightness(0.0);
    }

    button:disabled {
      background-color: #aaa;
    }

    button > img {
      width: 100%;
      height: 100%;
    }

    button.selected:not(:focus) {
      background-color: black;
    }
    
    button#switch {
      grid-area: switch;
    }
    
    button#load {
      grid-area: load;
      padding: 0; 
      position: relative; /* this is for the marker */
    }

    div#notification-marker {
      background-color: #fff;
      border: 0.2em solid #333;
      width: 0.7em;
      height: 0.7em;
      border-radius: 50%;
      
      box-sizing: border-box; /* this makes positioning much easier */
      position: absolute;
      top: -0.25em;
      left: calc(2em - 0.7em + 0.25em);
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
