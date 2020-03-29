import { openDB, deleteDB, wrap, unwrap } from 'idb';

self.addEventListener('install', (event) => {
    event.waitUntil(setup().catch(error => console.log("Error in setup: " + error + " (" + error.fileName + ":" + error.lineNumber + ")")));
});

async function setup() {
    const db = await openDB("data", 1, {
	upgrade(db, oldVersion, newVersion, transaction) {
	    db.createObjectStore("fileversions", { keyPath: "url" });
	}
    });

    const store = db.transaction("fileversions").objectStore("fileversions");
    const version = store.get("<app-version>");
    if (version) {
	await checkForUpdates();
    } else {
	await installNewestVersion();
    }
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

	let version_data = await driveParserFromReader(readVersionData(), reader);
	console.log("version info parsed: " + JSON.stringify(version_data, null, 2));
    } else {
	console.log("retrieving version file failed.");
    }
}

async function installNewestVersion() {

    // check one last time to make sure we're getting the most up-to-date version
    await checkForUpdates();

    let cache = await caches.open('v1');
    await cache.addAll([
	'./', // add the base URL so it handles this correctly
	'./index.html',
	'./global.css',
	'./favicon.png',
	'./app.webmanifest', // cache the manifest and icon,
	'./icon.png', // in case people decide they want to "add to homescreen" while offline
	'./build/bundle.css',
	'./build/bundle.js',
	'./load-icon.svg',
	'./abc-icon.svg',
	'./STK-icon.svg'
    ]);
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
	event.source.postMessage("friday-rollup1.3");
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
    record.filename = yield* readString();
    record.hash = yield* readString();
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
		return { records: records, apphash: apphash }; 
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
