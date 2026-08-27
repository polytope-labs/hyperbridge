import http from "node:http"
import type { AddressInfo } from "node:net"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import type { ApiPromise } from "@polkadot/api"
import { IntentsCoprocessor } from "@/chains/intentsCoprocessor"

/**
 * Hyperbridge's public endpoints police the instantaneous request rate, and every read the
 * coprocessor makes arrives in bursts: a poll tick fires its whole block range back-to-back, and a
 * phantom order interval fans out one offchain read per configured chain at once. A poll interval
 * measured in seconds says nothing about either. The pacing therefore lives at the provider, where
 * the endpoint's whole traffic is visible, rather than at any one caller — which is also what lets
 * several fillers in one process stay collectively within one budget instead of each pacing to it.
 */

const REQUESTS_PER_SECOND = 10

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

/** A node that answers instantly, recording when each request arrived. */
function stubNode() {
	const arrivals: number[] = []
	const server = http.createServer((req, res) => {
		let body = ""
		req.on("data", (chunk: Buffer) => {
			body += chunk
		})
		req.on("end", () => {
			arrivals.push(Date.now())
			const { id } = JSON.parse(body) as { id: number }
			res.setHeader("content-type", "application/json")
			res.end(JSON.stringify({ jsonrpc: "2.0", id, result: "0x00" }))
		})
	})

	return {
		arrivals,
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

/** A coprocessor whose derived HTTP endpoint is the stub node. */
function coprocessorFor(port: number): IntentsCoprocessor {
	const wsApi = { _rpcCore: { provider: { endpoint: `ws://127.0.0.1:${port}` } } } as unknown as ApiPromise
	return IntentsCoprocessor.fromApi(wsApi)
}

async function provider(coprocessor: IntentsCoprocessor) {
	const { provider } = (await coprocessor.queryApi()) as unknown as {
		provider: { send: (method: string, params: unknown[]) => Promise<unknown> }
	}
	return provider
}

describe("coprocessor HTTP request pacing", () => {
	let node: ReturnType<typeof stubNode>
	let port: number

	beforeEach(async () => {
		vi.stubEnv("HYPERBRIDGE_RPC_MAX_RPS", String(REQUESTS_PER_SECOND))
		node = stubNode()
		port = await node.listen()
	})

	afterEach(async () => {
		vi.unstubAllEnvs()
		await node.close()
	})

	it("lets one second's worth of requests burst and paces the rest", async () => {
		const send = await provider(coprocessorFor(port))

		const started = Date.now()
		await Promise.all(
			Array.from({ length: REQUESTS_PER_SECOND * 2 }, () => send.send("state_getStorage", ["0x00"])),
		)

		expect(node.arrivals).toHaveLength(REQUESTS_PER_SECOND * 2)
		// Unpaced, twenty requests against a local node land in a handful of milliseconds.
		expect(Date.now() - started).toBeGreaterThanOrEqual(900)

		// Spans rather than absolute times, so a loaded machine slows both sides alike: the first
		// ten are a burst, the next ten are a tenth of a second apart each.
		const burst = node.arrivals[REQUESTS_PER_SECOND - 1] - node.arrivals[0]
		const paced = node.arrivals[REQUESTS_PER_SECOND * 2 - 1] - node.arrivals[REQUESTS_PER_SECOND - 1]
		expect(burst).toBeLessThan(paced)
	})

	// Several fillers in one process each hold their own coprocessor, and the limit they are up
	// against counts requests per address. A bucket per instance would exceed it by their count.
	it("shares one budget across every coprocessor pointed at the same endpoint", async () => {
		const [first, second] = await Promise.all([
			provider(coprocessorFor(port)),
			provider(coprocessorFor(port)),
		])

		const started = Date.now()
		await Promise.all([
			...Array.from({ length: REQUESTS_PER_SECOND }, () => first.send("state_getStorage", ["0x00"])),
			...Array.from({ length: REQUESTS_PER_SECOND }, () => second.send("state_getStorage", ["0x00"])),
		])

		// Twenty requests through one bucket takes a second. Through two it would take none.
		expect(node.arrivals).toHaveLength(REQUESTS_PER_SECOND * 2)
		expect(Date.now() - started).toBeGreaterThanOrEqual(900)
	})
})
