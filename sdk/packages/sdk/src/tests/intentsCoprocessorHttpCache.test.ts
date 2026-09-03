import http from "node:http"
import type { AddressInfo } from "node:net"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { HttpProvider, type ApiPromise } from "@polkadot/api"
import { IntentsCoprocessor } from "@/chains/intentsCoprocessor"

/**
 * polkadot-js's `HttpProvider` caches every request that names a block hash, and what it caches is
 * the request promise itself — a rejected one included — under a TTL that every hit refreshes. The
 * phantom order poll retries the block it failed on with identical parameters on every tick, so a
 * single reset connection turned into the same rejection replayed from memory for as long as the
 * process lived; the node never saw a second request. The coprocessor builds its HTTP provider with
 * that cache off. These pin both halves: the hazard in the dependency, so the workaround rests on a
 * check rather than a memory, and the provider the coprocessor actually builds being free of it.
 */

const BLOCK_HASH = `0x${"de".repeat(32)}`
const RUNTIME_VERSION = { specName: "nexus", specVersion: 1 }

vi.mock("@polkadot/api", async (importOriginal) => {
	const actual = await importOriginal<typeof import("@polkadot/api")>()

	/** Stands in for ApiPromise: ready without a handshake, keeping the provider it was built on. */
	class FakeApiPromise {
		provider: unknown
		isReadyOrError = Promise.resolve(this)
		disconnect = async () => {}
		constructor(options: { provider: unknown }) {
			this.provider = options.provider
		}
	}

	return { ...actual, ApiPromise: FakeApiPromise }
})

/**
 * A node that resets the connection on the first `resets` requests and answers every one after —
 * the failure seen in the field was a TCP RST mid-request, which fetch reports as a bare
 * "fetch failed".
 */
function stubNode(resets: number) {
	let resetsLeft = resets
	let answered = 0
	const server = http.createServer((req, res) => {
		let body = ""
		req.on("data", (chunk: Buffer) => {
			body += chunk
		})
		req.on("end", () => {
			const { id } = JSON.parse(body) as { id: number }
			if (resetsLeft > 0) {
				resetsLeft -= 1
				req.socket.resetAndDestroy()
				return
			}
			answered += 1
			res.setHeader("content-type", "application/json")
			res.end(JSON.stringify({ jsonrpc: "2.0", id, result: RUNTIME_VERSION }))
		})
	})

	return {
		/** Requests that reached the node and were answered. */
		answered: () => answered,
		listen: () =>
			new Promise<number>((resolve) =>
				server.listen(0, "127.0.0.1", () => resolve((server.address() as AddressInfo).port)),
			),
		close: () =>
			new Promise<void>((resolve, reject) => {
				server.closeAllConnections()
				server.close((err) => (err ? reject(err) : resolve()))
			}),
	}
}

describe("coprocessor HTTP provider cache", () => {
	let node: ReturnType<typeof stubNode>
	let port: number

	beforeEach(async () => {
		node = stubNode(1)
		port = await node.listen()
	})

	afterEach(() => node.close())

	// The dependency's own behaviour. If this starts failing, polkadot-js no longer caches
	// rejections, and the capacity override in `IntentsCoprocessor.http` can be reconsidered.
	it("polkadot-js HttpProvider replays a cached rejection instead of retrying", async () => {
		const provider = new HttpProvider(`http://127.0.0.1:${port}`)

		await expect(provider.send("state_getRuntimeVersion", [BLOCK_HASH], true)).rejects.toThrow("fetch failed")
		await expect(provider.send("state_getRuntimeVersion", [BLOCK_HASH], true)).rejects.toThrow("fetch failed")

		expect(node.answered()).toBe(0)
		expect(provider.stats.total.cached).toBe(1)
	})

	it("the coprocessor's HTTP api retries a read that failed rather than replaying the failure", async () => {
		// The HTTP endpoint is derived from the websocket's; that endpoint is all the coprocessor
		// reads from the ws api to build it.
		const wsApi = { _rpcCore: { provider: { endpoint: `ws://127.0.0.1:${port}` } } } as unknown as ApiPromise
		const coprocessor = IntentsCoprocessor.fromApi(wsApi)

		const { provider } = (await coprocessor.queryApi()) as unknown as { provider: HttpProvider }

		await expect(provider.send("state_getRuntimeVersion", [BLOCK_HASH], true)).rejects.toThrow("fetch failed")
		await expect(provider.send("state_getRuntimeVersion", [BLOCK_HASH], true)).resolves.toEqual(RUNTIME_VERSION)

		expect(node.answered()).toBe(1)
	})
})
