// TODO: clear out unused caches?
//       i mean, this should happen normally in installNewestVersion()
//       but if that crashes we may accumulate caches we don't need

import { get, set } from 'idb-keyval';

let cached_local_version;

async function getActiveCache() {
    if (!cached_local_version) {
	console.log("perf: retrieving active cache name from db");
	cached_local_version = await get("local-version");
    }
    const active_cache = await caches.open("v1-" + cached_local_version);
    return active_cache;
}

// no, we cannot store this message in the cache, since we need to be able
// to display it precisely when the cache may have failed.
const urlNotInCacheMessage = `<!DOCTYPE html>
<html>
<head>
<title>File not found</title>
</head>
<body>
<p>The resource you requested was not in our cache. This could be for one of three reasons:</p>
<ul>
<li>The resource is part of the app, but we forgot to cache it. In this case, it would be nice if you filed an issue.</li>
<li>The resource is part of the app, but the browser deleted it from our cache. In this case, please try <a href="./reinstall">re-downloading all relevant files</a>.</li>
<li>The resource is not part of the app, but you or your browser decided to request it anyway. In this case, there is nothing we can do.</li>
</ul>
</body>
</html>`

self.addEventListener('install', (event) => {
    event.waitUntil(setup().catch(error => console.log("Error in setup: " + error + " (" + error.fileName + ":" + error.lineNumber + ")")));
});

async function setup() {

    const version = await get("local-version");
    if (version) {
    	await checkForUpdates();
    } else {
    	await installNewestVersion();
    }

    // NOTE: we're only installing if there is currently no version installed.
    //       updating the app when the service worker updates creates more problems
    //       than it solves:
    //        (a) we have to defer deleting the old cache, since the old sw might
    //            still be using it
    //        (b) we also can't set local-version, since if the user initiates an
    //            update via the old sw, things will get confusing
    //        (c) when we introduce binary format versioning, we want to warn the user
    //            explicitly that updating to the new version will mean that they will
    //            have to reload their dictionary. this would circumvent that. (note
    //            that there's still a way around this, which is the emergency /reinstall
    //            link. so we'll obviously still have to check the format when loading a
    //            dictionary from local storage, but this should still avoid most instances
    //            of annoying behavior.
}


async function checkForUpdates() {

    let start = performance.now();
    const response = await fetch("./version", {cache: "no-cache"});
    let end = performance.now();
    console.log(`perf: checkForUpdates: fetch took ${end-start}ms`);
    start = end;

    if (response.ok) {
	console.log("reading version info...");
	let reader = response.body.getReader();

	// I'm a little bit sorry for using a binary format here,
	// but given that we only get uint8's from our reader,
	// it's just the easiest thing here.
	//
	// And the push-parser functions were just the most elegant way
	// I could think of, for handling arbitrary chunk boundaries
	// without having to allocate a big Uint8Array where we don't
	// even know the initial size.
	//
	// Also, it's really fun to see that generators are so powerful.

	const version_info = await driveParserFromReader(readVersionData(), reader);
	console.log("version info parsed: " + JSON.stringify(version_info, null, 2));
	end = performance.now();
	console.log(`perf: checkForUpdates: parsing took ${end-start}ms`);
	start = end;

	const upstream_version = version_info.version;
        const local_version = await get("local-version");
	const date = new Date();
        const update_available = (local_version != upstream_version);

        await set("date-checked", date);
        await set("update-available", update_available);
        await set("update-size", version_info.size);

	console.log("local version: " + local_version + " / upstream version " + upstream_version)
	await notifyClients({
            type: "update-info",
            status: (update_available)? "available":"up-to-date",
            date_checked: date,
            update_size: version_info.size
        });

	end = performance.now();
	console.log(`perf: checkForUpdates: updating db and notifying clients took ${end-start}ms`);
	start = end;

	return version_info;
    } else {
	console.log("retrieving version file failed.");
    }
}

async function installNewestVersion() {

    const start = performance.now();

    // check one last time to make sure we're getting the most up-to-date version
    await notifyClients({
	type: "serviceworker-info",
	text: "Updating version info..."
    });
    const upstream_versioninfo = await checkForUpdates();

    const upstream_version = upstream_versioninfo.version;
    const local_version = await get("local-version");

    console.log(`local_version: ${local_version}`);
    console.log(`upstream_version: ${upstream_version}`);

    if (local_version == upstream_version) {
        console.log("versions match, we're all good.")
	await notifyClients({
	    type: "serviceworker-info",
	    text: "Versions match nothing to update..."
	});
	return;
    }

    console.log("mismatch, updating.")

    await notifyClients({
	type: "serviceworker-info",
	text: "Downloading files..."
    });

    // open a new cache for the new version,
    // so we can replace everything cleanly
    const new_cache_name = 'v1-' + upstream_version;
    const old_cache_name = 'v1-' + local_version;
    let new_cache = await caches.open(new_cache_name);

    // TODO: make sure we cache versioned files??
    // is this important?
    await new_cache.addAll(upstream_versioninfo.files);

    console.log("installNewestVersion: new cache's keys: " + await new_cache.keys());

    await notifyClients({
	type: "serviceworker-info",
	text: "Cleaning up..."
    });

    await set("local-version", upstream_version);
    cached_local_version = upstream_version;

    // now that indexeddb is successfully set to the new version, we can delete the old cache
    if (await caches.delete(old_cache_name)) {
	console.log("old cache deleted.");
    } else {
	console.log("coulnd't find old cache??");
    }

    console.log("perf: installNewestVersion took " + (performance.now() - start) + "ms");

    await notifyClients({
	type: "serviceworker-info",
	text: "Done! Your page should reload now."
    });

    notifyClients({ type: "update-info", status: "installed", last_checked: new Date() });
}

async function notifyClients (message) {
    const all_clients = await clients.matchAll({ type: "window" });
    console.log("notifyClients: posting message " + message + " to " + all_clients.length + " clients.");
    for (const client of all_clients) {
	client.postMessage(message);
    }
}

self.addEventListener('fetch', (event) => {
    // use an iife so I can write async code
    event.respondWith((async (event) => {
	const start = performance.now();
	const cache = await getActiveCache();
        const response = await cache.match(event.request);
	// const response = await caches.match(event.request);

        if (response) {
	    const end = performance.now();
	    console.log("fast path perf: " + (end - start));
            return response;
        } else {
	    const path = new URL(event.request.url).pathname;
	    const scope_path = new URL(registration.scope).pathname;
            console.log(`file ${path} not in cache`);

	    if (path == scope_path) {
		// try retrieving the index
		const response = await cache.match("/index.html");
		if (response) {
		    return response;
		}
	    }
	    // just check for the final component, for simplicity
	    else if (path.split("/").slice(-1)[0] == "reinstall") {
		// delete the local cache and set version to undefined
		const cache_name = "v1-" + (await get("local-version"));
		console.log(`deleting cache ${cache_name}`);
		console.log(await caches.delete(cache_name));
		await set("local-version", undefined);
		await installNewestVersion();
		const headers = new Headers({ "Location": scope_path + "index.html" });
		const response = new Response("Done! Please go back to the main page now.", {status: 307, statusText: "Redirecting back home", headers: headers});
		console.log("sent response with headers");
		return response;
	    }
	    // TODO: check if the resource should have been cached, so our error message can be more helpful?
	    const headers = new Headers({ "Content-Type": "text/html" });
	    const fail_response = new Response(urlNotInCacheMessage, {status: 404, statusText: "Not Found", headers: headers});
	    console.log("Warning: tried to fetch unpackaged resource: " + event.request.url);
            return fail_response;
        }
    })(event).catch((error) => {
        console.log("cache lookup error: " + error + " (" + error.fileName + ":" + error.lineNumber + ")");
    }));
});

self.addEventListener('message', async (event) => {
    if (event.data == "getversion") {
	event.source.postMessage({
            type: "version-info",
            serviceworker_version: "friday-lite-0.12"
        });
    }
    else if (event.data == "checkforupdates") {
	console.log("hello");
	event.source.postMessage({
	    type: "serviceworker-info",
	    text: "Checking..."
	});

	try {
	    await checkForUpdates();
	    event.source.postMessage({
		type: "serviceworker-info",
		text: ""
	    });
	}
	catch(error) {
	    event.source.postMessage({
		type: "serviceworker-info",
		text: `Sorry, couldn't check for updates. (Error: ${error})`
	    });
	    console.log("Error in checkForUpdates: " + error + " (" + error.fileName + ":" + error.lineNumber + ")");
	}
    }
    else if (event.data == "do-update") {
	try {
	    await installNewestVersion();
	}
	catch (error) {
	    // this needs to be a notifyClients, since all clients get updates
	    // from inside of the function
	    notifyClients({
		type: "serviceworker-info",
		text: `Sorry, update failed. (Error: ${error})`
	    });
            console.log("Error in installNewestVersion: " + error + " (" + error.fileName + ":" + error.lineNumber + ")");
        }
    }
    else if (event.data == "get-update-info") {
        const update_available = await get("update-available");
	event.source.postMessage({
            type: "update-info",
            status: update_available? "available":"up-to-date",
            date_checked: await get("date-checked"),
            update_size: await get("update-size")
        });
    }
});


// push-parser-style decoding functions
// we use little-endian by default
function* readUint16 () {
    const byte1 = yield;
    const byte2 = yield;
    return byte1 | (byte2 << 8);
}

function* readUint32 () {
    const byte1 = yield;
    const byte2 = yield;
    const byte3 = yield;
    const byte4 = yield;
    return byte1 | (byte2 << 8) | (byte3 << 16) | (byte4 << 24);
}

function* readString() {
    const string_length = yield* readUint16();
    //console.log("readString: reading string of length " + string_length);
    let buffer = new Uint8Array(string_length);
    for (let i=0; i < buffer.length; i++) {
	buffer[i] = yield;
    }

    let decoder = new TextDecoder();
    const result = decoder.decode(buffer);
    //console.log("readString: string is \"" + result + "\"");
    return result;
}

function* readVersionData() {
    const version = yield* readString();
    const size = yield* readUint32();

    const num_files = yield* readUint16();
    console.log("reading " + num_files + " files");
    let files = []
    for (let i = 0; i < num_files; i++) {
	files.push(yield* readString());
    }
    
    return { version: version, size: size, files: files };
}

async function driveParserFromReader(parser, reader) {

    // run parser till the first yield
    parser.next()
    
    while (true) {
	let { value: chunk, done: done } = await reader.read();

	if (done) {
	    parser.throw("driveParserFromReader: endOfStream");
	}

	//console.log("driveParserFromReader: reading chunk: " + chunk);
	for (let byte of chunk) {
	    let parser_state = parser.next(byte);
	    if (parser_state.done) {
		//console.log("driveParserFromReader: parser has signaled done.");
		return parser_state.value;
	    }
	}
    }
}
