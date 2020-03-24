<script>
  import { createEventDispatcher, onMount } from 'svelte';

  const dispatch = createEventDispatcher();

  let files = [];
  export let status = "initializing";
  let errormsg = "";
  let readprogress = 0;
  export let dictionary;

  onMount(() => {

      if (status == "initializing") {
        let stored_dictionary = window.localStorage.getItem("dictionary");
        if (stored_dictionary === null) {
            status = "choosefile";
        } else {
            dictionary = JSON.parse(stored_dictionary); // TODO: can this fail?
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
	      //readpromise = file.text().then(JSON.parse).then(finishReadFile);
	  }
      }
  }

  function finishReadFile (filereader) {
      let data;
      try {
          data = JSON.parse(filereader.result);
      } catch (error) {
          status = "error";
          errormsg = "Sorry, we can't read the file that you uploaded. Are you sure that it's an actual json dictionary? (" + e.name + ": " + e.message + ")";
          return;
      }
      dictionary = {name: files[0].name, data: Object.entries(data)};

      window.localStorage.setItem("dictionary", JSON.stringify(dictionary));

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
  grid-area: query;
}

p {
  grid-area: main;
  margin: 0;
}

div#loaded {
  grid-area: query;
  display: flex;
  margin: 0;
}

div#loaded > p {
  margin: 0;
  padding: 0;
  flex-grow: 1;
  text-align: left;
  padding: 0.3em 0.6em;
  border: 1px solid #444;
  border-right: none;
}

div#loaded > button {
  margin: 0;
  padding: 0.3em 0.6em;
  background-color: green;
  color: white;
  border: none;
  cursor: pointer;
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
