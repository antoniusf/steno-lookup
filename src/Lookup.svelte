<script>
    import ResultsTable from './ResultsTable.svelte';
    
    export let dictionary;
    let query = "";
    let query_result;

    $: doQuery(query)
    function doQuery(query) {
	query_result = dictionary.lookup(query);
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
</style>

<input type="text" bind:value={query}/>
{#if query_result}
<ResultsTable results={query_result}/>
{:else}
<p>query running...</p>
{/if}
