let text_decoder = new TextDecoder("utf-8");
let memory;

export async function loadWasm (url) {

    // TODO: export memory from wasm instead? initially I thought maybe this way I could initialize it,
    // but I think I can do that one way or the other? And right now, I have to do this weird thing
    // with getting the initial size right :/
    //
    // TODO 2: instantiateStreaming doesn't work on Safari, which is really annoying, because the
    // alternative (normal instantiate from fetch.arrayBuffer()) doesn't work on firefox for android
    // since I'm using the latter but not the former, I'm going to keep it like this for now,
    // but this'll definitely have to be fixed. ugh.

    function logErr (offset, length) {
	if (memory) {
	    const buffer = new Uint8Array(memory.buffer, offset, length);
	    console.log("WebAssembly module panicked with '" + text_decoder.decode(buffer) + "'");
	}
	else {
	    console.log("Warning: logErr got called, but memory was not initialized??");
	}
    }

    const start = performance.now();
    let result = await WebAssembly.instantiateStreaming(fetch(url), { env: { logErr: logErr }});
    console.log(`loading wasm took ${performance.now() - start}ms`);

    memory = result.instance.exports.memory;
    return result.instance;
}

export async function loadJson (json) {

    const wasm_page_size = 65536;
    
    const wasm = await loadWasm("/helpers.wasm");
    const encoder = new TextEncoder();
    const data = encoder.encode(json);

    const pages_needed = Math.ceil(data.length / wasm_page_size);

    const num_base_pages = wasm.exports.memory.grow(pages_needed);
    const base_offset = num_base_pages * 65536;
    console.log("number of base pages: " + num_base_pages);

    const memoryarray = new Uint8Array(wasm.exports.memory.buffer);

    memoryarray.subarray(base_offset, base_offset + data.length).set(data);

    console.log("before wasm");
    const start = performance.now()
    wasm.exports.load_json(base_offset, data.length);
    console.log(`after wasm (took ${performance.now() - start}ms)`);
    console.log(memoryarray.subarray(base_offset, base_offset + 20));
}
