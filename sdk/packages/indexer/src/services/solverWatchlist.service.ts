// The Hyperbridge node's half of solver discovery: poll the HyperFX orderbook's watchlist and queue
// every solver it has not asked about before, for the chain's EVM node to start tracking
// (solverInventory.service.ts). This closes the cold start that fill-based discovery leaves open: a
// solver that has never filled has no inventory row, so the orderbook cannot validate its orders,
// so it can never win the fill that would have made it visible.
//
// On determinism: the watchlist describes now, so a resync sees it as it is then, not as it was.
// That is tolerable only because discovery decides *when* tracking starts — the genesis read is
// pinned to the block the EVM node consumes the request at, so tracking is correct from whichever
// block that turns out to be. It would be wrong for anything read as a time series.
import { SolverDiscoveryRequest, SolverWatchlist } from "@/configs/src/types"
import { ENV_CONFIG } from "@/constants"
import { safeFetch } from "@/utils/safeFetch"
import { readAllPages } from "@/utils/store.helpers"
import { YIELD_VAULT_ADDRESSES } from "@/yield-vault-addresses"

/** How close to wall clock a block must be for the poll to run. */
export const LIVE_SLACK_MS = 120_000
/** A poll runs every block, so a slow orderbook must not hold one up for long. */
export const FETCH_TIMEOUT_MS = 5_000
/** Solvers accepted per chain per response; the orderbook caps its own list the same way. */
export const MAX_WATCHLIST_PER_CHAIN = 5_000

const ADDRESS = /^0x[0-9a-f]{40}$/

// The ETag of the last response fully applied. Losing it (a restart) costs one full response.
let etag: string | undefined

// When a poll last said why it did nothing. Every block asks, and the answer rarely changes, so
// this reports at most once a minute: enough to tell a silent-but-working node from a silent-and-
// skipping one, which is otherwise indistinguishable from the outside.
let lastSkipReport = 0
const SKIP_REPORT_INTERVAL_MS = 60_000

function reportSkip(reason: string, now: number): void {
	if (now - lastSkipReport < SKIP_REPORT_INTERVAL_MS) return
	lastSkipReport = now
	logger.info(`[solver-watchlist] Not polling: ${reason}`)
}

/** Forgets the last ETag and the skip report clock. For tests. */
export function resetSolverWatchlistPoll(): void {
	etag = undefined
	lastSkipReport = 0
}

function configuredUrl(): string | undefined {
	return (ENV_CONFIG as Record<string, string | null | undefined>)["HYPERFX_WATCHLIST_URL"] || undefined
}

const tracksChain = (chain: string) => Object.keys(YIELD_VAULT_ADDRESSES[chain] ?? {}).length > 0

/**
 * `{ chains: [{ chain, solvers: [{ address }] }] }` → lowercased, de-duplicated addresses per chain
 * this indexer tracks inventory on, capped per chain. Undefined when the body is not that shape.
 */
export function parseWatchlist(body: unknown): Map<string, string[]> | undefined {
	const chains = (body as { chains?: unknown })?.chains
	if (!Array.isArray(chains)) return undefined
	const parsed = new Map<string, string[]>()
	for (const entry of chains) {
		const chain = entry?.chain
		if (typeof chain !== "string" || !Array.isArray(entry?.solvers)) return undefined
		if (!tracksChain(chain)) continue
		const addresses = new Set<string>()
		for (const solver of entry.solvers) {
			const address = typeof solver?.address === "string" ? solver.address.toLowerCase() : undefined
			if (address && ADDRESS.test(address)) addresses.add(address)
		}
		let solvers = [...addresses]
		if (solvers.length > MAX_WATCHLIST_PER_CHAIN) {
			logger.warn(
				`[solver-watchlist] ${chain} lists ${solvers.length} solvers; ignoring all past the first ${MAX_WATCHLIST_PER_CHAIN}`,
			)
			solvers = solvers.slice(0, MAX_WATCHLIST_PER_CHAIN)
		}
		parsed.set(chain, solvers)
	}
	return parsed
}

/** Queues the solvers not already requested for `chain` as one new SolverWatchlist version. */
async function queueRequests(chain: string, solvers: string[], blockNumber: bigint, at: Date): Promise<void> {
	const requested = await readAllPages((limit, offset) =>
		SolverDiscoveryRequest.getByFields([["chain", "=", chain]], {
			limit,
			offset,
			orderBy: "id",
			orderDirection: "ASC",
		}),
	)
	const known = new Set(requested.map((request) => request.solver))
	const fresh = solvers.filter((solver) => !known.has(solver))
	if (fresh.length === 0) return

	const watchlist = await SolverWatchlist.get(chain)
	const version = (watchlist?.version ?? 0) + 1
	for (const solver of fresh) {
		await SolverDiscoveryRequest.create({
			id: `${chain}-${solver}`,
			chain,
			solver,
			version,
			blockNumber,
			requestedAt: at,
		}).save()
	}
	await SolverWatchlist.create({
		id: chain,
		chain,
		version,
		requestCount: known.size + fresh.length,
		blockNumber,
		updatedAt: at,
	}).save()
}

/**
 * One poll, run on every Hyperbridge block. Never throws for the orderbook's sake: an unreachable
 * or misbehaving orderbook is not an indexing error, so it is logged and the next block retries.
 */
export async function pollSolverWatchlist(params: {
	blockNumber: bigint
	blockTime: Date | undefined
	/** Wall clock, injectable for tests. */
	now?: number
	/** Overrides HYPERFX_WATCHLIST_URL. */
	url?: string
}): Promise<void> {
	const now = params.now ?? Date.now()
	const url = params.url ?? configuredUrl()
	if (!url) {
		reportSkip("HYPERFX_WATCHLIST_URL is not set", now)
		return
	}
	// A resync from genesis would otherwise replay one fetch per historical block of a list that
	// only describes now, hammering the endpoint and learning nothing.
	if (!params.blockTime) {
		reportSkip(`block ${params.blockNumber} has no timestamp`, now)
		return
	}
	const lagMs = now - params.blockTime.getTime()
	if (lagMs > LIVE_SLACK_MS) {
		reportSkip(
			`block ${params.blockNumber} trails wall clock by ${Math.round(lagMs / 1000)}s, over the ${LIVE_SLACK_MS / 1000}s bound`,
			now,
		)
		return
	}

	let chains: Map<string, string[]> | undefined
	let responseTag: string | undefined
	try {
		const response = await safeFetch(url, {
			headers: { accept: "application/json", ...(etag ? { "if-none-match": etag } : {}) },
			timeoutMs: FETCH_TIMEOUT_MS,
		})
		if (response.status === 304) return
		if (!response.ok) {
			logger.warn(`[solver-watchlist] ${url} answered ${response.status}; retrying next block`)
			return
		}
		chains = parseWatchlist(await response.json())
		const header = response.headers["etag"]
		responseTag = Array.isArray(header) ? header[0] : header
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error)
		logger.warn(`[solver-watchlist] Fetching ${url} failed; retrying next block: ${message}`)
		return
	}
	if (!chains) {
		logger.warn(`[solver-watchlist] ${url} returned an unrecognised body; retrying next block`)
		return
	}

	for (const [chain, solvers] of chains) {
		await queueRequests(chain, solvers, params.blockNumber, params.blockTime)
	}
	// Only once everything is queued: a store failure above must not let the next poll 304 past it.
	etag = responseTag
}
