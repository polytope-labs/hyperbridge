import { createServer, type Server } from "node:http"
import { createPublicClient } from "viem"
import { afterEach, describe, expect, it } from "vitest"
import { EMPTY_ANSWER_GRACE_MS, raceTransport } from "@/services/ChainClientManager"

type Behaviour =
	| { answer: unknown; afterMs?: number }
	| { revert: true; afterMs?: number }
	| { refuse: number }
	| { status: number }
	| { hang: true }

const servers: Server[] = []
afterEach(() => {
	for (const server of servers.splice(0)) {
		server.closeAllConnections()
		server.close()
	}
})

/** A local JSON-RPC endpoint that answers every call the same way, and counts the calls it got. */
async function endpoint(behaviour: Behaviour): Promise<{ url: string; calls: () => number }> {
	let calls = 0
	const server = createServer((req, res) => {
		let body = ""
		req.on("data", (chunk) => {
			body += chunk
		})
		req.on("end", () => {
			calls += 1
			if ("hang" in behaviour) return
			if ("status" in behaviour) {
				res.writeHead(behaviour.status)
				res.end()
				return
			}
			const { id } = JSON.parse(body) as { id: number }
			const reply =
				"answer" in behaviour
					? { jsonrpc: "2.0", id, result: behaviour.answer }
					: "refuse" in behaviour
						? {
								jsonrpc: "2.0",
								id,
								error: { code: behaviour.refuse, message: "the method does not exist" },
							}
						: { jsonrpc: "2.0", id, error: { code: 3, message: "execution reverted" } }
			setTimeout(
				() => {
					res.writeHead(200, { "Content-Type": "application/json" })
					res.end(JSON.stringify(reply))
				},
				"afterMs" in behaviour ? (behaviour.afterMs ?? 0) : 0,
			)
		})
	})
	servers.push(server)
	const url = await new Promise<string>((resolve) => {
		server.listen(0, "127.0.0.1", () => {
			const address = server.address()
			resolve(`http://127.0.0.1:${typeof address === "object" && address !== null ? address.port : 0}`)
		})
	})
	return { url, calls: () => calls }
}

const url = async (behaviour: Behaviour) => (await endpoint(behaviour)).url

/** A URL nothing listens on, so the connection is refused at once. */
async function refusedUrl(): Promise<string> {
	const refused = await url({ hang: true })
	const server = servers.pop()!
	await new Promise<void>((resolve) => server.close(() => resolve()))
	return refused
}

const blockNumber = (urls: string[]) => createPublicClient({ transport: raceTransport(urls) }).getBlockNumber()

/** The raw JSON-RPC result, so the tests see exactly what the race settled on. */
const ask = (urls: string[], method: string) =>
	createPublicClient({ transport: raceTransport(urls) }).request({ method, params: [] } as never) as Promise<unknown>

const RECEIPT = { status: "0x1", transactionHash: `0x${"ab".repeat(32)}` }

describe("raceTransport", () => {
	it("answers from a live endpoint without waiting for the ones before it", async () => {
		const urls = [
			await refusedUrl(),
			await url({ hang: true }),
			await url({ status: 429 }),
			await url({ answer: "0x10" }),
		]
		const started = Date.now()
		await expect(blockNumber(urls)).resolves.toBe(16n)
		// The ordered fallback spent seven seconds of retries on each dead endpoint first.
		expect(Date.now() - started).toBeLessThan(2_000)
	})

	it("uses whichever endpoint answers first", async () => {
		const urls = [await url({ answer: "0x1", afterMs: 300 }), await url({ answer: "0x2" })]
		await expect(blockNumber(urls)).resolves.toBe(2n)
	})

	it("takes a revert as the answer", async () => {
		const urls = [await url({ revert: true }), await url({ answer: "0x", afterMs: 300 })]
		const client = createPublicClient({ transport: raceTransport(urls) })
		await expect(client.call({ to: "0x0000000000000000000000000000000000000001" })).rejects.toThrow(/revert/i)
	})

	it("fails once every endpoint has failed", async () => {
		const urls = [await refusedUrl(), await url({ status: 503 })]
		await expect(blockNumber(urls)).rejects.toThrow()
	})

	it("retries the race when one endpoint refused the call and another could not serve it", async () => {
		// The refusal arrives first. Reported alone, it would end the read with no retry.
		const refusing = await endpoint({ refuse: -32601 })
		const overloaded = await endpoint({ status: 503 })
		await expect(blockNumber([refusing.url, overloaded.url])).rejects.toThrow()
		expect(overloaded.calls()).toBeGreaterThan(1)
	})
})

describe("raceTransport and an endpoint that is behind", () => {
	/**
	 * A node one block behind says "not found" for a receipt another node has. viem's
	 * receipt wait reads that as a failed transaction, so a faster "not found" must not win.
	 */
	it("returns the receipt one endpoint has, although another answered first that there is none", async () => {
		const urls = [await url({ answer: null }), await url({ answer: RECEIPT, afterMs: 200 })]
		await expect(ask(urls, "eth_getTransactionReceipt")).resolves.toEqual(RECEIPT)
	})

	it("returns logs and code the same way", async () => {
		const logs = [{ logIndex: "0x0" }]
		await expect(
			ask([await url({ answer: [] }), await url({ answer: logs, afterMs: 200 })], "eth_getLogs"),
		).resolves.toEqual(logs)
		await expect(
			ask([await url({ answer: "0x" }), await url({ answer: "0xef0100", afterMs: 200 })], "eth_getCode"),
		).resolves.toBe("0xef0100")
	})

	it("says there is nothing as soon as every endpoint has said so", async () => {
		const urls = [await url({ answer: null }), await url({ answer: null, afterMs: 100 })]
		const started = Date.now()
		await expect(ask(urls, "eth_getTransactionReceipt")).resolves.toBeNull()
		expect(Date.now() - started).toBeLessThan(EMPTY_ANSWER_GRACE_MS)
	})

	it("stops waiting for a silent endpoint a second after another said there is nothing", async () => {
		const urls = [await url({ answer: null }), await url({ hang: true })]
		const started = Date.now()
		await expect(ask(urls, "eth_getTransactionReceipt")).resolves.toBeNull()
		const elapsed = Date.now() - started
		expect(elapsed).toBeGreaterThanOrEqual(EMPTY_ANSWER_GRACE_MS - 50)
		expect(elapsed).toBeLessThan(EMPTY_ANSWER_GRACE_MS + 1_500)
	})

	it("does not treat an empty call result as nothing", async () => {
		// `0x` from eth_call is data, not absence: the first answer stands.
		const urls = [await url({ answer: "0x" }), await url({ answer: "0x01", afterMs: 200 })]
		await expect(ask(urls, "eth_call")).resolves.toBe("0x")
	})
})

describe("raceTransport nonce reads", () => {
	it("asks the endpoints in order, like the wallet client's sends", async () => {
		const urls = [await url({ answer: "0x5", afterMs: 200 }), await url({ answer: "0x9" })]
		await expect(ask(urls, "eth_getTransactionCount")).resolves.toBe("0x5")
	})

	it("moves to the next endpoint when the first cannot answer", async () => {
		const urls = [await refusedUrl(), await url({ answer: "0x9" })]
		await expect(ask(urls, "eth_getTransactionCount")).resolves.toBe("0x9")
	})
})
