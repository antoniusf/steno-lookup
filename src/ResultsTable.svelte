<script>
//TODO: paging?
    export let results = [];
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
      padding: 0.4em;
    }
    
    tr:nth-child(odd) {
      background-color: #eee;
    }
    
    td.strokes {
      font-family: Courier, monospace;
    }
</style>

<table>
  {#each results as [strokes, translation]}
    <tr><td class="strokes">{strokes}</td><td>{translation}</td></tr>
  {/each}
</table>