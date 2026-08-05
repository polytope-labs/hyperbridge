// Turns per-leg phantom snapshots into the pair-centric pool entities. A pair has one price
// across chains per direction — solvers rebalance and arbitrage away per-chain drift — so each
// chain's sample only differs in depth, and the pool's single buy/sell number is the
// depth-weighted merge of the chains' latest samples.
import { getPoolToken, poolSlug, sortPoolSymbols } from "@/addresses/pool-tokens.addresses"
import { LiquidityPool, LiquidityProvider, PoolBidder, PoolChainLiquidity } from "@/configs/src/types"
import { readAllPages } from "@/utils/store.helpers"
import type { PhantomLegAggregation } from "@hyperbridge/sdk/intents-helpers"

export const SELL = "SELL"
export const BUY = "BUY"

/** Pool rates and depths are fixed-point integers with this many decimals. */
export const POOL_RATE_DECIMALS = 18

// Chain samples older than this relative to the block being processed stop influencing the pool
// price, so one quiet chain cannot pin a stale price or depth on a live market — unless EVERY
// sample is stale, in which case last-known values are better than nulling a pool that merely
// paused. Sized as a small multiple of the expected phantom generation interval; the interval is
// governance-configurable, and one longer than this constant makes every sample "stale" — which
// degrades gracefully to that same all-rows fallback rather than dropping data.
const MAX_SAMPLE_AGE_BLOCKS = 1800n

// One row per (chain, direction) and the chain set is config-bounded, so a single page covers it.
const CHAIN_ROWS_LIMIT = 100

/** One registered directed leg of the phantom order, with its tokens as 20-byte lowercase addresses. */
export interface RegisteredLeg {
	legIndex: number
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

/** A registered leg together with its pool attribution (null when not registry-tracked). */
export interface AttributedLeg extends RegisteredLeg {
	resolved: ResolvedPoolLeg | null
}

/**
 * Resolves a phantom leg to its pool identity via the token registry. Null when either token is
 * not registry-tracked, or when the pallet's standard amount disagrees with the registry's input
 * decimals — the standard amount is the denominator of every rate, so a mismatch means either
 * side is misconfigured and any derived rate would be off by an integer factor.
 */
export function resolvePoolLeg(chain: string, leg: RegisteredLeg): ResolvedPoolLeg | null {
	const inputToken = getPoolToken(chain, leg.tokenA)
	const outputToken = getPoolToken(chain, leg.tokenB)
	if (!inputToken || !outputToken || outputToken.decimals > POOL_RATE_DECIMALS) return null

	if (leg.standardAmount !== 10n ** BigInt(inputToken.decimals)) {
		logger.warn(
			{ chain, tokenA: leg.tokenA, standardAmount: leg.standardAmount.toString(), decimals: inputToken.decimals },
			"Phantom leg standard amount disagrees with the registry decimals, skipping pool attribution",
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

/**
 * A pool's rates oriented for `baseSymbol`: `direct` is quote units per 1 whole base (the
 * base -> quote legs), `inverse` is base units per 1 whole quote. Keeping the orientation rule
 * next to the direction writer above means SELL/BUY semantics have a single code home.
 */
export function orientedPoolRates(
	pool: { token0Symbol: string; sellRate?: bigint; buyRate?: bigint },
	baseSymbol: string,
): { direct?: bigint; inverse?: bigint } {
	const baseIsToken0 = pool.token0Symbol === baseSymbol
	return {
		direct: baseIsToken0 ? pool.sellRate : pool.buyRate,
		inverse: baseIsToken0 ? pool.buyRate : pool.sellRate,
	}
}

// The one rate-merge policy: depth-weighted average, falling back to the unweighted mean when
// the whole sample set carries zero depth (so a price is still reported). Every merge in this
// file — collapsed legs, the cross-chain pool merge, and the divergence alarm's consensus —
// must agree on this, hence the single home.
function weightedRate(samples: { rate: bigint; depth: bigint }[]): bigint {
	const depth = samples.reduce((acc, sample) => acc + sample.depth, 0n)
	return depth > 0n
		? samples.reduce((acc, sample) => acc + sample.rate * sample.depth, 0n) / depth
		: samples.reduce((acc, sample) => acc + sample.rate, 0n) / BigInt(samples.length)
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
	/** Every registered leg of the order, resolved once by the caller. */
	legs: AttributedLeg[]
	priced: PhantomLegAggregation[]
}): Promise<void> {
	const { chain, blockNumber, snapshotTime, legs, priced } = params
	const pricedByIndex = new Map(priced.map((leg) => [leg.legIndex, leg]))

	// Accumulate per pool, per direction — the chain is fixed for the whole order. A registered
	// leg nobody quoted is a real zero-liquidity signal, but only when no other leg quoted the
	// same direction (same-asset pairs collapse both token representations onto one direction).
	const pools = new Map<string, Map<string, DirectionSample>>()
	for (const leg of legs) {
		const { resolved } = leg
		if (!resolved) continue

		const directions = pools.get(resolved.poolId) ?? new Map<string, DirectionSample>()
		pools.set(resolved.poolId, directions)
		const entry: DirectionSample = directions.get(resolved.direction) ?? {
			resolved,
			samples: [],
			bidCount: 0,
			bidders: new Map(),
			quoted: false,
		}
		directions.set(resolved.direction, entry)

		const quote = pricedByIndex.get(leg.legIndex)
		if (!quote) continue

		const scale = 10n ** BigInt(POOL_RATE_DECIMALS - resolved.outDecimals)
		let depth = 0n
		for (const bidder of quote.bidders) {
			const solver = bidder.solver.toLowerCase()
			const liquidity = bidder.weight * scale
			depth += liquidity
			entry.bidders.set(`${resolved.poolId}-${chain}-${resolved.direction}-${leg.tokenB}-${solver}`, {
				solver,
				liquidity,
				acceptedSources: bidder.acceptedSources,
				outputToken: leg.tokenB,
			})
		}
		entry.samples.push({ rate: quote.medianPrice * scale, depth })
		entry.bidCount += quote.bidCount
		entry.quoted = true
	}
	if (pools.size === 0) return

	// Bidders are not guaranteed to appear in the liquidity sweep (zero balances are skipped
	// there), so providers are ensured here too — once per distinct solver, not per row.
	const solvers = new Set<string>()
	for (const directions of pools.values()) {
		for (const entry of directions.values()) {
			for (const bidder of entry.bidders.values()) solvers.add(bidder.solver)
		}
	}
	for (const solver of solvers) {
		if (!(await LiquidityProvider.get(solver))) {
			await LiquidityProvider.create({ id: solver }).save()
		}
	}

	// Fetch or create every touched pool once; the merge below mutates and saves these instances.
	const poolRows = new Map<string, LiquidityPool>()
	for (const [poolId, directions] of pools) {
		const { resolved } = directions.values().next().value as DirectionSample
		let pool = await LiquidityPool.get(poolId)
		if (!pool) {
			// Rates stay null until the merge below prices a direction, so a one-way pair never
			// fabricates the other side.
			pool = LiquidityPool.create({
				id: poolId,
				token0Symbol: resolved.token0Symbol,
				token1Symbol: resolved.token1Symbol,
				sellDepth: 0n,
				buyDepth: 0n,
				sellBidCount: 0,
				buyBidCount: 0,
				lastUpdatedBlock: blockNumber,
				lastUpdatedAt: snapshotTime,
			})
			await pool.save()
		}
		poolRows.set(poolId, pool)

		// Reconcile this chain's bidder rows against the ones this window produced. The order
		// carries every configured pair for the chain, so any existing row it did not regenerate
		// belongs to a solver that stopped bidding (if pairs were ever sharded across several
		// orders per chain, this set-diff would wrongly drop the other shard's bidders).
		const desired = new Map<string, { bidder: PoolBidderSample; direction: string }>()
		for (const [direction, entry] of directions) {
			for (const [id, bidder] of entry.bidders) desired.set(id, { bidder, direction })
		}
		// Bidder rows scale with the number of solvers, which nothing bounds — a truncated read
		// here would leave a departed solver's row advertising capacity forever.
		const existing = await readAllPages((limit, offset) =>
			PoolBidder.getByFields(
				[
					["poolId", "=", poolId],
					["chain", "=", chain],
				],
				{ limit, offset, orderBy: "id", orderDirection: "ASC" },
			),
		)
		for (const row of existing) {
			if (!desired.has(row.id)) await PoolBidder.remove(row.id)
		}
		for (const [id, { bidder, direction }] of desired) {
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

		for (const [direction, entry] of directions) {
			const id = `${poolId}-${chain}-${direction}`
			if (entry.quoted) {
				const depth = entry.samples.reduce((acc, sample) => acc + sample.depth, 0n)
				const rate = weightedRate(entry.samples)
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
	}

	for (const [poolId, pool] of poolRows) {
		const rows = await PoolChainLiquidity.getByPoolId(poolId, { limit: CHAIN_ROWS_LIMIT })

		for (const direction of [SELL, BUY]) {
			const directionRows = rows.filter((row) => row.direction === direction)
			if (directionRows.length === 0) continue

			// Blocks are processed in order, so the difference is non-negative in practice; a row
			// from a "future" block would simply count as fresh, which is the right reading anyway.
			const fresh = directionRows.filter((row) => blockNumber - row.lastUpdatedBlock <= MAX_SAMPLE_AGE_BLOCKS)
			const merged = fresh.length > 0 ? fresh : directionRows

			warnOnDivergentSample(poolId, direction, merged)

			const depth = merged.reduce((acc, row) => acc + row.depth, 0n)
			const rate = weightedRate(merged)
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
		const consensus = weightedRate(rows.filter((other) => other !== row))
		if (consensus === 0n) continue
		if (row.rate > consensus * 5n || row.rate * 5n < consensus) {
			logger.warn(
				{ poolId, direction, chain: row.chain, rate: row.rate.toString(), consensus: consensus.toString() },
				"Pool chain sample diverges >5x from the other chains — check the token registry decimals",
			)
		}
	}
}
