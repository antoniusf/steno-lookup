self.addEventListener('install', (event) => {
    const start = performance.now();
    event.waitUntil(
	caches.open('v1').then((cache) => {
	    return cache.addAll([
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
	}).then(() => {
	    console.log("perf: install took " + (performance.now() - start) + "ms");
	})
    );
});

self.addEventListener('fetch', (event) => {
    const start = performance.now();
    event.respondWith((async (event) => {
	// Development
	const response = await caches.match(event.request);
	console.log("perf: fetch took " + (performance.now() - start) + "ms");
	return response;
    })(event));
});

self.addEventListener('message', (event) => {
    if (event.data == "getversion") {
	    event.source.postMessage("dev/friday3");
    }
});
