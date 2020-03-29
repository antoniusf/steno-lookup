self.addEventListener('install', (event) => {
    event.waitUntil(setup());
});

async function setup() {
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
    return null;
}

self.addEventListener('fetch', (event) => {
    event.respondWith(
	// Development
	// fetch(event.request)
	// normal use
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
	event.source.postMessage("friday4.6");
    }
    if (event.data == "checkforupdates") {
	fetch("./version", {cache: "no-cache"}).then((response) => {
	    if (response.ok) {
		console.log("response okay");
		let readResponse = async (response) => {
		    console.log("in readResponse.");
		    let reader = response.body.getReader();
		    let buffer = new Uint8Array(32); // we're hard-limiting this to 32 bytes, that should be enough and it saves me the work of resizing
		    let pos = 0;
		    while (true) {
			let { value: chunk, done: done } = await reader.read();

			if (done) {
			    break;
			}

			let remaining_space = buffer.length - pos;
			if (chunk.length > remaining_space) {
			    // the next chunk would overflow our buffer.
			    // theoretically, we could just cut off the chunk here
			    // however, this is kind of unclean,
			    // and it may also create problems if the received string
			    // is utf-8, since we may cut off in the middle of a
			    // continuation byte.
			    //
			    // in short: we're just going to error when this happens.
			    console.log("error: version response too long!");
			    return undefined;
			}
			
			buffer.set(chunk, pos);
			pos += chunk.length;
		    }

		    // we should get plain ascii from the server, so we
		    // don't have to worry about cutting of continuation bytes
		    // by hard-limiting our buffer to 32 bytes
		    let decoder = new TextDecoder();
		    text = decoder.decode(buffer.subarray(0, pos));
		    console.log(text);
		};
		readResponse(response).catch(error => console.log("Error in readResponse: " + error + " (" + error.fileName + ":" + error.lineNumber + ")"));
	    }
	});
    }
});
