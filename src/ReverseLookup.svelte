<script>
    export let dictionary;
    let query = "";
    let query_result;
    let queryRunnerState = {
	running: false, // if running is true, there is still a doQueryInChunks call in the message queue
	chunk: 0,
	query: null
    };
    const itemsPerChunk = 1000;

    // perf
    let startQueryTime = null;

    $: startQuery(query)

    function startQuery(query) {
	queryRunnerState.chunk = 0;
	queryRunnerState.query = query;
	query_result = [];

	if (queryRunnerState.running === false) {
	    // enqueue the query runner task
	    // (if running was true, there would already be a task enqueued, which will just start again
	    // now that we've reset it's state)
	    window.setTimeout(doQueryInChunks, 0);
            queryRunnerState.running = true;
	}

	startQueryTime = performance.now();
    }

    function doQueryInChunks() {
	// console.log("query \"" + queryRunnerState.query + "\", chunk " + queryRunnerState.chunk);
	let startIndex = queryRunnerState.chunk * itemsPerChunk;
	let endIndex = startIndex + itemsPerChunk;
	endIndex = Math.min(endIndex, dictionary.length);
	
	for (let i=startIndex; i<endIndex; i++) {
	    let entry = dictionary[i];
	    if (entry[1] == queryRunnerState.query) {
		query_result.push(entry);
	    }
	}

	queryRunnerState.chunk += 1;

	if (endIndex === dictionary.length) {
	    queryRunnerState.running = false;

	    // re-assign query_result so svelte updates the view
	    // we're only doing this now to save unnecessary updates while loading.
	    // though we should consider if displaying the data as it streams in might be desirable.
	    query_result = query_result;
	    console.log("Query took " + (performance.now() - startQueryTime) + "ms.");
	} else {
	    // re-enqueue this function at the back of the message queue.
	    // if there were input events in the meantime, they will get handled first.
	    window.setTimeout(doQueryInChunks, 0);
	}
    }

    // function doQuery(query) {
    //     query_result = dictionary.filter(([strokes, translation]) => (translation == query));
    // }
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
    
