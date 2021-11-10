<!-- This Source Code Form is subject to the terms of the Mozilla Public
   - License, v. 2.0. If a copy of the MPL was not distributed with this
   - file, You can obtain one at https://mozilla.org/MPL/2.0/. -->
<script>
    import StrokeDisplay from './StrokeDisplay.svelte';
    import ResultsTable from './ResultsTable.svelte';
    import { textToStroke, strokeToText, strokeListToPackedStrokes } from './util.js';

    export let dictionary;

    let strokes = [0];
    let current_text = "";
    let input_element;

    let results = [];
    let error_msg;

    // we have to do this manually, since bind and reactivity
    // do not support bi-directional data flow
    function onInput(event) {
	const new_value = event.target.value;

	// this check is necessary, since input will also fire
	// an event when we update its value from current_text.
	// if we didn't do this, we might get into an update dependency loop
	if (new_value == current_text) {
	    return;
	}

	current_text = new_value;
	strokes = current_text.toUpperCase().split("/").map(textToStroke);
	doQuery();
    }

    $: {
        current_text = strokes.map(strokeToText).join("/");
        if (input_element && input_element.value != current_text) {
            input_element.value = current_text;
        }
        doQuery();
    }

    async function doQuery() {
	error_msg = undefined;
	results = undefined;
	try {
	    results = dictionary.find_strokes(strokeListToPackedStrokes(strokes));
	}
	catch (e) {
	    results = undefined;
	    if (!e.message) {
	        error_msg = { message: e.toString() };
	    }
	    else {
		error_msg = e;
	    }
	}
    }
</script>

<style>
  input[type="text"] {
    width: 60%;
    margin: 0px auto 0.5em;
    min-width: 0;
    border: 1px solid #777;
    padding: 0.3em 0.6em;
  }

  p#smallerror {
    margin-top: 0.5em;
    font-size: 0.8rem;
  }
</style>

<!--pass through the entire strokes array (even if we'd only need the last entry)
    just because i don't know if binding would work if we passed an element-->
<StrokeDisplay bind:strokes/>

<input type="text" aria-labelledby="mode-label" on:input={onInput} bind:this={input_element} />
{#if results !== undefined}
  <ResultsTable results={results} />
{:else}
  {#if error_msg}
    <p id="bigerror">{error_msg.message}</p>
    {#if error_msg.details}
      <p id="smallerror">{error_msg.details}</p>
    {/if}
  {/if}
{/if}
