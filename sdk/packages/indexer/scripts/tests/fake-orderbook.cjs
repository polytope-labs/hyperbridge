// A stand-in for the HyperFX orderbook's `GET /solvers`, for the solver-inventory E2E.
//
// It serves the watchlist the Hyperbridge node polls, in the shape the real server publishes
// (`crates/server/src/app.rs` in hyperfx-orderbook): `{ chains: [{ chain, solvers: [{ address }] }] }`,
// with an ETag over the body and `304` for a matching `If-None-Match`. The indexer's parser
// (`solverWatchlist.service.ts`) accepts exactly that, so this exercises the real contract rather
// than a shape invented for the test.
//
// Run: node scripts/tests/fake-orderbook.cjs [--port 8547]
const http = require("node:http")
const { createHash } = require("node:crypto")

const { CHAIN, SOLVERS } = require("./solver-fixtures.cjs")

const port = Number(process.env.ORDERBOOK_PORT || argValue("--port") || 8547)
/** Chains the real server would have configured; one with no live orders is an empty list. */
const CONFIGURED = [CHAIN, "EVM-42161"]

function argValue(flag) {
	const index = process.argv.indexOf(flag)
	return index === -1 ? undefined : process.argv[index + 1]
}

function watchlist(chain) {
	const chains = (chain ? [chain] : CONFIGURED).map((name) => ({
		chain: name,
		// Sorted, as the real server sorts for a stable diff.
		solvers: name === CHAIN ? SOLVERS.map((solver) => ({ address: solver.address })).sort(byAddress) : [],
	}))
	return JSON.stringify({ chains })
}

const byAddress = (a, b) => (a.address < b.address ? -1 : a.address > b.address ? 1 : 0)

/** The real server's tag is a hash of the response body, so an unchanged list costs a comparison. */
const etagOf = (body) => `"${createHash("sha256").update(body).digest("hex").slice(0, 32)}"`

let polls = 0

const server = http.createServer((request, response) => {
	const url = new URL(request.url, `http://127.0.0.1:${port}`)
	if (url.pathname === "/healthz") {
		response.writeHead(200, { "content-type": "text/plain" }).end("ok")
		return
	}
	if (url.pathname !== "/solvers") {
		response.writeHead(404).end()
		return
	}

	const chain = url.searchParams.get("chain")
	if (chain && !CONFIGURED.includes(chain)) {
		response.writeHead(400, { "content-type": "text/plain" }).end(`chain ${chain} is not configured`)
		return
	}

	const body = watchlist(chain)
	const etag = etagOf(body)
	polls += 1
	if (request.headers["if-none-match"] === etag) {
		console.log(`[fake-orderbook] poll ${polls}: 304, list unchanged`)
		response.writeHead(304, { etag }).end()
		return
	}
	console.log(`[fake-orderbook] poll ${polls}: 200, ${JSON.parse(body).chains.length} chains`)
	response.writeHead(200, { "content-type": "application/json", etag }).end(body)
})

server.listen(port, "127.0.0.1", () => {
	console.log(`[fake-orderbook] GET http://127.0.0.1:${port}/solvers`)
	console.log(`[fake-orderbook] publishing ${SOLVERS.length} solvers on ${CHAIN}`)
})

for (const signal of ["SIGINT", "SIGTERM"]) {
	process.on(signal, () => {
		console.log(`[fake-orderbook] answered ${polls} polls`)
		server.close(() => process.exit(0))
	})
}
