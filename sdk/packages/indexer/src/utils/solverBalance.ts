// Shared wiring for reading a solver's on-chain inventory. Two paths need it — the phantom price
// snapshot on Hyperbridge and the pool liquidity refresh on every EVM fill — and both must read
// balances exactly the same way, or a refresh would republish depth on a different basis than the
// snapshot it is correcting.
import { ENV_CONFIG, HYPERBRIDGE } from "@/constants"
import { replaceWebsocketWithHttp } from "@/utils/rpc.helpers"
import { safeFetch } from "@/utils/safeFetch"
import { YIELD_VAULT_ADDRESSES } from "@/yield-vault-addresses"
import { memoizedSolverBalance, setAggregationFetch, type SolverBalanceReader } from "@hyperbridge/sdk/intents-helpers"

// The aggregation's RPC helpers run inside the SubQuery VM2 sandbox, which has no global `fetch`.
// Inject the indexer's sandbox-safe HTTP client so its JSON-RPC calls work here.
setAggregationFetch(safeFetch)

/**
 * HTTP RPC per supported EVM chain. Both callers sweep balances across every chain, not just the
 * one whose event they are handling, so a chain missing from here is simply one whose balances
 * cannot be read.
 */
export function evmRpcUrls(): Record<string, string> {
	const urls: Record<string, string> = {}
	for (const [stateMachineId, url] of Object.entries(ENV_CONFIG)) {
		if (!stateMachineId.startsWith("EVM-")) continue
		const http = replaceWebsocketWithHttp(url ?? "")
		if (http) urls[stateMachineId] = http
	}
	return urls
}

// Balances are point-in-time reads that cannot differ within a block, so one memo serves every
// read on it — without this, several orders closing on the same Hyperbridge block would each
// repeat the full liquidity sweep, and several fills in one EVM block would each re-read the same
// bidders. Blocks are processed in order, so keeping only the current block's memo is enough.
let memoKey: string | null = null
let memo: SolverBalanceReader | null = null

/**
 * The balance reader for `key`, which must identify one block of one chain (handlers for
 * different chains run in separate processes, so the key only has to be unique within one).
 */
export function blockBalanceReader(key: string): SolverBalanceReader {
	if (memoKey !== key || !memo) {
		memo = memoizedSolverBalance(YIELD_VAULT_ADDRESSES)
		memoKey = key
	}
	return memo
}

// Hyperbridge's head is read once per block of the chain being indexed, for the same reason
// balances are: several fills can land in one block, and they all record the same head.
let headKey: string | null = null
let head: Promise<bigint | null> | null = null

/**
 * Hyperbridge's current head block number, or null when there is no Hyperbridge RPC configured or
 * it cannot be read.
 *
 * Balance rows are keyed by Hyperbridge block, because that is the clock the phantom snapshots
 * write on. A balance re-read on an EVM fill has no such block of its own, so it borrows the one
 * Hyperbridge is on at that moment: the number stays monotonic with the rest of the series, which
 * is what keeps "greatest blockNumber is the current balance" true. It is a stamp, not a proof —
 * the balance was read at the EVM chain's head, not reconstructed at this Hyperbridge block.
 */
export function hyperbridgeHeadBlock(key: string): Promise<bigint | null> {
	if (headKey !== key || !head) {
		// Evict on rejection so one unreachable moment does not pin null for the whole block.
		head = readHyperbridgeHead().catch((err) => {
			logger.warn({ err }, "Could not read Hyperbridge's head block, skipping the LP balance row")
			return null
		})
		headKey = key
	}
	return head
}

async function readHyperbridgeHead(): Promise<bigint | null> {
	// Whichever Hyperbridge this deployment indexes; only one of them is ever configured.
	const host = [HYPERBRIDGE.mainnet, HYPERBRIDGE.testnet, HYPERBRIDGE.local].find((id) => ENV_CONFIG[id])
	if (!host) return null
	const url = replaceWebsocketWithHttp(ENV_CONFIG[host] ?? "")
	if (!url) return null

	const response = await safeFetch(url, {
		method: "POST",
		headers: { accept: "application/json", "content-type": "application/json" },
		body: JSON.stringify({ id: 1, jsonrpc: "2.0", method: "chain_getHeader", params: [] }),
	})
	const number = (await response.json())?.result?.number
	if (typeof number !== "string") return null
	return BigInt(number)
}
