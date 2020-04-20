<script>
    import ResultsTable from './ResultsTable.svelte';
    import { strokeToText } from './util.js';
    
    export let dictionary;
    let query = "";
    let query_result;
    const itemsPerChunk = 10000;

    $: doQuery(query)

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
