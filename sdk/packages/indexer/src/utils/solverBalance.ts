// Shared wiring for reading a solver's on-chain inventory. Two paths need it — the phantom price
// snapshot on Hyperbridge and the liquidity refresh on every EVM event that moves a solver's
// inventory — and both must read balances exactly the same way, or a refresh would republish depth
// on a different basis than the snapshot it is correcting.
import { UNISWAP_V4_ADDRESSES } from "@/addresses/uniswap-v4.addresses"
import type { InventoryReadContext } from "@/services/inventoryReading.service"
import type { InventoryReadingTrigger } from "@/configs/src/types"
import { ENV_CONFIG } from "@/constants"
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
 * HTTP RPC per supported EVM chain. The phantom sweep reads balances across every chain; an
 * event-driven read uses only its own chain's entry. A chain missing from here is simply one
 * whose balances cannot be read.
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
 * replay as it did live. The phantom sweep pins nothing and reads every chain at its head.
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

/**
 * The reads and the clock an inventory publication triggered by an event on `chain` runs against.
 *
 * Everything is scoped to the event's block: the balance and position reads are pinned to it, so a
 * replay records what the event actually left behind rather than today's state, and the memo is
 * keyed by it, so several events in one block share one set of reads instead of repeating them.
 * Only this chain is read — the node handling the event has this chain's RPC and this chain's
 * events, and any other chain's inventory is that chain's node's to publish.
 */
export function inventoryReadContext(
	chain: string,
	blockNumber: number | bigint,
	timestamp: bigint,
	trigger: InventoryReadingTrigger,
): InventoryReadContext {
	const key = `${chain}-${blockNumber}`
	const readers = blockReaders(key, { [chain]: `0x${blockNumber.toString(16)}` })
	return {
		chain,
		evmRpcUrl: evmRpcUrls()[chain],
		getBalance: readers.getBalance,
		readPosition: readers.readPosition,
		blockNumber: BigInt(blockNumber),
		observedAt: timestampToDate(timestamp),
		trigger,
	}
}
