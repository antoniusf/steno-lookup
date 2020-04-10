import { strokesToText } from './util';

let text_decoder = new TextDecoder("utf-8");
let text_encoder = new TextEncoder("utf-8");

let global_module;
let global_instance;
let loaded_for_query = false;
let query_info;

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

export async function initialize (dictionary = undefined) {

    // TODO: compileStreaming doesn't work on Safari, which is really annoying, because the
    // alternative (normal instantiate from fetch.arrayBuffer()) doesn't work on firefox for android
    // since I'm using the latter but not the former, I'm going to keep it like this for now,
    // but this'll definitely have to be fixed. ugh.

    let url = './helpers.wasm';

    global_module = WebAssembly.compileStreaming(fetch(url));

    // instanciate the module as well, given the dictionary
    if (dictionary) {
	let instance = instanciate(global_module);
	global_instance = prepare_instance_for_querying(instance, dictionary);
    }
}

// takes a promise for a module, returns a promise for an instance.
// this is so we can chain everything nicely and immediately obtain
// a promise for global_instance, at least when using with prepare_instance_for_querying.
async function instanciate(module) {
    let instance = await WebAssembly.instantiate(await module, { env: { logErr: logErr, yield_result: yield_result }});

    // store reference to memory in global so that logErr (and yield_result) work
    memory = instance.exports.memory;
    // (NOTE: there should not be an active global_instance at this point, else logErr will get very confused)

    return instance;
}

async function prepare_instance_for_querying(instance, dictionary) {

    const wasm_page_size = 65536;

    let strokes_length = dictionary.strokes.length;
    let strokes_size = strokes_length * 4;

    let strings_length = dictionary.strings.length;
    let strings_size = strings_length;

    let query_maxlength = 100;
    
    const length = strokes_size + strings_size + query_maxlength;
    const pages_needed = Math.ceil(length / wasm_page_size);

    instance = await instance;
    const num_base_pages = instance.exports.memory.grow(pages_needed);
    const base_offset = num_base_pages * wasm_page_size;

    // put the stroke array first, so it's aligned (hopefully)
    let strokes_start = base_offset;
    let strings_start = strokes_start + strokes_size;
    let query_start = strings_start + strings_size;
    
    let wasm_strokes = new Uint32Array(instance.exports.memory.buffer, strokes_start, strokes_length);
    wasm_strokes.set(dictionary.strokes);
    let wasm_strings = new Uint8Array(instance.exports.memory.buffer, strings_start, strings_length);
    wasm_strings.set(dictionary.strings);

    // now that strings and strokes are stored in wasm memory, we don't need an extra copy in js!
    // we can just have the dictionary refer to the wasm version.
    // (since we don't realloc, this should be stable)
    dictionary.strokes = wasm_strokes;
    dictionary.strings = wasm_strings;

    // store necessary info in global state
    // in principle, we could also store this in wasm memory,
    // but i don't see the point. (also, it would be work since
    // globals in rust are always pointers.)
    query_info = {};
    query_info.strokes_start = strokes_start;
    query_info.strokes_length = strokes_length;
    query_info.strings_start = strings_start;
    query_info.strings_length = strings_length;
    query_info.query_start = query_start;

    // set loaded_for_query to signal that the instance can now be used for querying.
    loaded_for_query = true;

    return instance;
}

export async function loadJson (json) {

    const wasm_page_size = 65536;

    // make sure we have a wasm module loaded
    initialize();
    
    loaded_for_query = false;
    global_instance = Promise.resolve(undefined);
    const wasm = await instanciate(global_module);
    const data = text_encoder.encode(json);

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
    let dictionary = { strings: string_array, strokes: stroke_array };

    // release the module references to the memory and instance
    memory = undefined;

    // convenience: load the new dictionary into a query-mode instance
    global_instance = prepare_instance_for_querying(instanciate(global_module), dictionary);

    return dictionary;
}

export async function doQuery(dictionary, query) {

    // wait for the instance first, in case it's still being prepared
    let instance = await global_instance;
    if (!loaded_for_query) {
	console.log("Error: doQuery: there is currently now wasm module loaded for querying. Call prepare_instance_for_querying first.");
	return;
    }
    
    // limit length to 100 bytes, since that's how much is reserved
    const encoded_query = text_encoder.encode(query).subarray(0, 100);

    let wasm_query = new Uint8Array(instance.exports.memory.buffer, query_info.query_start, encoded_query.length);
    wasm_query.set(encoded_query);

    results = [];
    const start = performance.now();
    instance.exports.query(query_info.query_start, encoded_query.length,
		       query_info.strings_start, query_info.strings_length,
		       query_info.strokes_start, query_info.strokes_length);
    console.log(`query took ${performance.now() - start}ms`);
    return results;
}
