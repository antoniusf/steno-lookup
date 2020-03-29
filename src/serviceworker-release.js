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
    const version = await store.get("local-version");
    // if (version) {
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
	    name: "last-checked",
	    value: date
	});
	await store.put({
	    name: "upstream-version",
	    value: upstream_version
	});
	const local_version = await store.get("local-version");

	notifyClients({ type: "update-info", update_available: (local_version == upstream_version), date_checked: date });

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
    const local_version = (await db.get("general", "local-version")).value;

    console.log(`local_version: ${local_version}`);
    console.log(`upstream_version: ${upstream_version}`);

    if (local_version == upstream_version) {
	return;
    }

    console.log("mismatch, updating.")

    // we're requesting all resources first, to make sure that
    // they're all reachable. then we'll update them all at once.
    let add_to_cache = [];
    let keep = []

    for (const upstream_file of upstream_versioninfo.files) {

	const url = upstream_file.url;
	const local_file = await db.get("fileversions", url)
	if (local_file) {
	    console.log(`${url} / local hash: ${local_file.version} / upstream hash: ${upstream_file.version}`);
	    if (upstream_file.version == local_file.version) {
		// we don't need to update this file
		console.log(`${url} hashes match, no update necessary.`);
		keep.push(url);
		continue;
	    }
	}
	
	const response = await fetch(upstream_file.version + "/" + url);
	if (!response.ok) {
	    // abort
	    // TODO: handling
	    throw new TypeError("installNewestVersion: couldn't retrieve newest version of file");
	}

	add_to_cache.push({ url: url, response: response, version: upstream_file.version });
    }

    console.log("add_to_cache: " + JSON.stringify(add_to_cache, null, 2));
    console.log("keep: " + JSON.stringify(keep, null, 2));

    let cache = await caches.open('v1');

    // turns out, awaiting cache will also close a transaction.
    // this means that we either have to give up on the idea of
    // using a single transaction (which may be unnecessary anyways,
    // since the important property is that cache and db match), or
    // splitting the cache and db access into two separate loops.
    // the second idea is more work, and would probably not be useful
    // due to the above reason, so I'm going with the first.
    const local_file_versioninfos = await db.getAll("fileversions");
    for (const { url } of local_file_versioninfos) {
	if (!keep.includes(url)) {
	    console.log(`deleting file ${url} from cache`);
	    const success = await cache.delete(url);
	    await db.delete("fileversions", url);
	    if (!success) {
		console.log("installNewestVersion: mismatch between idb and cache, couldn't delete old version from cache (" + url + ")");
	    }
	}
    }
    console.log("old stuff deleted.");

    for (const { url, version, response } of add_to_cache) {
	await cache.put(url, response);
	await db.put("fileversions", { url: url, version: version });
    }

    await db.put("general", { name: "local-version", value: upstream_version });
}

async function notifyClients (message) {
    const clients = await clients.matchAll({ type: "window" });
    console.log("notifyClients: posting message " + message + " to " + clients.length + " clients.");
    for (const client of clients) {
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
	event.source.postMessage("friday-fullversion0.10");
    }
    if (event.data == "checkforupdates") {
	checkForUpdates().catch(error => console.log("Error in checkForUpdates: " + error + " (" + error.fileName + ":" + error.lineNumber + ")"));
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
