<script>
  // TODO: move updater into its own separate element
  import { createEventDispatcher, onMount } from 'svelte';
  import { textToStroke, strokeToText } from './util';

  const dispatch = createEventDispatcher();

  let files = [];
  export let status = "initializing";
  export let update_info;
  let errormsg = "";
  let readprogress = 0;
  export let dictionary;

  onMount(() => {

      if (status == "initializing") {
        let stored_dictionary = window.localStorage.getItem("dictionary");
        if (stored_dictionary === null) {
            status = "choosefile";
        } else {
            // TODO: can this fail?
	    const filename = window.localStorage.getItem("dictionary-name");
            dictionary = loadJson(filename, stored_dictionary); 
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

  function loadJson (filename, json) {
      // TODO: better error handling, instead of just letting the errors bubble up
      const data = JSON.parse(json);
      let dictionary = { name: filename };

      // the total number of strokes across all definitions, not excluding
      // duplicates. this will determine the length of our final stroke array.
      let total_num_strokes = 0;
      let num_entries = 0;
      for (const [strokes_text, translation] of Object.entries(data)) {
	  total_num_strokes += strokes_text.split("/").length;
	  num_entries += 1;
      }

      dictionary.packed_strokes = new Uint32Array(total_num_strokes);
      // we're adding the length of the array as the final index, since
      // this simplifies access
      dictionary.packed_stroke_indices = new Uint32Array(num_entries + 1);
      dictionary.translations = [];
      let packed_strokes_index = 0;
      let max_delta = 0;

      dictionary.by_stroke = new Map();

      for (const [index, [strokes_text, translation]] of Object.entries(data).entries()) {

	  dictionary.packed_stroke_indices[index] = packed_strokes_index;
	  dictionary.translations.push(translation);
	  
	  for (const stroke_text of strokes_text.split("/")) {
	      const stroke = textToStroke(stroke_text);
	      
	      dictionary.packed_strokes[packed_strokes_index] = stroke;
	      packed_strokes_index += 1;

	      const by_stroke_defs = dictionary.by_stroke.get(stroke);
	      if (by_stroke_defs) {
		  by_stroke_defs.push(index);
	      }
	      else {
		  dictionary.by_stroke.set(stroke, []);
	      }
	  }
	  const delta = packed_strokes_index - dictionary.packed_stroke_indices[index-0];
	  if (delta > max_delta) {
	      max_delta = delta;
	      console.log(`delta: ${delta} (at ${strokes_text} ${translation}`);
	  }
      }

      // packed_strokes_index should be equal to total_num_strokes here
      dictionary.packed_stroke_indices[num_entries] = packed_strokes_index;

      // provice a convenience access function
      dictionary.getEntry = function (index) {
	  const strokes = this.packed_strokes.slice(
	      this.packed_stroke_indices[index],
	      this.packed_stroke_indices[index+1]
	  );

	  // map doesn't work here, since it apparently returns another uint8array,
	  // and not a normal array of strings
	  const stroke_texts = [];
	  for (const stroke of strokes) {
	      stroke_texts.push(strokeToText(stroke));
	  }
	  return [stroke_texts.join("/"), this.translations[index]];
      };

      return dictionary;
  }

  function finishReadFile (filereader) {
      let data;
      try {
	  dictionary = loadJson(files[0], filereader.result);
      } catch (error) {
          status = "error";
          errormsg = "Sorry, we can't read the file that you uploaded. Are you sure that it's an actual json dictionary? (" + error.name + ": " + error.message + ")";
          return;
      }

      window.localStorage.setItem("dictionary", filereader.result);
      window.localStorage.setItem("dictionary-name", files[0]);

      status = "loaded";
      signalDone();
  }

  function removeDict (event) {
      dictionary = null;
      window.localStorage.removeItem("dictionary");
      status = "choosefile";
  }

  function signalDone () {
      dispatch("message", {
	  status: "done"
      });
  }
    
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
    <p>Last checked: {Intl.DateTimeFormat("en-US", {year: "numeric", month:"long", day:"numeric", hour:"numeric", minute:"numeric"}).format(update_info.date_checked)}</p>
    <button on:click={event => navigator.serviceWorker.controller.postMessage("checkforupdates")}>Check</button>
  {/if}
</div>
