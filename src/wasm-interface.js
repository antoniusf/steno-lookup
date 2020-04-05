let text_decoder = new TextDecoder("utf-8");

export async function loadWasm (url) {

    // TODO: export memory from wasm instead? initially I thought maybe this way I could initialize it,
    // but I think I can do that one way or the other? And right now, I have to do this weird thing
    // with getting the initial size right :/
    //
    // TODO 2: instantiateStreaming doesn't work on Safari, which is really annoying, because the
    // alternative (normal instantiate from fetch.arrayBuffer()) doesn't work on firefox for android
    // since I'm using the latter but not the former, I'm going to keep it like this for now,
    // but this'll definitely have to be fixed. ugh.

    let memory = new WebAssembly.Memory({ initial: 17 });

    function logErr (offset, length) {
	const buffer = new Uint8Array(memory.buffer, offset, length);
	console.log("WebAssembly module panicked with '" + text_decoder.decode(buffer) + "'");
    }


    const start = performance.now();
    let result = await WebAssembly.instantiateStreaming(fetch(url), { env: { memory: memory, logErr: logErr }});
    console.log(`loading wasm took ${performance.now() - start}ms`);
    return { memory: memory, instance: result.instance };
}

export async function testWasm () {

    const wasm = await loadWasm("/helpers.wasm");
    // grow by 1 page
    const num_base_pages = wasm.memory.grow(1);
    const base_offset = num_base_pages * 65536;
    console.log("number of base pages: " + num_base_pages);

    let array = new Uint8Array(wasm.memory.buffer, base_offset, 10);

    for (let i = 0; i < 10; i++) {
	array[i] = i+1;
    }

    console.log("wasm result: " + wasm.instance.exports.load_json(base_offset, 10));
}
