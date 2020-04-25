<!-- This Source Code Form is subject to the terms of the Mozilla Public
   - License, v. 2.0. If a copy of the MPL was not distributed with this
   - file, You can obtain one at https://mozilla.org/MPL/2.0/. -->
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

    p {
      padding: 0;
      margin: 0.1em 0 0.4em;
    }
</style>

{#if results.length == 0}
  <p>(no results)</p>
{:else}
  <table>
    {#each results as [strokes, translation]}
      <tr><td class="strokes">{strokes}</td><td>{translation}</td></tr>
    {/each}
  </table>
{/if}