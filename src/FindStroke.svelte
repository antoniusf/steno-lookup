<script>
    import StrokeDisplay from './StrokeDisplay.svelte';
    import ResultsTable from './ResultsTable.svelte';
    import { textToStroke, strokeToText } from './util.js';

    export let dictionary;

    let stroke = 0;
    let current_text = "";
    let input_element;

    let results;
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
	stroke = textToStroke(current_text.toUpperCase());
	doQuery();
    }

    function onStrokeChanged(event) {
	stroke = event.detail.stroke;

	// update the text input
	// TODO: diffing to preserve consonant clusters etc?
	current_text = strokeToText(stroke);
	input_element.value = current_text;
	doQuery();
    }

    async function doQuery() {
	error_msg = undefined;
	try {
	    results = dictionary.find_stroke(stroke);
	}
	catch (e) {
	    results = undefined;
	    error_msg = e;
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
</style>

<StrokeDisplay bind:stroke on:strokeChanged={onStrokeChanged}/>

<input type="text" on:input={onInput} bind:this={input_element} />
{#if results}
  <ResultsTable results={results} />
{:else}
  {#if error_msg}
    <p>{error_msg}</p>
  {/if}
{/if}
