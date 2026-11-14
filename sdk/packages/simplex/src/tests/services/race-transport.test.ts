import { createServer, type Server } from "node:http"
import { createPublicClient } from "viem"
import { afterEach, describe, expect, it } from "vitest"
import { raceTransport } from "@/services/ChainClientManager"

type Behaviour =
	| { answer: string; afterMs?: number }
	| { revert: true; afterMs?: number }
	| { status: number }
	| { hang: true }

const servers: Server[] = []
afterEach(() => {
	for (const server of servers.splice(0)) {
		server.closeAllConnections()
		server.close()
	}
})

/** A local JSON-RPC endpoint that answers every call the same way. */
function endpoint(behaviour: Behaviour): Promise<string> {
	const server = createServer((req, res) => {
		let body = ""
		req.on("data", (chunk) => {
			body += chunk
		})
		req.on("end", () => {
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
					: { jsonrpc: "2.0", id, error: { code: 3, message: "execution reverted" } }
			setTimeout(() => {
				res.writeHead(200, { "Content-Type": "application/json" })
				res.end(JSON.stringify(reply))
			}, behaviour.afterMs ?? 0)
		})
	})
	servers.push(server)
	return new Promise((resolve) => {
		server.listen(0, "127.0.0.1", () => {
			const address = server.address()
			resolve(`http://127.0.0.1:${typeof address === "object" && address !== null ? address.port : 0}`)
		})
	})
}

/** A URL nothing listens on, so the connection is refused at once. */
async function refusedUrl(): Promise<string> {
	const url = await endpoint({ hang: true })
	const server = servers.pop()!
	await new Promise<void>((resolve) => server.close(() => resolve()))
	return url
}

const blockNumber = (urls: string[]) => createPublicClient({ transport: raceTransport(urls) }).getBlockNumber()

describe("raceTransport", () => {
	it("answers from a live endpoint without waiting for the ones before it", async () => {
		const urls = [
			await refusedUrl(),
			await endpoint({ hang: true }),
			await endpoint({ status: 429 }),
			await endpoint({ answer: "0x10" }),
		]
		const started = Date.now()
		await expect(blockNumber(urls)).resolves.toBe(16n)
		// The ordered fallback spent seven seconds of retries on each dead endpoint first.
		expect(Date.now() - started).toBeLessThan(2_000)
	})

	it("uses whichever endpoint answers first", async () => {
		const urls = [await endpoint({ answer: "0x1", afterMs: 300 }), await endpoint({ answer: "0x2" })]
		await expect(blockNumber(urls)).resolves.toBe(2n)
	})

	it("takes a revert as the answer", async () => {
		const urls = [await endpoint({ revert: true }), await endpoint({ answer: "0x", afterMs: 300 })]
		const client = createPublicClient({ transport: raceTransport(urls) })
		await expect(client.call({ to: "0x0000000000000000000000000000000000000001" })).rejects.toThrow(/revert/i)
	})

	it("fails once every endpoint has failed", async () => {
		const urls = [await refusedUrl(), await endpoint({ status: 503 })]
		await expect(blockNumber(urls)).rejects.toThrow()
	})
})
