// Turns per-leg phantom snapshots into the pair-centric pool entities. A pair has one price
// across chains per direction — solvers rebalance and arbitrage away per-chain drift — so each
// chain's sample only differs in depth, and the pool's single buy/sell number is the
// depth-weighted merge of the chains' latest samples.
import { getPoolToken, poolSlug, sortPoolSymbols } from "@/addresses/pool-tokens.addresses"
import { LiquidityPool, LiquidityProvider, PoolBidder, PoolChainLiquidity } from "@/configs/src/types"
import type { PhantomLegAggregation } from "@hyperbridge/sdk/intents-helpers"

export const SELL = "SELL"
export const BUY = "BUY"

// A small multiple of the phantom generation interval. Chain samples older than this relative to
// the block being processed stop influencing the pool price, so one quiet chain cannot pin a
// stale price or depth on a live market — unless EVERY sample is stale, in which case last-known
// values are better than nulling a pool that merely paused.
const MAX_SAMPLE_AGE_BLOCKS = 1800n

// Registered pairs are pallet-bounded at 64 per chain and chains are bounded by config, so these
// reads cover the full row sets.
const POOL_ROWS_LIMIT = 100

/** One registered pair of the phantom order, with its leg tokens as 20-byte lowercase addresses. */
export interface RegisteredPair {
	pairIndex: number
	tokenA: string
	tokenB: string
	standardAmount: bigint
}

export interface ResolvedPoolLeg {
	poolId: string
	/** SELL when tokenA is the pool's token0 (or the pair is same-asset), BUY otherwise. */
	direction: string
	token0Symbol: string
	token1Symbol: string
	/** Registry decimals of the leg's output token on this chain, for 1e18 normalization. */
	outDecimals: number
}

/**
 * Resolves a phantom leg to its pool identity via the token registry. Null when either token is
 * not registry-tracked, or when the pallet's standard amount disagrees with the registry's input
 * decimals — the standard amount is the denominator of every rate, so a mismatch means either
 * side is misconfigured and any derived rate would be off by an integer factor.
 */
export function resolvePoolLeg(chain: string, pair: RegisteredPair): ResolvedPoolLeg | null {
	const inputToken = getPoolToken(chain, pair.tokenA)
	const outputToken = getPoolToken(chain, pair.tokenB)
	if (!inputToken || !outputToken || outputToken.decimals > 18) return null

	if (pair.standardAmount !== 10n ** BigInt(inputToken.decimals)) {
		logger.warn(
			{ chain, tokenA: pair.tokenA, standardAmount: pair.standardAmount.toString(), decimals: inputToken.decimals },
			"Phantom pair standard amount disagrees with the registry decimals, skipping pool attribution",
		)
		return null
	}

	const [token0Symbol, token1Symbol] = sortPoolSymbols(inputToken.symbol, outputToken.symbol)
	return {
		poolId: poolSlug(inputToken.symbol, outputToken.symbol),
		direction: inputToken.symbol === token0Symbol ? SELL : BUY,
		token0Symbol,
		token1Symbol,
		outDecimals: outputToken.decimals,
	}
}

interface PoolBidderSample {
	solver: string
	liquidity: bigint
	acceptedSources: string[] | null
	outputToken: string
}

interface DirectionSample {
	resolved: ResolvedPoolLeg
	/** Rates/depths for the (pool, direction) key; more than one only when distinct legs collapse onto it. */
	samples: { rate: bigint; depth: bigint }[]
	bidCount: number
	bidders: Map<string, PoolBidderSample>
	/** False when the key exists only as a registered-but-unquoted zero-liquidity signal. */
	quoted: boolean
}

/**
 * Upserts the pool entities for one phantom price snapshot. Every write is a deterministic
 * upsert or an id-set-diff removal, so replaying the same event converges to the same state.
 */
export async function updateLiquidityPools(params: {
	chain: string
	blockNumber: bigint
	snapshotTime: Date
	pairs: RegisteredPair[]
	legs: PhantomLegAggregation[]
}): Promise<void> {
	const { chain, blockNumber, snapshotTime, pairs, legs } = params
	const legsByIndex = new Map(legs.map((leg) => [leg.pairIndex, leg]))

	// Accumulate per (pool, direction) — the chain is fixed for the whole order. A registered pair
	// nobody quoted is a real zero-liquidity signal, but only when no other leg quoted the same key
	// (same-asset pairs collapse both token representations onto one key).
	const keyed = new Map<string, DirectionSample>()
	for (const pair of pairs) {
		const resolved = resolvePoolLeg(chain, pair)
		if (!resolved) continue

		const key = `${resolved.poolId}|${resolved.direction}`
		const entry: DirectionSample = keyed.get(key) ?? {
			resolved,
			samples: [],
			bidCount: 0,
			bidders: new Map(),
			quoted: false,
		}
		keyed.set(key, entry)

		const leg = legsByIndex.get(pair.pairIndex)
		if (!leg) continue

		const scale = 10n ** BigInt(18 - resolved.outDecimals)
		let depth = 0n
		for (const bidder of leg.bidders) {
			const solver = bidder.solver.toLowerCase()
			const liquidity = bidder.weight * scale
			depth += liquidity
			entry.bidders.set(`${resolved.poolId}-${chain}-${resolved.direction}-${pair.tokenB}-${solver}`, {
				solver,
				liquidity,
				acceptedSources: bidder.acceptedSources,
				outputToken: pair.tokenB,
			})
		}
		entry.samples.push({ rate: leg.medianPrice * scale, depth })
		entry.bidCount += leg.bidCount
		entry.quoted = true
	}
	if (keyed.size === 0) return

	const touchedPools = new Map<string, ResolvedPoolLeg>()
	for (const entry of keyed.values()) touchedPools.set(entry.resolved.poolId, entry.resolved)

	for (const [poolId, resolved] of touchedPools) {
		if (!(await LiquidityPool.get(poolId))) {
			// Rates stay null until the merge below prices a direction, so a one-way pair never
			// fabricates the other side.
			await LiquidityPool.create({
				id: poolId,
				token0Symbol: resolved.token0Symbol,
				token1Symbol: resolved.token1Symbol,
				sellDepth: 0n,
				buyDepth: 0n,
				sellBidCount: 0,
				buyBidCount: 0,
				lastUpdatedBlock: blockNumber,
				lastUpdatedAt: snapshotTime,
			}).save()
		}

		// Reconcile this chain's bidder rows against the ones this window produced. The order
		// carries every configured pair for the chain, so any existing row it did not regenerate
		// belongs to a solver that stopped bidding (if pairs were ever sharded across several
		// orders per chain, this set-diff would wrongly drop the other shard's bidders).
		const desired = new Map<string, { bidder: PoolBidderSample; direction: string }>()
		for (const entry of keyed.values()) {
			if (entry.resolved.poolId !== poolId) continue
			for (const [id, bidder] of entry.bidders) desired.set(id, { bidder, direction: entry.resolved.direction })
		}
		const existing = await PoolBidder.getByFields(
			[
				["poolId", "=", poolId],
				["chain", "=", chain],
			],
			{ limit: POOL_ROWS_LIMIT },
		)
		for (const row of existing) {
			if (!desired.has(row.id)) await PoolBidder.remove(row.id)
		}
		for (const [id, { bidder, direction }] of desired) {
			// Bidders are not guaranteed to appear in the liquidity sweep (zero balances are
			// skipped there), so the provider row is ensured here too.
			if (!(await LiquidityProvider.get(bidder.solver))) {
				await LiquidityProvider.create({ id: bidder.solver }).save()
			}
			await PoolBidder.create({
				id,
				poolId,
				providerId: bidder.solver,
				chain,
				direction,
				outputToken: bidder.outputToken,
				liquidity: bidder.liquidity,
				acceptedSources: bidder.acceptedSources ?? undefined,
				lastUpdatedBlock: blockNumber,
				lastUpdatedAt: snapshotTime,
			}).save()
		}
	}

	for (const entry of keyed.values()) {
		const { poolId, direction } = entry.resolved
		const id = `${poolId}-${chain}-${direction}`
		if (entry.quoted) {
			const depth = entry.samples.reduce((acc, s) => acc + s.depth, 0n)
			// One rate per (chain, direction): depth-weighted across collapsed legs, unweighted
			// when the whole window ran on zero inventory.
			const rate =
				depth > 0n
					? entry.samples.reduce((acc, s) => acc + s.rate * s.depth, 0n) / depth
					: entry.samples.reduce((acc, s) => acc + s.rate, 0n) / BigInt(entry.samples.length)
			const row = await PoolChainLiquidity.get(id)
			if (row) {
				row.rate = rate
				row.depth = depth
				row.bidCount = entry.bidCount
				row.lastUpdatedBlock = blockNumber
				row.lastUpdatedAt = snapshotTime
				await row.save()
			} else {
				await PoolChainLiquidity.create({
					id,
					poolId,
					chain,
					direction,
					rate,
					depth,
					bidCount: entry.bidCount,
					lastUpdatedBlock: blockNumber,
					lastUpdatedAt: snapshotTime,
				}).save()
			}
		} else {
			// No quotes this window: zero the depth but keep the last known rate. Only mutate an
			// existing row — a direction that was never priced has no rate to carry and gets no row.
			const row = await PoolChainLiquidity.get(id)
			if (row) {
				row.depth = 0n
				row.bidCount = 0
				row.lastUpdatedBlock = blockNumber
				row.lastUpdatedAt = snapshotTime
				await row.save()
			}
		}
	}

	for (const poolId of touchedPools.keys()) {
		const pool = await LiquidityPool.get(poolId)
		if (!pool) continue
		const rows = await PoolChainLiquidity.getByPoolId(poolId, { limit: POOL_ROWS_LIMIT })

		for (const direction of [SELL, BUY]) {
			const directionRows = rows.filter((row) => row.direction === direction)
			if (directionRows.length === 0) continue

			const fresh = directionRows.filter((row) => blockNumber - row.lastUpdatedBlock <= MAX_SAMPLE_AGE_BLOCKS)
			const merged = fresh.length > 0 ? fresh : directionRows

			warnOnDivergentSample(poolId, direction, merged)

			const depth = merged.reduce((acc, row) => acc + row.depth, 0n)
			const rate =
				depth > 0n
					? merged.reduce((acc, row) => acc + row.rate * row.depth, 0n) / depth
					: merged.reduce((acc, row) => acc + row.rate, 0n) / BigInt(merged.length)
			const bidCount = merged.reduce((acc, row) => acc + row.bidCount, 0)

			if (direction === SELL) {
				pool.sellRate = rate
				pool.sellDepth = depth
				pool.sellBidCount = bidCount
			} else {
				pool.buyRate = rate
				pool.buyDepth = depth
				pool.buyBidCount = bidCount
			}
		}

		pool.lastUpdatedBlock = blockNumber
		pool.lastUpdatedAt = snapshotTime
		await pool.save()
	}
}

// A pair prices identically across chains, so one sample far off the others' consensus almost
// certainly means a wrong registry decimals entry poisoning that chain's normalization — worth an
// alarm, but the sample still merges: this cannot tell a bad entry from a genuinely dislocated
// market, and silently dropping data would hide the bug the alarm exists to surface.
function warnOnDivergentSample(
	poolId: string,
	direction: string,
	rows: { chain: string; rate: bigint; depth: bigint }[],
): void {
	if (rows.length < 2) return
	for (const row of rows) {
		const others = rows.filter((other) => other !== row)
		const othersDepth = others.reduce((acc, other) => acc + other.depth, 0n)
		const consensus =
			othersDepth > 0n
				? others.reduce((acc, other) => acc + other.rate * other.depth, 0n) / othersDepth
				: others.reduce((acc, other) => acc + other.rate, 0n) / BigInt(others.length)
		if (consensus === 0n) continue
		if (row.rate > consensus * 5n || row.rate * 5n < consensus) {
			logger.warn(
				{ poolId, direction, chain: row.chain, rate: row.rate.toString(), consensus: consensus.toString() },
				"Pool chain sample diverges >5x from the other chains — check the token registry decimals",
			)
		}
	}
}
