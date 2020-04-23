<script>
    import ResultsTable from './ResultsTable.svelte';
    
    export let dictionary;
    let query = "";
    let query_result;
    let error_msg;

    $: doQuery(query)
    function doQuery(query) {
	error_msg = undefined;
	try {
	    query_result = dictionary.lookup(query);
	}
	catch (e) {
	    query_result = undefined;
	    error_msg = e;
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
</style>

<input type="text" bind:value={query}/>
{#if query_result}
  <ResultsTable results={query_result}/>
{:else}
  {#if error_msg}
    <p>{error_msg}</p>
  {:else}
    <p>query running...</p>
  {/if}
{/if}
