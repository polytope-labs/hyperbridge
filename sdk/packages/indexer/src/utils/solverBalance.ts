// Shared wiring for reading a solver's on-chain inventory. Two paths need it — the phantom price
// snapshot on Hyperbridge and the liquidity refresh on every EVM event that moves a solver's
// inventory — and both must read balances exactly the same way, or a refresh would republish depth
// on a different basis than the snapshot it is correcting.
import { UNISWAP_V4_ADDRESSES } from "@/addresses/uniswap-v4.addresses"
import type { LiquidityRefreshContext } from "@/services/liquidityPool.service"
import { ENV_CONFIG, HYPERBRIDGE } from "@/constants"
import { timestampToDate } from "@/utils/date.helpers"
import { keccakVm2 } from "@/utils/phantom-decode"
import { replaceWebsocketWithHttp } from "@/utils/rpc.helpers"
import { safeFetch } from "@/utils/safeFetch"
import { YIELD_VAULT_ADDRESSES } from "@/yield-vault-addresses"
import {
	memoizedSolverBalance,
	readV4Position,
	setAggregationFetch,
	type SolverBalanceReader,
	type V4PositionState,
} from "@hyperbridge/sdk/intents-helpers"

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

/** The on-chain reads a refresh needs, pinned to one block and memoized for it. */
export interface BlockReaders {
	getBalance: SolverBalanceReader
	/** Null when the position no longer exists; throws when it cannot be read at all. */
	readPosition: (chain: string, tokenId: bigint) => Promise<V4PositionState | null>
}

// Reads are point-in-time and cannot differ within a block, so one memo serves every read on it —
// without this, several orders closing on the same Hyperbridge block would each repeat the full
// liquidity sweep, and several events in one EVM block would each re-read the same bidders and the
// same positions. Blocks are processed in order, so keeping only the current block's memo is enough.
let memoKey: string | null = null
let memo: BlockReaders | null = null

/**
 * The readers for `key`, which must identify one block of one chain (handlers for different chains
 * run in separate processes, so the key only has to be unique within one).
 *
 * `blockTags` pins a chain to a specific block, so an event's re-read returns the same value on a
 * replay as it did live. Only the chain the event is on can be pinned: block numbers are per chain,
 * and a refresh reaches across every chain the pool is quoted on.
 */
export function blockReaders(key: string, blockTags: Record<string, string> = {}): BlockReaders {
	if (memoKey !== key || !memo) {
		memo = {
			getBalance: memoizedSolverBalance(YIELD_VAULT_ADDRESSES, blockTags),
			readPosition: positionReader(blockTags),
		}
		memoKey = key
	}
	return memo
}

/**
 * Promise-caching position reader. A position is shared by every bidder row of the solver that
 * declared it, so without the memo a solver backing four pools would read the same position four
 * times per event.
 *
 * A chain with no configured Uniswap V4 deployment, or no RPC, throws rather than reporting the
 * position absent: absent means burned or sold, which drops the row, and a configuration gap must
 * not be allowed to delete a solver's positions.
 */
function positionReader(blockTags: Record<string, string>): BlockReaders["readPosition"] {
	const urls = evmRpcUrls()
	const cache = new Map<string, Promise<V4PositionState | null>>()
	return (chain: string, tokenId: bigint) => {
		const contracts = UNISWAP_V4_ADDRESSES[chain]
		const evmRpcUrl = urls[chain]
		if (!contracts || !evmRpcUrl) {
			return Promise.reject(
				new Error(
					`No Uniswap V4 deployment or RPC configured for ${chain}, cannot re-read position ${tokenId}`,
				),
			)
		}
		const cacheKey = `${chain}|${tokenId}`
		let pending = cache.get(cacheKey)
		if (!pending) {
			pending = readV4Position({
				evmRpcUrl,
				contracts,
				tokenId,
				keccak: keccakVm2,
				blockTag: blockTags[chain],
				logger,
			}).catch((err) => {
				// Evict on rejection so one blip is not cached for the whole block.
				cache.delete(cacheKey)
				throw err
			})
			cache.set(cacheKey, pending)
		}
		return pending
	}
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

/**
 * The reads and the clock a liquidity refresh triggered by an event on `chain` runs against.
 *
 * Everything is scoped to the event's block: the balance and position reads are pinned to it, so a
 * replay records what the event actually left behind rather than today's state, and the memo is
 * keyed by it, so several events in one block share one set of reads instead of repeating them.
 * Only this chain is pinned — a pool spans chains, and a block number means nothing on any other.
 */
export function liquidityRefreshContext(
	chain: string,
	blockNumber: number | bigint,
	timestamp: bigint,
): LiquidityRefreshContext {
	const key = `${chain}-${blockNumber}`
	const readers = blockReaders(key, { [chain]: `0x${blockNumber.toString(16)}` })
	return {
		evmRpcUrls: evmRpcUrls(),
		getBalance: readers.getBalance,
		readPosition: readers.readPosition,
		headBlock: () => hyperbridgeHeadBlock(key),
		observedAt: timestampToDate(timestamp),
	}
}
