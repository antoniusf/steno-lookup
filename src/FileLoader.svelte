<script>
  // TODO: move updater into its own separate element
  import { createEventDispatcher, onMount } from 'svelte';
  import { set, get, del } from 'idb-keyval';
  import { textToStroke, strokeToText } from './util';
  import { initialize, loadJson } from './wasm-interface.js';

  const dispatch = createEventDispatcher();

  let files = [];
  export let status = "initializing";
  export let update_info;
  let errormsg = "";
  let readprogress = 0;
  export let dictionary;

  onMount(async () => {

      if (status == "initializing") {
        // TODO: move to indexedDB so we can store the prepared form of
        // the dictionary directly, and save the load time.
        // this should still be smaller than the json representation.
        let stored_dictionary = await get("dictionary");
        if (!stored_dictionary) {
            status = "choosefile";
        } else {
            // TODO: can this fail?
	    dictionary = stored_dictionary;
	    initialize(dictionary);
            status = "loaded";
            signalDone();
        }
      }
  });

  // TODO: make some kind of util file where I can put this?
  function formatFilesize (size) {

      if (size < 0) {
          throw RangeError("File size must be larger than 0!");
      }

      // first, choose prefix
      // (get order of magnitude in base 1000)
      let order_of_magnitude = Math.log10(size) / 3;

      // i'm adding a small constant, so that values very
      // close to the next multiple of thousand get rounded up
      // (displaying 1MB instead of 995kB)
      // again, i am dividing the result from log10 by 3,
      // since we're working in base 1000
      const rounding_correction = Math.log10(1000/995) / 3;
      order_of_magnitude += rounding_correction;
      
      // i'm also adding another constant because i want
      // 100 kB (or 99.5kB, because of our correction) to get shown as 0.1 MB.
      // this means i am adding 1 / 3., which is basically
      // the same as if i'd multiplied the original value by 10
      // before doing anything else
      order_of_magnitude += 1/3;

      // lastly, i'm rounding down to get our final classification.
      // everything below 99.5 goes to 0, etc.
      // (yes i know that we can't have half bytes, so it should really
      //  say 99, but for the higher orders it's going to be
      //  99.5 * 1000^n gets rounded to n
      order_of_magnitude = Math.floor(order_of_magnitude);

      // now we can actually compute the rounded mantissa!
      // (that's the part before the prefix)
      // round to one decimal digit
      const mantissa = Math.round(size / (1000**order_of_magnitude) * 10) / 10;

      const prefixes = ["", "k", "M", "G", "T", "P", "E"]
      if (order_of_magnitude < prefixes.length) {
          // \u2009 is a narrow no-break space
          return mantissa + "\u202F" + prefixes[order_of_magnitude] + "B";
      } else {
          // just in case someone passes in an unrealistically large value
          return mantissa + "e" + (3 * order_of_magnitude) + "\u202FB";
      }
  }
    

  $: readFile(files);

  function readFile (files) {
      if (files.length > 0) {
	  let file = files[0];
	  if (file.size > 10 * 2**20) {
	      status = "toobig";
	  } else if (file.type != "application/json") {
	      status = "wrongtype";
	  } else {
	      status = "reading";
              let filereader = new FileReader();
              filereader.addEventListener("load", event => finishReadFile(filereader));
              filereader.addEventListener("progress", event => { readprogress = Math.floor(event.loaded / event.total * 100); });
              filereader.addEventListener("abort", event => { status = "load-aborted"; });
              filereader.addEventListener("error", event => { status = "error"; errormsg = "Your browser failed to load the file. Please try again."});
              filereader.readAsText(file);
	  }
      }
  }

  async function finishReadFile (filereader) {
      try {
	  dictionary = await loadJson(filereader.result);
      } catch (error) {
          console.log(error);
          status = "error";
          errormsg = "Sorry, we can't read the file that you uploaded. Are you sure that it's an actual json dictionary? (" + error.name + ": " + error.message + ")";
          return;
      }

      dictionary.name = files[0].name;

      await set("dictionary", dictionary);

      status = "loaded";
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
  {:else if status == "toobig"}
    <p>Sorry, we only accept files up to 10MB currently.</p>
  {:else if status == "wrongtype"}
    <p>Sorry, right now we can only read plover json dictionaries.</p>
  {:else if status == "reading"}
    <p>Reading... {readprogress}%</p>
  {:else if status == "load-aborted"}
    <p>Aborted.</p>
  {:else if status == "load-error"}
    <p>Error: {errormsg}</p>
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
                  
    <button on:click={event => navigator.serviceWorker.controller.postMessage("checkforupdates")}>Check</button>
  {/if}
</div>
{#if (update_info.serviceworker_info)}
  <p id="update-info">{update_info.serviceworker_info}</p>
{/if}
