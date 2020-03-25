<script>
    export let dictionary;
    let query = "";
    let query_result;
    let queryRunnerState = {
	running: false, // if running is true, there is still a doQueryInChunks call in the message queue
	chunk: 0,
	query: null
    };
    const itemsPerChunk = 10000;

    // perf
    let startQueryTime = null;

    $: startQuery(query)

    // I feel like I owe you a quick explanation of what is going on
    // here. What you have to know about this app is that my development
    // target is my 8 year old Galaxy S2, and I want the app to run well
    // on that.
    //
    // If I just do the simple thing and let the whole query run through
    // each time the input changes, it slows down the UI noticably when
    // keys are pressed in quick succession (again, on my phone). This is
    // because, though quite fast, it still takes a small amount of time
    // to search through all entries at once, and since this happens every
    // time you press a key, it can add up. Now the obvious solution might
    // be to add a web worker, but remember that I am working with a
    // really slow phone and want to avoid as much overhead as possible.
    //
    // So the solution that I chose was chunking the query into subqueries
    // that are small enough they don't block the UI, and return control
    // to the event loop in between chunks, so it can process any new
    // events. If we get a change event while the old query is still
    // going, we simply abort it and start running the new query! This is
    // basically cooperative multi-tasking on the main thread. I like this
    // solution because it has low technical complexity, no background
    // jobs that can crash my phone, and it's fast!
    //
    // My first idea for this was the simple setTimeout with a timeout
    // value of 0, since that does exactly what we need: queue up our
    // function on the (macro) task queue, ie. the main event
    // loop. However, as MDN helpfully tells us, the timeout function is
    // throttled on modern browsers, meaning that it spends lots of time
    // doing nothing before querying the next chunk. (See:
    // https://developer.mozilla.org/en-US/docs/Web/API/WindowOrWorkerGlobalScope/setTimeout#Timeouts_throttled_to_%E2%89%A5_4ms)
    // However, MDN also has the solution: Register your function as an
    // onMessage handler on the window, and use postMessage to enqueue
    // it. So that's what I'm doing.
    function startQuery(query) {
	queryRunnerState.chunk = 0;
	queryRunnerState.query = query;
	query_result = [];

	if (queryRunnerState.running === false) {
	    // enqueue the query runner task
	    // (if running was true, there would already be a task enqueued, which will just start again
	    // now that we've reset it's state)
	    window.postMessage("wakeQueryRunner", "*");
            queryRunnerState.running = true;
	}

	startQueryTime = performance.now();
    }

    function doQueryInChunks(event) {
	if (event.source == window && event.data == "wakeQueryRunner") {
	    event.stopPropagation();
	} else {
	    return;
	}

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
	    window.postMessage("wakeQueryRunner", "*");
	}
    }

    // function doQuery(query) {
    //     query_result = dictionary.filter(([strokes, translation]) => (translation == query));
    // }
</script>

<!--Register our query runner in the window's message handler. There's probably a more elegant way to do this so it doesn't risk interfering with anything else, but for now this should work.-->
<svelte:window on:message={doQueryInChunks}/>

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
    
