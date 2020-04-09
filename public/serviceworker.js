self.addEventListener('install', (event) => {
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
		'./helpers.wasm',
		'./load-icon.svg',
		'./abc-icon.svg',
		'./STK-icon.svg'
	    ]);
	})
    );
});

self.addEventListener('fetch', (event) => {
    event.respondWith(
	// Development
	fetch(event.request)
    );
});

self.addEventListener('message', (event) => {
    if (event.data == "getversion") {
	    event.source.postMessage("dev/friday3");
    }
});
