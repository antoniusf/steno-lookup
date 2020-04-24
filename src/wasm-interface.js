/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

import { packedStrokesToText, textToStroke } from './util';

let text_decoder = new TextDecoder("utf-8");
let text_encoder = new TextEncoder("utf-8");

let global_module;

export async function initialize (dictionary_data = undefined) {

    let url = './helpers.wasm';

    if (!global_module) {
	// safari doesn't support compileStreaming, so I *hope* that this switch works
	if (WebAssembly.compileStreaming) {
	    global_module = WebAssembly.compileStreaming(fetch(url))
	        .catch(error => {
		    throw `Oh no, there was a problem with loading the WebAssembly module. This is not good, since we can't do anything without that. (${error})`
		});
	}
	else {
	    // but I also don't want to use this as standard,
	    // (a) because compileStreaming is recommended, and
	    // (b) because response.arrayBuffer doesn't work in
	    //     firefox mobile yet, so I need a switch anyways
	    global_module = fetch(url)
	        .catch(error => { throw `Couldn't load the wasm file for some reason. Sorry, this shouldn't happen. (${error})`; })
		.then(response => response.arrayBuffer())
	        .then(bytes => WebAssembly.compile(bytes))
	        .catch(error => {
		    if (error instanceof TypeError) {
			throw `Sorry, it seems like your browser does not support WebAssembly. We need this to make your queries fast while using as little of your precious RAM as possible. WebAssembly should be supported in the newest versions of all major browsers, except for Internet Explorer. (${error})`
		    } else {
			throw error;
		    }
		});
	}
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
    let last_error;
    
    function logErr (message_offset, message_length, details_offset, details_length, line) {
	if (memory) {
	    const message_buffer = new Uint8Array(memory.buffer, message_offset, message_length);
	    const details_buffer = new Uint8Array(memory.buffer, details_offset, details_length);

	    const message = text_decoder.decode(message_buffer);
	    const details = text_decoder.decode(details_buffer);
	    
	    console.log(`WebAssembly module panicked with '${message} (${details})' on line ${line}`);
	    last_error = { message: message, details: details };
	}
	else {
	    console.log("Warning: logErr got called, but memory was not initialized??");
	}
    }

    function get_last_error () {
	return last_error;
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

    return {instance: instance, results: results, get_last_error: get_last_error};
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
	try {
	    instance.exports.query(query_start, encoded_query.length,
				data_start,
				0);
	}
	catch (e) {
	    let last_error = instance_info.get_last_error();
	    if (last_error) {
		throw last_error;
	    }
	    else {
		throw `Error in WebAssembly module: ${e} (this probably shouldn't have happened)`;
	    }
	}
	    
	console.log(`query took ${performance.now() - start}ms`);
	// make a copy, so that the caller can't accidentally mess with our data
	return results.slice();
    }

    function find_stroke(stroke) {

	// clear results in place
	// this is necessary since it is captured by the yield_results function, so we can't reassign
	results.splice(0, results.length);
	const start = performance.now();
	try {
	    instance.exports.query(stroke, 0,
				data_start,
				1);
	}
	catch (e) {
	    let last_error = instance_info.get_last_error();
	    if (last_error) {
		throw last_error;
	    }
	    else {
		throw `Error in WebAssembly module: ${e} (this probably shouldn't have happened)`;
	    }
	}
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
    
    const instance_info = await instanciate(global_module);
    const wasm = instance_info.instance;
    const data = text_encoder.encode(json);

    const pages_needed = Math.ceil(data.length / wasm_page_size);

    const num_base_pages = wasm.exports.memory.grow(pages_needed);
    const base_offset = num_base_pages * 65536;
    console.log("number of base pages: " + num_base_pages);

    let memoryarray = new Uint8Array(wasm.exports.memory.buffer);

    memoryarray.subarray(base_offset, base_offset + data.length).set(data);

    console.log("before wasm");
    const start = performance.now();
    let info_ptr;
    try {
	info_ptr = wasm.exports.load_json(base_offset, data.length);
    }
    // TODO: unify error handling
    catch (e) {
	let last_error = instance_info.get_last_error();
	if (last_error) {
	    throw last_error;
	}
	else {
	    throw `Error in WebAssembly module: ${e} (this probably shouldn't have happened)`;
	}
    }
    console.log(`after wasm (took ${performance.now() - start}ms)`);

    // this matches the Header struct in lib.rs
    const lengths = new Uint32Array(wasm.exports.memory.buffer, info_ptr, 5);
    const data_length = lengths[4];

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
