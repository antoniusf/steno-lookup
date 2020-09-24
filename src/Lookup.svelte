<!-- This Source Code Form is subject to the terms of the Mozilla Public
   - License, v. 2.0. If a copy of the MPL was not distributed with this
   - file, You can obtain one at https://mozilla.org/MPL/2.0/. -->
<script>
    import ResultsTable from './ResultsTable.svelte';
    
    export let dictionary;
    let query = "";
    let query_result = [];
    let error_msg;

    $: doQuery(query)
    function doQuery(query) {
	error_msg = undefined;
	query_result = undefined;
	try {
	    query_result = dictionary.lookup(query);
	    console.log("asdf " + query_result);
	}
	catch (e) {
	    query_result = undefined;
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
    input {
      grid-area: query;
      width: 100%;
      min-width: 0;
      border: 1px solid #444;
      padding: 0.3em 0.6em;
    }

    p {
      margin: 0;
      padding: 0;
    }

    p#smallerror {
      margin-top: 0.5em;
      font-size: 0.8rem;
    }
    
</style>

<input id="query-input" type="text" aria-labelledby="mode-label" bind:value={query}/>
{#if query_result !== undefined}
  <ResultsTable results={query_result}/>
{:else}
  {#if error_msg}
    <p id="bigerror">{error_msg.message}</p>
    {#if error_msg.details}
      <p id="smallerror">{error_msg.details}</p>
    {/if}
  {/if}
{/if}
