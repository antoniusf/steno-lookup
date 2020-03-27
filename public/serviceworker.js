self.addEventListener('install', (event) => {
    event.waitUntil(
	caches.open('v1').then((cache) => {
	    return cache.addAll([
		'./', // add the base URL so it handles this correctly
		'./index.html',
		'./global.css',
		'./favicon.png',
		'./build/bundle.css',
		'./build/bundle.js'
	    ]);
	})
    );
});

self.addEventListener('fetch', (event) => {
    event.respondWith(
	caches.match(event.request).then((response) => {
	    if (response) {
		return response;
	    } else {
		response = new Response("Not Found: Your browser tried to request a resource that was not packaged with this app. This might be a bug in the app, but it really shouldn't happen.", {status: 404, statusText: "Not Found"});
		//return response;
		return fetch(event.request);
	    }
	})
    );
});
