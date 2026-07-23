import { existsSync, readFileSync, statSync } from "node:fs"
import { extname, join, resolve } from "node:path"
import type { ServerResponse } from "node:http"

const MIME_TYPES: Record<string, string> = {
	".html": "text/html; charset=utf-8",
	".js": "text/javascript; charset=utf-8",
	".mjs": "text/javascript; charset=utf-8",
	".css": "text/css; charset=utf-8",
	".json": "application/json",
	".svg": "image/svg+xml",
	".png": "image/png",
	".ico": "image/x-icon",
	".map": "application/json",
	".woff2": "font/woff2",
	".txt": "text/plain; charset=utf-8",
}

/**
 * Serves a file from the built SPA directory, falling back to index.html for
 * client-routed paths. Returns false when nothing could be served (no dist dir
 * or missing index.html).
 */
export function serveStatic(res: ServerResponse, uiDistDir: string, urlPath: string): boolean {
	const root = resolve(uiDistDir)
	let decodedPath: string
	try {
		decodedPath = decodeURIComponent(urlPath)
	} catch {
		decodedPath = urlPath
	}
	const requested = resolve(join(root, decodedPath === "/" ? "index.html" : decodedPath))
	if (!requested.startsWith(root)) {
		res.writeHead(403, { "Content-Type": "text/plain" })
		res.end("Forbidden")
		return true
	}

	const target = existsSync(requested) && statSync(requested).isFile() ? requested : join(root, "index.html")
	if (!existsSync(target)) return false

	const ext = extname(target)
	// Vite content-hashes everything under /assets; index.html must revalidate.
	const cacheControl =
		requested.includes(`${root}/assets/`) && target === requested
			? "public, max-age=31536000, immutable"
			: "no-cache"
	res.writeHead(200, { "Content-Type": MIME_TYPES[ext] ?? "application/octet-stream", "Cache-Control": cacheControl })
	res.end(readFileSync(target))
	return true
}
