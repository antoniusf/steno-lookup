import { packedStrokesToText, textToStroke } from './util';

let text_decoder = new TextDecoder("utf-8");
let text_encoder = new TextEncoder("utf-8");

let global_module;

export async function initialize (dictionary_data = undefined) {

    // TODO: compileStreaming doesn't work on Safari, which is really annoying, because the
    // alternative (normal instantiate from fetch.arrayBuffer()) doesn't work on firefox for android
    // since I'm using the latter but not the former, I'm going to keep it like this for now,
    // but this'll definitely have to be fixed. ugh.

    let url = './helpers.wasm';

    if (!global_module) {
	global_module = WebAssembly.compileStreaming(fetch(url));
    }

    // instanciate the module as well, given the dictionary
    if (dictionary_data) {
	let instance = await instanciate(global_module);
	return prepare_instance_for_querying(instance, dictionary_data);
    }
}

// takes a promise for a module, returns a promise for an instance.
async function instanciate(module) {

    // there is a bit of a chicken-and-egg problem here, where we want the module import functions
    // to capture the modules memory, so there always in sync, but we need to provide the functions
    // at instanciation and we only get the memory back after that. so my idea was to declare
    // the memory locally and then re-set its value after instanciating the module and hope that
    // this works.

    let memory;

    // i'm going to try and do the same thing with the results array
    let results = [];
    
    function logErr (offset, length, line) {
	if (memory) {
	    const buffer = new Uint8Array(memory.buffer, offset, length);
	    console.log("WebAssembly module panicked with '" + text_decoder.decode(buffer) + "' on line " + line + "\nraw buffer: " + buffer);
	}
	else {
	    console.log("Warning: logErr got called, but memory was not initialized??");
	}
    }

    // strokes_offset is a ptr, strokes_length is in units of the contained type (ie 4 bytes)
    // handily, this is just how the constructor for Uint32Array works!
    function yield_result (string_offset, string_length, strokes_offset, strokes_length) {
	let string = text_decoder.decode(new Uint8Array(memory.buffer, string_offset, string_length));
	let strokes = new Uint8Array(memory.buffer, strokes_offset, strokes_length);
	results.push([packedStrokesToText(strokes), string]);
    }

    let instance = await WebAssembly.instantiate(await module, { env: { logErr: logErr, yield_result: yield_result }});

    // store reference to memory so that logErr (and yield_result) work
    memory = instance.exports.memory;

    return {instance: instance, results: results};
}

function prepare_instance_for_querying(instance_info, dictionary_data) {

    const wasm_page_size = 65536;

    let data_size = dictionary_data.length;

    let query_maxlength = 100;
    
    const size = data_size + query_maxlength;
    const pages_needed = Math.ceil(size / wasm_page_size);

    let instance = instance_info.instance;
    let results = instance_info.results;

    const num_base_pages = instance.exports.memory.grow(pages_needed);
    const base_offset = num_base_pages * wasm_page_size;
    const query_start = base_offset + data_size

    let wasm_data = new Uint8Array(instance.exports.memory.buffer, base_offset, data_size);
    wasm_data.set(dictionary_data);


    let data_start = base_offset;
    // define the two query functions here, so they can capture
    // all necessary variables and gain correct scoping automatically
    function lookup(query) {

	const start = performance.now();

	// limit length to 100 bytes, since that's how much is reserved
	const encoded_query = text_encoder.encode(query).subarray(0, 100);

	let wasm_query = new Uint8Array(instance.exports.memory.buffer, query_start, encoded_query.length);
	wasm_query.set(encoded_query);

	// clear results in place
	// this is necessary since it is captured by the yield_results function, so we can't reassign
	results.splice(0, results.length);
	instance.exports.query(query_start, encoded_query.length,
			    data_start,
			    0);
	console.log(`query took ${performance.now() - start}ms`);
	// make a copy, so that the caller can't accidentally mess with our data
	return results.slice();
    }

    function find_stroke(stroke) {

	// clear results in place
	// this is necessary since it is captured by the yield_results function, so we can't reassign
	results.splice(0, results.length);
	const start = performance.now();
	instance.exports.query(stroke, 0,
			    data_start,
			    1);
	console.log(`query took ${performance.now() - start}ms`);
	// make a copy, so that the caller can't accidentally mess with our data
	return results.slice();
    }

    // return wasm_data as well, so that the caller can store it if they want
    return { lookup: lookup, find_stroke: find_stroke, data: wasm_data };
}

export async function loadJson (json) {

    const wasm_page_size = 65536;

    // make sure we have a wasm module loaded
    initialize();
    
    const wasm = (await instanciate(global_module)).instance;
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

    const lengths = new Uint32Array(wasm.exports.memory.buffer, info_ptr, 4);
    const data_length = lengths[3];

    console.log(`data length: ${data_length}`);

    // use slice to create a copy of this array, so we can release the memory
    const data_array = new Uint8Array(wasm.exports.memory.buffer, info_ptr, data_length).slice();

    // [releasing the memory along with the wasm instance:]
    // since it is only referenced by the imported js functions,
    // which are stored along with the instance, it should get gc'd
    // automatically when our instance goes out of scope at the end of this
    // function.

    // convenience: load the new dictionary into a query-mode instance
    let dictionary = prepare_instance_for_querying(await instanciate(global_module), data_array);

    return dictionary;
}
