import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http"
import { createPublicClient, http as viemHttp } from "viem"
import { base } from "viem/chains"
import { describe, it, expect, afterAll } from "vitest"
import { QuorumPublicClient, isRateLimited } from "@/services/QuorumPublicClient"

/**
 * 429 detection tests that exercise viem's REAL error objects, not synthetic ones.
 *
 * The unit tests in quorum-public-client.test.ts throw `new Error("429 …")` and
 * assert we skip it — which is circular: it passes even if viem's actual error
 * shape for an HTTP 429 changed. Here we run genuine HTTP servers that answer
 * with real 429/500 responses, let viem produce whatever it produces, and assert
 * `isRateLimited` classifies it correctly — both directly and end-to-end through
 * `QuorumPublicClient`'s skip-vs-pause policy.
 *
 * The last suite goes further and floods a real public RPC until it rate-limits
 * us, then asserts we detect the provider's actual 429. That deliberately abuses
 * a keyless endpoint, so it is opt-in. `eth.meowrpc.com` throttles ~a few
 * hundred requests/sec/IP, so it trips reliably:
 *
 *   SPAM_RPC_429=https://eth.meowrpc.com pnpm vitest run src/tests/rate-limit-detection.test.ts
 */

const BASE_CHAIN_ID = 8453

type Mode = "ok" | "limited" | "broken"

// Distinct loopback IPs so validateRpcUrls' distinct-hostname rule is satisfied.
const HOSTS = ["127.0.0.1", "127.0.0.2", "127.0.0.3", "127.0.0.4"] as const

const servers: Server[] = []
afterAll(async () => {
	await Promise.all(servers.map((s) => new Promise((resolve) => s.close(resolve))))
})

/** Real HTTP server speaking just enough JSON-RPC for eth_blockNumber. */
function rpcServer(host: string, mode: Mode): Promise<string> {
	const server = createServer((req: IncomingMessage, res: ServerResponse) => {
		let body = ""
		req.on("data", (chunk) => {
			body += chunk
		})
		req.on("end", () => {
			if (mode === "limited") {
				// Retry-After: 0 keeps viem's internal retries instant, so the
				// exhausted-retries 429 surfaces to our policy without slow backoff.
				res.writeHead(429, { "Content-Type": "text/plain", "Retry-After": "0" })
				res.end("Too Many Requests")
				return
			}
			if (mode === "broken") {
				res.writeHead(500, { "Content-Type": "text/plain", "Retry-After": "0" })
				res.end("internal error")
				return
			}
			const id = (() => {
				try {
					return JSON.parse(body).id ?? 1
				} catch {
					return 1
				}
			})()
			res.writeHead(200, { "Content-Type": "application/json" })
			res.end(JSON.stringify({ jsonrpc: "2.0", id, result: "0x64" })) // block 100
		})
	})
	servers.push(server)
	return new Promise((resolve, reject) => {
		server.once("error", reject)
		server.listen(0, host, () => {
			const address = server.address()
			if (address && typeof address === "object") {
				resolve(`http://${host}:${address.port}`)
			} else {
				reject(new Error("no server address"))
			}
		})
	})
}

describe("isRateLimited against real HTTP responses (local server)", () => {
	it("classifies viem's actual error for a genuine HTTP 429", async () => {
		const url = await rpcServer(HOSTS[0], "limited")
		const client = createPublicClient({ chain: base, transport: viemHttp(url, { retryCount: 0, timeout: 10_000 }) })

		let thrown: unknown
		try {
			await client.getBlockNumber()
		} catch (error) {
			thrown = error
		}
		expect(thrown).toBeInstanceOf(Error)
		// The property under test: whatever shape viem wraps the 429 in, we detect it.
		expect(isRateLimited(thrown)).toBe(true)
	}, 30_000)

	it("does NOT classify a genuine HTTP 500 as rate limiting", async () => {
		const url = await rpcServer(HOSTS[0], "broken")
		const client = createPublicClient({ chain: base, transport: viemHttp(url, { retryCount: 0, timeout: 10_000 }) })

		let thrown: unknown
		try {
			await client.getBlockNumber()
		} catch (error) {
			thrown = error
		}
		expect(thrown).toBeInstanceOf(Error)
		expect(isRateLimited(thrown)).toBe(false)
	}, 30_000)

	it("end-to-end: a really-429ing (or really-broken) public endpoint is excluded; the tiered quorum still succeeds", async () => {
		// operator + 3 public (operatorQuorum=1, requiredPublic=2). One public
		// genuinely 429s and one genuinely 500s — both excluded — but the operator
		// plus the two healthy publics still form the quorum.
		const [operator, pubOk1, pubOk2, pubBad] = await Promise.all([
			rpcServer(HOSTS[0], "ok"),
			rpcServer(HOSTS[1], "ok"),
			rpcServer(HOSTS[2], "ok"),
			rpcServer(HOSTS[3], "limited"),
		])
		const client = new QuorumPublicClient(BASE_CHAIN_ID, [operator, pubOk1, pubOk2, pubBad], 1)
		await expect(client.getBlockNumber()).resolves.toBe(100n)

		// With the 500 endpoint too: still one healthy operator + 2 healthy publics.
		const pubBroken = await rpcServer(HOSTS[3], "broken")
		const client2 = new QuorumPublicClient(BASE_CHAIN_ID, [operator, pubOk1, pubOk2, pubBroken], 1)
		await expect(client2.getBlockNumber()).resolves.toBe(100n)

		// But if a 429 drops one healthy public below the 2-witness floor, the call fails.
		const client3 = new QuorumPublicClient(BASE_CHAIN_ID, [operator, pubOk1, pubBad], 1)
		await expect(client3.getBlockNumber()).rejects.toThrow(/Quorum not reached/)
	}, 60_000)
})

// ---------------------------------------------------------------------------
// Opt-in: flood a real public RPC until IT rate-limits us, then verify we
// detect the provider's actual 429 shape. Not for CI — it hammers a shared
// keyless endpoint on purpose.
// ---------------------------------------------------------------------------

const SPAM_TARGET = process.env.SPAM_RPC_429
const describeIfSpam = SPAM_TARGET ? describe : describe.skip

describeIfSpam(`live 429 induction against ${SPAM_TARGET}`, () => {
	it("spams until rate limited and detects the provider's real 429", async (ctx) => {
		const client = createPublicClient({
			chain: base,
			transport: viemHttp(SPAM_TARGET!, { retryCount: 0, timeout: 10_000 }),
		})

		// Test-local evidence check, deliberately separate from the production
		// detector so the assertion below isn't circular: we SELECT the sample by
		// raw evidence of a 429, then assert the production classifier agrees.
		const looksLike429 = (error: unknown): boolean => {
			let current: unknown = error
			for (let depth = 0; depth < 6 && current instanceof Error; depth++) {
				if ((current as { status?: number }).status === 429) return true
				current = (current as { cause?: unknown }).cause
			}
			return /\b429\b/.test(String(error))
		}

		// Rate limits are per-second windows, so concurrency matters more than
		// total volume: escalate burst size until the endpoint throttles. Use the
		// raw EIP-1193 request, NOT getBlockNumber — viem's block-number action
		// dedupes concurrent calls into a single HTTP request, so a burst of them
		// would never actually reach the provider's rate limiter.
		const BURSTS = [50, 100, 200, 400, 800, 800, 800, 800]
		let real429: unknown
		let sampleRejection: unknown
		let sent = 0
		outer: for (const burstSize of BURSTS) {
			sent += burstSize
			const burst = await Promise.allSettled(
				Array.from({ length: burstSize }, () => client.request({ method: "eth_blockNumber" })),
			)
			for (const result of burst) {
				if (result.status !== "rejected") continue
				sampleRejection ??= result.reason
				if (looksLike429(result.reason)) {
					real429 = result.reason
					break outer
				}
			}
		}

		if (!real429) {
			// Endpoint absorbed everything up to 800-way concurrency (generous
			// tier or a CDN soaking the burst) — inconclusive, not a failure.
			console.warn(
				`No 429 induced from ${SPAM_TARGET} after ${sent} requests (bursts up to 800); sample rejection:`,
				sampleRejection ? String(sampleRejection) : "(none)",
			)
			ctx.skip()
			return
		}

		console.log("Real provider 429:", (real429 as Error).constructor.name, String(real429).slice(0, 300))
		expect(isRateLimited(real429)).toBe(true)
	}, 180_000)
})
