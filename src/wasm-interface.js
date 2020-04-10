import { strokesToText } from './util';

let text_decoder = new TextDecoder("utf-8");
let memory;
let results;


function logErr (offset, length) {
    if (memory) {
	const buffer = new Uint8Array(memory.buffer, offset, length);
	console.log("WebAssembly module panicked with '" + text_decoder.decode(buffer) + "'");
    }
    else {
	console.log("Warning: logErr got called, but memory was not initialized??");
    }
}

// strokes_offset is a ptr, strokes_length is in units of the contained type (ie 4 bytes)
// handily, this is just how the constructor for Uint32Array works!
function yield_result (string_offset, string_length, strokes_offset, strokes_length) {
    let string = text_decoder.decode(new Uint8Array(memory.buffer, string_offset, string_length));
    let strokes = new Uint32Array(memory.buffer, strokes_offset, strokes_length);
    results.push([strokesToText(strokes), string]);
}

export async function loadWasm (url) {

    // TODO: export memory from wasm instead? initially I thought maybe this way I could initialize it,
    // but I think I can do that one way or the other? And right now, I have to do this weird thing
    // with getting the initial size right :/
    //
    // TODO 2: instantiateStreaming doesn't work on Safari, which is really annoying, because the
    // alternative (normal instantiate from fetch.arrayBuffer()) doesn't work on firefox for android
    // since I'm using the latter but not the former, I'm going to keep it like this for now,
    // but this'll definitely have to be fixed. ugh.

    const start = performance.now();
    let result = await WebAssembly.instantiateStreaming(fetch(url), { env: { logErr: logErr, yield_result: yield_result }});
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

    let memoryarray = new Uint8Array(wasm.exports.memory.buffer);

    memoryarray.subarray(base_offset, base_offset + data.length).set(data);

    console.log("before wasm");
    const start = performance.now()
    const info_ptr = wasm.exports.load_json(base_offset, data.length);
    console.log(`after wasm (took ${performance.now() - start}ms)`);

    const lengths = new Uint32Array(wasm.exports.memory.buffer, info_ptr, 8);
    const string_array_length = lengths[0];
    const stroke_array_length = lengths[1];

    console.log(`array lengths: ${string_array_length}, ${stroke_array_length}`);

    // use slice to create a copy of these arrays, so we can release the memory
    const string_array = new Uint8Array(wasm.exports.memory.buffer, base_offset, string_array_length).slice();
    const stroke_array = new Uint32Array(wasm.exports.memory.buffer, info_ptr + 8, stroke_array_length/4).slice();

    // release the module reference to the memory
    memory = undefined;

    return { strings: string_array, strokes: stroke_array };
}

export async function doQuery(dictionary, query) {
    // TODO: cache TextEncoder
    // TODO: the way this should actually work is that we initialize the wasm only once,
    // copy our data into its memory, and then delete our own copy of that data.
    // (replacing it with references to the wasm memory)
    // TODO: consistent naming (maybe length for number of elements, and size for number of bytes??)
    
    const wasm_page_size = 65536;
    
    const wasm = await loadWasm("/helpers.wasm");
    const encoder = new TextEncoder();
    const encoded_query = encoder.encode(query);
    const length = dictionary.strokes.length * 4 + dictionary.strings.length + encoded_query.length;

    const pages_needed = Math.ceil(length / wasm_page_size);

    const num_base_pages = wasm.exports.memory.grow(pages_needed);
    const base_offset = num_base_pages * 65536;

    // put the stroke array first, so it's aligned (hopefully)
    let strokes_start = base_offset;
    let strokes_length = dictionary.strokes.length;
    let strokes_size = strokes_length * 4;
    let strings_start = strokes_start + strokes_size;
    let strings_length = dictionary.strings.length;
    let query_start = strings_start + strings_length;
    
    let wasm_strokes = new Uint32Array(wasm.exports.memory.buffer, strokes_start, strokes_length);
    wasm_strokes.set(dictionary.strokes);
    let wasm_strings = new Uint8Array(wasm.exports.memory.buffer, strings_start, strings_length);
    wasm_strings.set(dictionary.strings);
    let wasm_query = new Uint8Array(wasm.exports.memory.buffer, query_start, encoded_query.length);
    wasm_query.set(encoded_query);

    results = [];
    const start = performance.now();
    wasm.exports.query(query_start, encoded_query.length, strings_start, strings_length, strokes_start, strokes_length);
    console.log(`query took ${performance.now() - start}ms`);
    return results;
}
