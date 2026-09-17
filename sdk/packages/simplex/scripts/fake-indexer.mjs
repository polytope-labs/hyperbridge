#!/usr/bin/env node
/**
 * The Hyperbridge indexer and the gateways' RPC, as the HyperFX orderbook's validation cycle reads
 * them, answering enough for a solver's orders to be validated and surfaced.
 *
 * Modelled on the orderbook's own test harness (`crates/server/tests/common/indexer.rs`), which
 * mounts the indexer at `/graphql` and every gateway at `/rpc/<chain>`. Everything here answers
 * generously and statically: every solver holds plenty of every token and is delegated, and the
 * protocol fee is whatever `--fee-bps` says. Nothing here is a fixture for the orderbook's own
 * behaviour, only enough for it to run without a chain or an indexer behind it.
 *
 *   node scripts/fake-indexer.mjs --port 4000 --fee-bps 0 --balance 1000000000
 */
import { createServer } from "node:http"

const args = new Map()
for (let i = 2; i < process.argv.length; i += 2) args.set(process.argv[i].replace(/^--/, ""), process.argv[i + 1])

const port = Number(args.get("port") ?? 4000)
const feeBps = Number(args.get("fee-bps") ?? 0)
/** Whole units every solver is reported to hold of every token asked about. */
const balance = BigInt(args.get("balance") ?? "1000000000")
/** The decimals the reported balances are scaled to, matching the orderbook's own 1e18 normalising. */
const decimals = Number(args.get("decimals") ?? 6)

/** `params()` on the gateway: six static words, the fifth of which is `protocolFeeBps`. */
const PARAMS_SELECTOR = "0xcff0ab96"
const PROTOCOL_FEE_WORD = 4

const now = () => new Date().toISOString().replace(/\.\d{3}Z$/, "+00:00")

function paramsResult() {
	const words = Array.from({ length: 6 }, (_, index) =>
		index === PROTOCOL_FEE_WORD ? feeBps.toString(16).padStart(64, "0") : "0".repeat(64),
	)
	return `0x${words.join("")}`
}

/** One page of a Relay connection, all of it: nothing here has more rows than a page. */
const page = (nodes) => ({ nodes, pageInfo: { hasNextPage: false, endCursor: String(nodes.length) } })

function inventory(variables) {
	const solvers = variables.solvers ?? []
	const tokens = variables.tokens ?? []
	const data = {}
	if (variables.withHead) {
		data.solverInventoryHead = { blockNumber: "35000000", observedAt: now() }
	}
	if (variables.withInventories) {
		const raw = (balance * 10n ** BigInt(decimals)).toString()
		data.solverInventories = page(
			solvers.flatMap((solver) =>
				tokens.map((tokenAddress) => ({
					solver,
					tokenAddress,
					balance: raw,
					wallet: raw,
					vaults: "0",
					vaultShares: "0",
					blockNumber: "34990000",
					observedAt: now(),
					refreshedAt: now(),
				})),
			),
		)
	}
	if (variables.withDelegations) {
		data.solverDelegations = page(
			solvers.map((solver) => ({
				solver,
				delegated: true,
				delegate: null,
				blockNumber: "34000000",
				observedAt: now(),
				refreshedAt: now(),
			})),
		)
	}
	return { data }
}

const server = createServer((req, res) => {
	let body = ""
	req.on("data", (chunk) => {
		body += chunk
	})
	req.on("end", () => {
		const answer = (status, payload) => {
			res.writeHead(status, { "content-type": "application/json" })
			res.end(JSON.stringify(payload))
		}
		let parsed
		try {
			parsed = JSON.parse(body || "{}")
		} catch {
			return answer(400, { error: "not JSON" })
		}

		if (req.url?.startsWith("/rpc/")) {
			const call = parsed.params?.[0] ?? {}
			if (parsed.method !== "eth_call" || call.data !== PARAMS_SELECTOR) {
				// Loud rather than silent: a call this does not recognise means the
				// orderbook is asking for something this stub was never told about.
				console.error(`fake-indexer: unexpected RPC ${parsed.method} ${call.data ?? ""}`)
				return answer(400, { error: "only params() is served" })
			}
			return answer(200, { jsonrpc: "2.0", id: parsed.id ?? 1, result: paramsResult() })
		}

		if (req.url?.startsWith("/graphql")) {
			return answer(200, inventory(parsed.variables ?? {}))
		}
		return answer(404, { error: "no such path" })
	})
})

server.listen(port, "0.0.0.0", () => {
	console.log(`fake-indexer: listening on ${port}, fee ${feeBps} bps, every solver holding ${balance}`)
})
