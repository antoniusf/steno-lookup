export async function loadWasm (url) {

    let memory = new WebAssembly.Memory({ initial: 16 });
    let result = await WebAssembly.instantiateStreaming(fetch(url), { env: { memory: memory }});
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
