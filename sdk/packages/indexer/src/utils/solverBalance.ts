// Shared wiring for reading a solver's on-chain inventory. Two paths need it — the phantom price
// snapshot on Hyperbridge and the pool liquidity refresh on every EVM fill — and both must read
// balances exactly the same way, or a refresh would republish depth on a different basis than the
// snapshot it is correcting.
import { ENV_CONFIG } from "@/constants"
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
