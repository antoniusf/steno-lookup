<!-- This Source Code Form is subject to the terms of the Mozilla Public
   - License, v. 2.0. If a copy of the MPL was not distributed with this
   - file, You can obtain one at https://mozilla.org/MPL/2.0/. -->
<script>
  // TODO: move updater into its own separate element
  import { createEventDispatcher, onMount } from 'svelte';
  import { set, get, del } from 'idb-keyval';
  import { textToStroke, strokeToText, formatFilesize } from './util';
  import { initialize, loadJson } from './wasm-interface.js';

  const dispatch = createEventDispatcher();

  export let dictionary;
  export let status;
  export let update_info;

  let big_status_message = "";
  let small_status_message = "";

  let files = [];

  onMount(async () => {

      // status is "initializing" only on page load, so that we get a
      // chance to try and load the dictionary from storage.
      if (status == "initializing") {
        let stored_dictionary = await get("dictionary");
        if (!stored_dictionary) {
            status = "choosefile";
        } else {
            // TODO: can this fail?
	    try {
		dictionary = await initialize(stored_dictionary.data);
	    }
	    catch (error) {
		status = "error";
		big_status_message = "Error occured while trying to load stored dictionary.";
		small_status_message = `${error}`;
		return;
	    }
	    dictionary.name = stored_dictionary.name;
            status = "loaded";
            signalDone();
        }
      }
  });

    
  $: readFile(files);

  function readFile (files) {
      if (files.length > 0) {
	  let file = files[0];
	  if (file.size > 10 * 2**20) {
	      status = "error";
	      big_status_message = "Sorry, we only accept files up to 10MB currently.";
	      small_status_message = "";
	  } else if (file.type != "application/json") {
	      status = "error";
	      big_status_message = "Sorry, right now we can only read plover json dictionaries.";
	      small_status_message = "";
	  } else {
	      status = "reading";
	      big_status_message = "Loading... 0%";
	      small_status_message = "";
              let filereader = new FileReader();
              filereader.addEventListener("load", event => finishReadFile(filereader));
              filereader.addEventListener("progress", event => {
		  readprogress = Math.floor(event.loaded / event.total * 100);
		  status_message = `Loading... ${readprogress}%`;
	      });
              filereader.addEventListener("abort", event => { status = "error"; big_status_message = "Aborted."; small_status_message = ""; });
              filereader.addEventListener("error", event => { status = "error"; big_status_message = "Your browser failed to load the file. Please try again."; small_status_message = ""; });
              filereader.readAsText(file);
	  }
      }
  }

  async function finishReadFile (filereader) {
      big_status_message = "Organizing data...";
      small_status_message = "";
      try {
	  dictionary = await loadJson(filereader.result);
      } catch (error) {
          console.log(error);
          status = "error";
	  if (!error.message) {
	      error = {
		  message: "An error occured while trying to load your dictionary.",
	          details: `${error}`
	      }
	  }
          big_status_message = error.message;
	  small_status_message = error.details;
          return;
      }

      dictionary.name = files[0].name;

      await set("dictionary", { name: dictionary.name, data: dictionary.data });

      status = "loaded";
      big_status_message = "";
      small_status_message = "";
      signalDone();
  }

  async function removeDict (event) {
      dictionary = null;
      await del("dictionary");
      status = "choosefile";
  }

  function signalDone () {
      dispatch("message", {
	  status: "done"
      });
  }

  const date_formatter = new Intl.DateTimeFormat(undefined, {
      month:"long",
      day:"numeric",
      hour:"numeric",
      minute:"numeric"
  });
      
</script>

<style>
input {
    width: 100%;
    margin: 0;
    margin-bottom: 0.5em;
}

p {
  margin: 0;
  padding: 0;
}

p#smallstatus {
  margin-top: 0.5em;
  font-size: 0.8rem;
}

button {
  margin: 0;
  padding: 0.3em 0.6em;
  background-color: green;
  color: white;
  border: none;
  cursor: pointer;
}

div#loaded {
  display: flex;
  margin: 0;
}

div#loaded > p {
  margin: 0;
  flex-grow: 1;
  text-align: left;
  padding: 0.3em 0.6em;
  border: 1px solid #444;
  border-right: none;
}

div#updates {
    display: flex;
    margin: 0;
    font-size: 0.9rem;
}

div#updates > p {
    margin: 0;
    padding: 0.4em 0.2em;
    flex-grow: 1;
    text-align: left;
}

p#update-info {
    font-size: 0.8rem;
    margin: 1em 0;
}

h2 {
    margin: 2em 0 0.1em 0.3em;
    padding: 0;
    font-size: 1.2rem;
    text-align: left;
}
</style>


{#if dictionary === null}

  <input type="file" bind:files>

  {#if status == "choosefile"}
    <p>Please choose a dictionary from your device.</p>
  {:else if (status == "error") || (status == "reading")}
    <p>{big_status_message}</p>
    {#if small_status_message}
      <p id="smallstatus">{small_status_message}</p>
    {/if}
  {/if}

{:else}
  <div id="loaded">
    <p>Current file: {dictionary.name}</p>
    <button on:click={removeDict}>Remove</button>
  </div>
{/if}

<h2>Updates</h2>
<div id="updates">
  {#if (update_info.update_available)}
    <p>Update available ({formatFilesize(update_info.update_size)})</p>
    <button on:click={event => navigator.serviceWorker.controller.postMessage("do-update")}>Update!</button>
  {:else}
    <p>Last checked: {
      update_info.date_checked?
	    date_formatter.format(update_info.date_checked)
    	    : "(unknown)"
    }
    </p>
                  
    <button on:click={event => navigator.serviceWorker.controller.postMessage("check-for-updates")}>Check</button>
  {/if}
</div>
{#if (update_info.serviceworker_info)}
  <p id="update-info">{update_info.serviceworker_info}</p>
{/if}
