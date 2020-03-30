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
	const upstream_version = version_data.app_version;
	const date = new Date();
	console.log("version info parsed: " + JSON.stringify(version_data, null, 2));

	const db = await getDB();
	const store = db.transaction("general", "readwrite").objectStore("general");
	await store.put({
	    name: "upstream-version",
	    value: upstream_version
	});
	const local_version_object = await store.get("local-version");
	const local_version = local_version_object ? local_version_object.value : undefined;

	console.log("local version: " + local_version + " / upstream version " + upstream_version)
	await notifyClients({ type: "update-info", update_available: (local_version != upstream_version), date_checked: date });

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
    const local_version_object = await db.get("general", "local-version");
    const local_version = local_version_object ? local_version_object.value : undefined;

    console.log(`local_version: ${local_version}`);
    console.log(`upstream_version: ${upstream_version}`);

    if (local_version == upstream_version) {
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
	
	const response = await fetch(upstream_file.version + "/" + url);
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

    notifyClients({ type: "update-info", update_available: false, last_checked: new Date() });
}

async function notifyClients (message) {
    const all_clients = await clients.matchAll({ type: "window" });
    console.log("notifyClients: posting message " + message + " to " + all_clients.length + " clients.");
    for (const client of all_clients) {
	client.postMessage(message);
    }
}

self.addEventListener('fetch', (event) => {
    event.respondWith(
	caches.match(event.request).then((response) => {
	    if (response) {
		return response;
	    } else {
		response = new Response("Not Found: Your browser tried to request a resource that was not packaged with this app. This might be a bug in the app, but it really shouldn't happen.", {status: 404, statusText: "Not Found"});
		console.log("Warning: tried to fetch unpackaged resource: " + event.request.url);
		return response;
	    }
	}).catch((error) => console.log("cache lookup error: " + error + " (" + error.fileName + ":" + error.lineNumber + ")"))
    );
});

self.addEventListener('message', async (event) => {
    if (event.data == "getversion") {
	event.source.postMessage({ type: "version-info", serviceworker_version: "friday-fullversion0.12" });
    }
    else if (event.data == "checkforupdates") {
	checkForUpdates().catch(error => console.log("Error in checkForUpdates: " + error + " (" + error.fileName + ":" + error.lineNumber + ")"));
    }
    else if (event.data == "do-update") {
	installNewestVersion().catch(error => console.log("Error in installNewestVersion: " + error + " (" + error.fileName + ":" + error.lineNumber + ")"));
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
