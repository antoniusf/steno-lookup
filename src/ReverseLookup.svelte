<script>
    export let dictionary;
    let query = "";
    let query_result;

    $: doQuery(query)

    function doQuery(query) {
        query_result = dictionary.filter(([strokes, translation]) => (translation == query));
    }
</script>

<style>
    table {
      grid-area:main;
      width: 100%;
      border: none;
      border-collapse: collapse;
      text-align: center;
    }
    
    td {
      padding: 0.4em 0.2em;
    }
    
    tr:nth-child(odd) {
      background-color: #eee;
    }
    
    td.strokes {
      font-family: Courier, monospace;
      padding: 0;
    }
    
    input {
      grid-area: query;
      width: 100%;
      min-width: 0;
      border: 1px solid #444;
      padding: 0.3em 0.6em;
    }
</style>

<input type="text" bind:value={query}/>
<table>
  {#each query_result as [strokes, translation]}
    <tr><td class="strokes">{strokes}</td><td>{translation}</td></tr>
  {/each}
</table>
    
