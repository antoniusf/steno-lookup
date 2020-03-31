// TODO: clear out unused caches?
//       i mean, this should happen normally in installNewestVersion()
//       but if that crashes we may accumulate caches we don't need

import { openDB } from 'idb';

let db;

// opens a new connection if necessary
// analogous to https://github.com/jakearchibald/svgomg/blob/2082499381882eacf9836d7f891cf348edb65404/src/js/utils/storage.js#L5
function getDB() {

    if (!db) {
	db = openDB("data", 1, {
	    upgrade(db, oldVersion, newVersion, transaction) {
		db.createObjectStore("fileversions", { keyPath: "url" });
		db.createObjectStore("general", { keyPath: "name" });
	    }
	});
    }

    return db;
}

let cached_local_version;

async function getActiveCache() {
    if (!cached_local_version) {
	cached_local_version = await getDBValue("local-version");
    }
    const active_cache = await caches.open("v1-" + cached_local_version);
    return active_cache;
}

// retrieves a value from the databases "general" key-value store
async function getDBValue (name) {
    const db = await getDB();
    const result = await db.get("general", name);
    if (result) {
        return result.value;
    } else {
        return undefined;
    }
}

self.addEventListener('install', (event) => {
    event.waitUntil(setup().catch(error => console.log("Error in setup: " + error + " (" + error.fileName + ":" + error.lineNumber + ")")));
});

async function setup() {

    const db = await getDB();
    const store = db.transaction("general").objectStore("general");
    const version_object = await store.get("local-version");
    // if (version_object) {
    // 	await checkForUpdates();
    // } else {
    // 	await installNewestVersion();
    // }

    //TESTING:
    await installNewestVersion();
}

async function checkForUpdates() {

    const response = await fetch("./version", {cache: "no-cache"});
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

	const version_data = await driveParserFromReader(readVersionData(), reader);
	console.log("version info parsed: " + JSON.stringify(version_data, null, 2));

	const db = await getDB();

	const upstream_version = version_data.app_version;
        const local_version = await getDBValue("local-version");
	const date = new Date();
        const update_available = (local_version != upstream_version);

        let update_size = 0; // in bytes
        if (update_available) {
            // check the version data against indexedDB, so we can compute the size of the update
            // if there was no update, the size just stays zero
            const store = db.transaction("fileversions").objectStore("fileversions");
            
            for (const record of version_data.files) {
                const local_record = await store.get(record.url);
                if (local_record) {
                    if (local_record.version != record.version) {
                        update_size += record.filesize;
                    }
                } else {
                    // file not stored
                    update_size += record.filesize;
                }
            }
        }

	const store = db.transaction("general", "readwrite").objectStore("general");
        await store.put({
            name: "date-checked",
            value: date
        });
        await store.put({
            name: "update-available",
            value: update_available
        });
        await store.put({
            name: "update_size",
            value: update_size
        });

	console.log("local version: " + local_version + " / upstream version " + upstream_version)
	await notifyClients({
            type: "update-info",
            status: (update_available)? "available":"up-to-date",
            date_checked: date,
            update_size: update_size
        });

	return version_data;
    } else {
	console.log("retrieving version file failed.");
    }
}

async function installNewestVersion() {

    // check one last time to make sure we're getting the most up-to-date version
    const upstream_versioninfo = await checkForUpdates();

    const db = await getDB();

    const upstream_version = upstream_versioninfo.app_version;
    const local_version = await getDBValue("local-version");

    console.log(`local_version: ${local_version}`);
    console.log(`upstream_version: ${upstream_version}`);

    if (local_version == upstream_version) {
        console.log("versions match, we're all good.")
	return;
    }

    console.log("mismatch, updating.")

    // we're requesting all resources first, to make sure that
    // they're all reachable. then we'll update them all at once.
    // let add_to_cache = [];
    // let keep = []

    // open a new cache for the new version,
    // so we can replace everything cleanly
    const new_cache_name = 'v1-' + upstream_version;
    const old_cache_name = 'v1-' + local_version;
    let new_cache = await caches.open(new_cache_name);
    let old_cache = await caches.open(old_cache_name);

    // we'll be keeping track of these, then store them
    // in a single transaction. (we can't do this directly,
    // since awaiting fetch and cache will close a transaction)
    let new_versions = [];

    for (const upstream_file of upstream_versioninfo.files) {

	const url = upstream_file.url;
	const local_file = await db.get("fileversions", url)
	console.log(`file ${url}`);
	if (local_file) {
	    console.log(`  local hash: ${local_file.version}\n  upstream hash: ${upstream_file.version}`);

	    if (upstream_file.version == local_file.version) {

		// we don't need to update this file
		console.log("  hashes match, copying version from old cache.");
		const response = await old_cache.match(url);

		if (response) {
		    await new_cache.put(url, response);
		    continue;
		}
		else {
		    console.log("  whoops, apparently this version wasn't in the old cache!! so we're fetching it again");
		}
	    }
	    else {
		console.log("  hashes don't match, so we're requesting the new version.");
	    }
	} else {
	    console.log("file not in local cache, fetching");
	}
	
	const response = await fetch("versioned/" + upstream_file.version + "/" + url);
	if (!response.ok) {
	    // abort
	    // TODO: handling
	    throw new TypeError("installNewestVersion: couldn't retrieve newest version of file");
	}

	new_versions.push({ url: url, version: upstream_file.version });
	await new_cache.put(url, response);
    }

    console.log("new_versions: " + JSON.stringify(new_versions, null, 2));
    console.log("new cache's keys: " + await new_cache.keys());

    const transaction = db.transaction(["general", "fileversions"], "readwrite");
    const fileversion_store = transaction.objectStore("fileversions");
    const general_store = transaction.objectStore("general");

    for (const new_version_record of new_versions) {
	await fileversion_store.put({
	    url: new_version_record.url,
	    version: new_version_record.version
	});
    }

    await general_store.put({ name: "local-version", value: upstream_version });
    cached_local_version = upstream_version;

    console.log("indexedDB updated");

    // note that we're updating the local version number in the same transaction as
    // everything else, so they're always in sync. what's more, at this point
    // we still have both caches, one with the old and one with the new version,
    // so everything is nice and clean. ("local-version" decides, which cache gets used.)

    // now that indexeddb is successfully set to the new version, we can delete the old cache
    if (await caches.delete(old_cache_name)) {
	console.log("old cache deleted.");
    } else {
	console.log("coulnd't find old cache??");
    }

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
            console.log(`file ${event.request.url} not in cache`);
            // our urls include only the path, but with an initial ./
            // (maybe I should change this, I don't even know why)
            const url = "." + new URL(event.request.url).pathname;

            // check if we should actually have a copy of this resource
            // (and it was evicted from cache or something)
            const db = await getDB();
            const file_record = await db.get("fileversions", url);

            if (file_record) {
                console.log("... but it should be. Refetching.");
                
                // try to re-fetch our version of that resource
                const response = await fetch("versioned/" + file_record.version + "/" + url);
                if (response.ok) {
                    console.log("refetch successful.");
                    
                    // get app_version so we can open the cache
                    const app_version = await getDBValue("local-version");

                    // put in cache
                    const cache = await caches.open('v1-' + app_version);
                    // note: we can't use the request object here, since that has the
                    // extra version on it!
                    // we also have to clone the response, so that it doesn't get used up.
                    // we still have to return it!
                    await cache.put(file_record.url, response.clone());
                    return response;
                } else {
                    // we're not getting a version from the server, so we probably have to do
                    // a full refresh
                    console.log("refetch failed. trying to re-install completely");
                    await installNewestVersion();
                    console.log("trying to get file from cache after re-install");
                    const response = await caches.match(event.request);
                    if (response) {
                        console.log("success!");
                        return response;
                    } else {
                        console.log("... okay, that didn't work, that's not good.");
                        const fail_response = new Response("This version of the file should be in the cache, but it wasn't. I tried re-installing the app to fix this, but something just went really wrong and it's still not here. I'm sorry.", {status: 500, statusText: "Internal Error"});
                        return fail_response;
                    }
                }
            } else {
                // okay, this file wasn't in cache, and according to our database, it shouldn't be.
                const fail_response = new Response("Not Found: Your browser tried to request a resource that was not packaged with this app. This might be a bug in the app, but it really shouldn't happen.", {status: 404, statusText: "Not Found"});
                console.log("Warning: tried to fetch unpackaged resource: " + event.request.url);
                return fail_response;
            }
        }
    })(event).catch((error) => {
        console.log("cache lookup error: " + error + " (" + error.fileName + ":" + error.lineNumber + ")");
    }));
});

self.addEventListener('message', async (event) => {
    if (event.data == "getversion") {
	event.source.postMessage({
            type: "version-info",
            serviceworker_version: "friday-fullversion0.14"
        });
    }
    else if (event.data == "checkforupdates") {
	checkForUpdates().catch((error) => {
            console.log("Error in checkForUpdates: " + error + " (" + error.fileName + ":" + error.lineNumber + ")");
        });
    }
    else if (event.data == "do-update") {
	installNewestVersion().catch((error) => {
            console.log("Error in installNewestVersion: " + error + " (" + error.fileName + ":" + error.lineNumber + ")");
        });
    }
    else if (event.data == "get-update-info") {
        const update_available = await getDBValue("update-available");
	event.source.postMessage({
            type: "update-info",
            status: update_available? "available":"up-to-date",
            date_checked: await getDBValue("date-checked"),
            update_size: await getDBValue("update-size")
        });
    }
});


// push-parser-style decoding functions
// we use little-endian by default
function* readUint16 () {
    const byte1 = yield;
    const byte2 = yield;
    console.log("readUint16: " + byte1 + ", " + byte2)
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
    console.log("readString: length = " + string_length);
    let buffer = new Uint8Array(string_length);
    for (let i=0; i < buffer.length; i++) {
	buffer[i] = yield;
    }

    console.log("readString: buffer = " + buffer);
    let decoder = new TextDecoder();
    return decoder.decode(buffer);
}

function* readFileRecord() {
    let record = {};
    record.url = yield* readString();
    record.version = yield* readString();
    record.filesize = yield* readUint32();
    console.log("readFileRecord: " + JSON.stringify(record, null, 2));

    return record;
}

function* readVersionData() {
    let apphash = yield* readString();
    let records = [];
    while (true) {
	try {
	    records.push(yield* readFileRecord());
	} catch (e) {
	    if (e == 'endOfStream') {
		return { files: records, app_version: apphash }; 
	    } else {
		throw e;
	    }
	}
    }
}

async function driveParserFromReader(parser, reader) {

    // run parser till the first yield
    parser.next()
    
    while (true) {
	console.log("driveParserFromReader: reading new chunk");
	let { value: chunk, done: done } = await reader.read();

	if (done) {
	    console.log("driveParserFromReader: chunk done, throwing");
	    // our stream is done, but the parser hasn't signaled
	    // done-ness yet.
	    let parser_state = parser.throw("endOfStream");
	    if (parser_state.done) {
		return parser_state.value;
	    } else {
		return undefined;
	    }
	}

	console.log("driveParserFromReader: reading chunk: " + chunk);
	for (let byte of chunk) {
	    let parser_state = parser.next(byte);
	    if (parser_state.done) {
		console.log("driveParserFromReader: parser has signaled done.");
		return parser_state.value;
	    }
	}
    }
}
