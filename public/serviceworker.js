self.addEventListener('install', (event) => {
    event.waitUntil(
	caches.open('v1').then((cache) => {
	    return cache.addAll([
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
	    return response || fetch(event.request);
	})
    );
});
