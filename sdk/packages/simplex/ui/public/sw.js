const CACHE_NAME = "simplex-shell-v7"
const PRECACHE_URLS = [
	"./",
	"./index.html",
	"./manifest.webmanifest",
	"./icons/mobile-logo.svg",
	"./icons/simplex-192.png",
	"./icons/simplex-512.png",
]

self.addEventListener("install", (event) => {
	event.waitUntil(precacheAppShell().then(() => self.skipWaiting()))
})

self.addEventListener("activate", (event) => {
	event.waitUntil(
		caches
			.keys()
			.then((keys) => {
				const staleKeys = []
				for (const key of keys) {
					if (key.startsWith("simplex-shell-") && key !== CACHE_NAME) staleKeys.push(key)
				}
				return Promise.all(staleKeys.map((key) => caches.delete(key)))
			})
			.then(() => self.clients.claim()),
	)
})

async function precacheAppShell() {
	const cache = await caches.open(CACHE_NAME)
	await cache.addAll(PRECACHE_URLS)

	// Vite fingerprints the production JS and CSS filenames. Discover them from
	// the built HTML during install so the very first offline launch has the
	// complete shell, rather than relying on a second online page load.
	const response = await fetch("./index.html", { cache: "no-store" })
	const html = await response.text()
	const entryUrls = Array.from(html.matchAll(/(?:src|href)=["']([^"']+)["']/g), (match) => match[1]).filter(
		(path) => path.startsWith("./assets/"),
	)
	await cache.addAll(entryUrls)
}

self.addEventListener("fetch", (event) => {
	if (event.request.method !== "GET") return
	const requestUrl = new URL(event.request.url)
	if (requestUrl.origin !== self.location.origin) return
	// API responses contain live balances, status, and operator data. They must
	// never become stale offline cache entries.
	if (requestUrl.pathname === "/api" || requestUrl.pathname.startsWith("/api/") || requestUrl.pathname === "/health") return

	if (event.request.mode === "navigate") {
		event.respondWith(
			fetch(event.request)
				.then((response) => {
					const copy = response.clone()
					void caches.open(CACHE_NAME).then((cache) => cache.put("./index.html", copy))
					return response
				})
				.catch(() => caches.match("./index.html", { ignoreVary: true })),
		)
		return
	}

	event.respondWith(
		caches.match(event.request, { ignoreVary: true }).then((cached) => {
			if (cached) return cached
			return fetch(event.request).then((response) => {
				if (response.ok) {
					const copy = response.clone()
					void caches.open(CACHE_NAME).then((cache) => cache.put(event.request, copy))
				}
				return response
			})
		}),
	)
})

self.addEventListener("push", (event) => {
	let notification = {
		title: "Simplex alert",
		body: "Open Simplex for details.",
		tag: "simplex-alert",
		url: "./",
	}
	try {
		if (event.data) notification = { ...notification, ...event.data.json() }
	} catch {
		// A malformed payload still tells the operator that Simplex needs attention.
	}
	event.waitUntil(
		self.registration.showNotification(notification.title, {
			body: notification.body,
			tag: notification.tag,
			icon: "./icons/simplex-192.png",
			badge: "./icons/simplex-192.png",
			data: { url: notification.url },
		}),
	)
})

self.addEventListener("notificationclick", (event) => {
	event.notification.close()
	const target = new URL(event.notification.data?.url ?? "./", self.registration.scope).href
	event.waitUntil(
		self.clients.matchAll({ type: "window", includeUncontrolled: true }).then(async (windows) => {
			for (const client of windows) {
				if (new URL(client.url).origin === self.location.origin) {
					await client.navigate(target)
					return client.focus()
				}
			}
			return self.clients.openWindow(target)
		}),
	)
})
