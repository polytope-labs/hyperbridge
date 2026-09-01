// Turns per-leg phantom snapshots into the pair-centric pool entities. A pair has one price
// across chains per direction — solvers rebalance and arbitrage away per-chain drift — so each
// chain's sample only differs in depth, and the pool's single buy/sell number is the
// depth-weighted merge of the chains' latest samples.
import { getPoolToken, poolSlug, sortPoolSymbols } from "@/addresses/pool-tokens.addresses"
import {
	LiquidityPool,
	LiquidityProvider,
	LiquidityProviderBalanceV2,
	PoolBidder,
	PoolChainLiquidity,
	PoolRoute,
} from "@/configs/src/types"
import { declaredV4Positions } from "@/services/solverPositions.service"
import { readAllPages } from "@/utils/store.helpers"
import { bytes32ToBytes20 } from "@/utils/transfer.helpers"
import { positionAmountOfToken } from "@hyperbridge/sdk/intents-helpers"
import type { PhantomLegAggregation, SolverBalanceReader, V4PositionState } from "@hyperbridge/sdk/intents-helpers"

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

// Plausibility window for a phantom leg's probe size, expressed against one whole input token:
// at most a million tokens, at least a thousandth of one. See resolvePoolLeg — this is the
// decimals-mismatch tripwire, not a policy limit, so it is deliberately far outside any size the
// pallet would actually configure and well inside the powers of ten a mismatch produces.
const MAX_STANDARD_UNITS = 1_000_000n
const MAX_STANDARD_SUBDIVISIONS = 1_000n

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
	/**
	 * Registry decimals of the leg's INPUT token on this chain. A pool rate is quoted per one
	 * whole input token, but the leg was quoted against `standardAmount` of them, so this is
	 * what lets the rate be renormalized back to one — for any probe size the pallet picks,
	 * whole-token or not.
	 */
	inDecimals: number
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

	// Any probe size is priced correctly — the rate is renormalized by this exact value, so the
	// pallet is free to raise it to buy quote precision (a leg's output integer IS the price, and
	// one whole token of a 6-decimal asset only affords ~3 digits). What is checked is that the
	// size is PLAUSIBLE, because a registry/pallet decimals disagreement shows up as exactly this
	// field being off by a power of ten, and there is no other signal that it happened. Every
	// realistic mismatch (6 vs 18, 6 vs 12, 8 vs 18) is a factor of 1e6 or more, so it falls
	// outside the window while any size anyone would actually probe sits well inside it.
	const inputUnit = 10n ** BigInt(inputToken.decimals)
	const plausible =
		leg.standardAmount > 0n &&
		leg.standardAmount <= inputUnit * MAX_STANDARD_UNITS &&
		leg.standardAmount * MAX_STANDARD_SUBDIVISIONS >= inputUnit
	if (!plausible) {
		logger.warn(
			{ chain, tokenA: leg.tokenA, standardAmount: leg.standardAmount.toString(), decimals: inputToken.decimals },
			"Phantom leg standard amount is implausible for the registry decimals, skipping pool attribution",
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
		inDecimals: inputToken.decimals,
	}
}

/**
 * A leg's quoted output, renormalized to the pool's rate convention: output units per ONE whole
 * input token, as a `POOL_RATE_DECIMALS` fixed-point integer.
 *
 * The quote was given for `standardAmount` of the input token, whatever the pallet configured, so
 * it is scaled back to one whole token. At the legacy probe of exactly one unit
 * (`standardAmount === 10 ** inDecimals`) the two powers cancel and this is precisely the old
 * `medianPrice * scale`, which is why raising the probe size cannot move a published rate.
 *
 * Every multiplication happens before the division, so only the final step truncates — by under
 * one unit of 1e18, and downward, the same conservative direction the filler's own quote is
 * floored in.
 */
export function poolRateFromQuote(
	medianPrice: bigint,
	resolved: Pick<ResolvedPoolLeg, "inDecimals" | "outDecimals">,
	standardAmount: bigint,
): bigint {
	const scale = 10n ** BigInt(POOL_RATE_DECIMALS - resolved.outDecimals)
	const inputUnit = 10n ** BigInt(resolved.inDecimals)
	return (medianPrice * scale * inputUnit) / standardAmount
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

/** One bidder's contribution to a (chain, direction)'s published depth. */
interface BidderLiquidity {
	liquidity: bigint
	/** Null or undefined for a bidder whose bid carried no accepted-source declaration. */
	acceptedSources?: string[] | null
}

/**
 * The depth a set of bidders on one (chain, direction) publishes, and the slice of it behind
 * bidders with no accepted-source declaration — reported separately because those bidders have no
 * PoolRoute rows for consumers to find their capacity under.
 *
 * The one home for that split: `updateLiquidityPools` derives it from a snapshot's bidders and
 * `refreshPoolLiquidity` from the stored rows, and a disagreement between the two would show up as
 * depth stepping between two numbers as fills and snapshots alternate.
 */
function bidderDepths(bidders: BidderLiquidity[]): {
	depth: bigint
	unrestrictedDepth: bigint
	unrestrictedBidCount: number
} {
	let depth = 0n
	let unrestrictedDepth = 0n
	let unrestrictedBidCount = 0
	for (const bidder of bidders) {
		depth += bidder.liquidity
		if (bidder.acceptedSources == null) {
			unrestrictedDepth += bidder.liquidity
			unrestrictedBidCount += 1
		}
	}
	return { depth, unrestrictedDepth, unrestrictedBidCount }
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
		entry.samples.push({
			rate: poolRateFromQuote(quote.medianPrice, resolved, leg.standardAmount),
			depth,
		})
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

		// Materialize each direction's explicit declarations into searchable routes, reconciled
		// with the same set-diff so a route disappears with its last declaring bidder. A bidder
		// with no declaration contributes to no route (its capacity is reported as the
		// chain-liquidity row's unrestricted slice); one with an empty declaration accepts
		// nothing and contributes to neither.
		const desiredRoutes = new Map<
			string,
			{ direction: string; sourceChain: string; depth: bigint; bidCount: number }
		>()
		for (const [direction, entry] of directions) {
			for (const bidder of entry.bidders.values()) {
				if (!bidder.acceptedSources) continue
				for (const sourceChain of new Set(bidder.acceptedSources)) {
					const id = `${poolId}-${chain}-${direction}-${sourceChain}`
					const route = desiredRoutes.get(id) ?? { direction, sourceChain, depth: 0n, bidCount: 0 }
					route.depth += bidder.liquidity
					route.bidCount += 1
					desiredRoutes.set(id, route)
				}
			}
		}
		// Route rows scale with bidders times declared sources, which nothing bounds.
		const existingRoutes = await readAllPages((limit, offset) =>
			PoolRoute.getByFields(
				[
					["poolId", "=", poolId],
					["chain", "=", chain],
				],
				{ limit, offset, orderBy: "id", orderDirection: "ASC" },
			),
		)
		for (const row of existingRoutes) {
			if (!desiredRoutes.has(row.id)) await PoolRoute.remove(row.id)
		}
		for (const [id, route] of desiredRoutes) {
			await PoolRoute.create({
				id,
				poolId,
				chain,
				direction: route.direction,
				sourceChain: route.sourceChain,
				depth: route.depth,
				bidCount: route.bidCount,
				lastUpdatedBlock: blockNumber,
				lastUpdatedAt: snapshotTime,
			}).save()
		}

		for (const [direction, entry] of directions) {
			const id = `${poolId}-${chain}-${direction}`
			if (entry.quoted) {
				const rate = weightedRate(entry.samples)
				const { depth, unrestrictedDepth, unrestrictedBidCount } = bidderDepths([...entry.bidders.values()])
				const row = await PoolChainLiquidity.get(id)
				if (row) {
					row.rate = rate
					row.depth = depth
					row.bidCount = entry.bidCount
					row.unrestrictedDepth = unrestrictedDepth
					row.unrestrictedBidCount = unrestrictedBidCount
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
						unrestrictedDepth,
						unrestrictedBidCount,
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
					row.unrestrictedDepth = 0n
					row.unrestrictedBidCount = 0
					row.lastUpdatedBlock = blockNumber
					row.lastUpdatedAt = snapshotTime
					await row.save()
				}
			}
		}
	}

	for (const [poolId, pool] of poolRows) {
		const rows = await PoolChainLiquidity.getByPoolId(poolId, { limit: CHAIN_ROWS_LIMIT })
		mergeChainRowsIntoPool(pool, rows, blockNumber)

		pool.lastUpdatedBlock = blockNumber
		pool.lastUpdatedAt = snapshotTime
		await pool.save()
	}
}

/**
 * Collapses a pool's per-(chain, direction) rows into its single buy/sell rate, depth and bid
 * count, in place. `referenceBlock` is the Hyperbridge block the sample ages are measured
 * against — the block being processed when a snapshot writes, the freshest row when a fill
 * refresh does, which is the same reading either way.
 *
 * Does not touch the pool's own `lastUpdatedBlock`/`lastUpdatedAt` or save it: those record which
 * snapshot last priced the pool, and a caller that is not a snapshot must leave them alone.
 */
function mergeChainRowsIntoPool(pool: LiquidityPool, rows: PoolChainLiquidity[], referenceBlock: bigint): void {
	for (const direction of [SELL, BUY]) {
		const directionRows = rows.filter((row) => row.direction === direction)
		if (directionRows.length === 0) continue

		// Blocks are processed in order, so the difference is non-negative in practice; a row
		// from a "future" block would simply count as fresh, which is the right reading anyway.
		const fresh = directionRows.filter((row) => referenceBlock - row.lastUpdatedBlock <= MAX_SAMPLE_AGE_BLOCKS)
		const merged = fresh.length > 0 ? fresh : directionRows

		warnOnDivergentSample(pool.id, direction, merged)

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

/**
 * The pools an order's fill traded through: every input token's symbol on the source chain paired
 * with every output token's symbol on the destination chain. A token the registry does not track
 * contributes no pool, so an order in unrelated assets resolves to nothing and costs nothing.
 *
 * The pair is resolved from the two chains separately because that is where the addresses live —
 * the inputs are escrowed on the source chain, the outputs delivered on the destination one — and
 * a symbol's decimals differ per chain. Same-symbol pairs are the same-asset pool, exactly as
 * `resolvePoolLeg` treats them.
 */
export function poolsForFill(params: {
	sourceChain: string
	inputTokens: string[]
	destChain: string
	outputTokens: string[]
}): string[] {
	const symbols = (chain: string, tokens: string[]) => [
		...new Set(
			tokens
				.map((token) => getPoolToken(chain, bytes32ToBytes20(token))?.symbol)
				.filter((symbol): symbol is string => !!symbol),
		),
	]
	const inputs = symbols(params.sourceChain, params.inputTokens)
	const outputs = symbols(params.destChain, params.outputTokens)

	const pools = new Set<string>()
	for (const input of inputs) {
		for (const output of outputs) pools.add(poolSlug(input, output))
	}
	return [...pools]
}

/**
 * Everything a refresh needs from outside the store: how to reach each chain, how to read a
 * balance and a position there, and what clock to file the balance rows under.
 *
 * The readers are injected rather than built here so the caller can pin them to the block of the
 * event that triggered the refresh — a balance read at the event's own block is the same value on
 * a replay as it was live, which a read at the chain head is not.
 */
export interface LiquidityRefreshContext {
	/** HTTP RPC per state machine id; a chain absent here is left as the snapshot wrote it. */
	evmRpcUrls: Record<string, string>
	getBalance: SolverBalanceReader
	/**
	 * Reads one declared Uniswap V4 position, or null when it no longer exists. Chains with no
	 * configured V4 deployment always read null, which is simply a solver with no position value.
	 */
	readPosition: V4PositionReader
	/**
	 * Hyperbridge's head block, the clock `LiquidityProviderBalanceV2` is keyed by. Null skips the
	 * balance rows — the pool entities are the ones consumers size orders against, so they are
	 * refreshed whether or not the series can be extended.
	 */
	headBlock: () => Promise<bigint | null>
	/**
	 * Wall-clock time of the event that moved the inventory. A pool whose last snapshot already
	 * postdates it is left alone: that snapshot read balances this event had already moved, so
	 * re-reading would replace fresher data with a partial view of it. This is also what keeps a
	 * historical resync cheap — every replayed event is older than the pools' current samples.
	 */
	observedAt: Date
}

/** Reads one Uniswap V4 position on a chain; null when it does not exist (burned, or never minted). */
export type V4PositionReader = (chain: string, tokenId: bigint) => Promise<V4PositionState | null>

/**
 * Re-reads the on-chain inventory of every LP recorded as backing `poolIds` and republishes the
 * pools' depths from it. Called when a fill has just consumed some of that inventory, so the depth
 * a taker reads reflects what the solvers still hold rather than what they held when the last
 * phantom bid window closed.
 */
export async function refreshPoolLiquidity(params: { poolIds: string[] } & LiquidityRefreshContext): Promise<void> {
	// Bidder rows scale with the number of solvers, which nothing bounds, and a truncated read
	// would look exactly like the missing solvers having withdrawn.
	const bidders: PoolBidder[] = []
	for (const poolId of new Set(params.poolIds)) {
		bidders.push(
			...(await readAllPages((limit, offset) =>
				PoolBidder.getByFields([["poolId", "=", poolId]], {
					limit,
					offset,
					orderBy: "id",
					orderDirection: "ASC",
				}),
			)),
		)
	}
	await refreshBidders(bidders, params)
}

/**
 * The same refresh, selected by provider instead of by pool: every pool this solver backs on
 * `chain` with one of `tokens` as its output token.
 *
 * This is the entry point for the events that move a solver's inventory without naming a pool — an
 * escrow release paying a filler back on the source chain, or a vault deposit or withdrawal moving
 * inventory between the raw and vault halves of the same total (and changing that total outright
 * when the counterparty is someone else).
 */
export async function refreshProviderLiquidity(
	params: {
		chain: string
		provider: string
		/** Token addresses whose inventory moved; rows for any other output token are untouched. */
		tokens: string[]
	} & LiquidityRefreshContext,
): Promise<void> {
	const wanted = new Set(params.tokens.map((token) => token.toLowerCase()))
	if (wanted.size === 0) return

	// Rows per solver per chain are bounded by the pools it backs, but nothing declares that bound.
	const rows = await readAllPages((limit, offset) =>
		PoolBidder.getByFields(
			[
				["providerId", "=", params.provider],
				["chain", "=", params.chain],
			],
			{ limit, offset, orderBy: "id", orderDirection: "ASC" },
		),
	)
	await refreshBidders(
		rows.filter((row) => wanted.has(row.outputToken.toLowerCase())),
		params,
	)
}

/**
 * Re-reads `bidders`, writes what they now hold, and republishes every pool row derived from them.
 *
 * Only depths, bid counts and the bidder rows themselves move. `lastUpdatedBlock` and
 * `lastUpdatedAt` are left exactly as the snapshot wrote them everywhere: they record which
 * Hyperbridge block priced the pool, and an EVM block number of another chain is not comparable
 * with them (writing one would make every row look astronomically fresh or stale to
 * `MAX_SAMPLE_AGE_BLOCKS`). Rates are not re-derived either — nothing here observes a new quote —
 * though a pool's merged rate can still shift, because the chains are weighted by the depths this
 * refresh just changed.
 *
 * `bidders` may be a subset of a pool-chain's bidders (the provider-scoped entry point passes one
 * solver's rows), so the depths are always re-summed from the stored rows rather than from the
 * subset that was re-read.
 */
async function refreshBidders(bidders: PoolBidder[], ctx: LiquidityRefreshContext): Promise<void> {
	// Group by pool, then by chain: a chain is the unit that can fail, since its RPC is one
	// endpoint and a partially read chain must not be republished — the unread bidders would look
	// departed — while a pool is the unit that gets re-merged at the end.
	const byPool = new Map<string, Map<string, PoolBidder[]>>()
	for (const row of bidders) {
		const chains = byPool.get(row.poolId) ?? new Map<string, PoolBidder[]>()
		byPool.set(row.poolId, chains)
		chains.set(row.chain, [...(chains.get(row.chain) ?? []), row])
	}

	for (const [poolId, chains] of byPool) {
		const pool = await LiquidityPool.get(poolId)
		if (!pool) continue
		if (pool.lastUpdatedAt > ctx.observedAt) continue

		let refreshed = false
		for (const [chain, rows] of chains) {
			const readings = await readChainLiquidity(chain, rows, ctx)
			if (!readings) continue
			await writeChainLiquidity(poolId, chain, readings)
			// The balance series is secondary to the pool rows and keyed on another chain's clock,
			// so it is extended after them and never in their way.
			await recordProviderBalances(chain, rows, readings, await ctx.headBlock(), ctx.observedAt)
			refreshed = true
		}
		if (!refreshed) continue

		const chainRows = await PoolChainLiquidity.getByPoolId(poolId, { limit: CHAIN_ROWS_LIMIT })
		if (chainRows.length === 0) continue
		// The freshest row defines "now" for the staleness window. The block being processed
		// cannot: it belongs to an EVM chain, and these rows are stamped with Hyperbridge blocks.
		const referenceBlock = chainRows.reduce(
			(newest, row) => (row.lastUpdatedBlock > newest ? row.lastUpdatedBlock : newest),
			0n,
		)
		mergeChainRowsIntoPool(pool, chainRows, referenceBlock)
		await pool.save()
	}
}

/** One bidder row's re-read inventory: raw token units, and the same value 18-decimal normalized. */
interface BidderReading {
	balance: bigint
	liquidity: bigint
}

/**
 * Each bidder row's current inventory on one chain, or null when the chain cannot be read in full.
 * Null is deliberately all-or-nothing: a bidder whose balance failed to read is indistinguishable
 * from one holding zero, so a partial result would publish a depth that is short by however many
 * reads happened to fail.
 *
 * Inventory here is the same total the phantom sweep weights a bid by — wallet ERC-20, redeemable
 * ERC-4626 vault positions, and the Uniswap V4 positions the solver declared. The declaration is the
 * solver's `SolverV4Positions` row, recorded when it last bid, since a bid is the only place a
 * position is ever named; each tokenId is then re-read on-chain rather than carried forward at its
 * last value, because simplex funds fills out of these positions and a carried value would keep
 * advertising the inventory a fill just spent.
 */
async function readChainLiquidity(
	chain: string,
	rows: PoolBidder[],
	ctx: LiquidityRefreshContext,
): Promise<Map<string, BidderReading> | null> {
	const evmRpcUrl = ctx.evmRpcUrls[chain]
	if (!evmRpcUrl) return null

	const positionsByProvider = new Map<string, V4PositionState[]>()
	for (const provider of new Set(rows.map((row) => row.providerId))) {
		const live: V4PositionState[] = []
		for (const tokenId of await declaredV4Positions(chain, provider)) {
			let state: V4PositionState | null
			try {
				state = await ctx.readPosition(chain, tokenId)
			} catch (err) {
				logger.warn(
					{ err, chain, tokenId: tokenId.toString() },
					"Failed to re-read a declared Uniswap V4 position, leaving the chain's liquidity as indexed",
				)
				return null
			}
			// Burned, or sold to someone else: either way it is no longer this solver's inventory —
			// and the recorded declaration cannot know that, so the owner check is what carries it.
			if (!state || state.owner.toLowerCase() !== provider.toLowerCase()) continue
			live.push(state)
		}
		positionsByProvider.set(provider, live)
	}

	const readings = new Map<string, BidderReading>()
	for (const row of rows) {
		const token = getPoolToken(chain, row.outputToken)
		if (!token || token.decimals > POOL_RATE_DECIMALS) {
			logger.warn(
				{ chain, outputToken: row.outputToken },
				"Pool bidder's output token is no longer registry-tracked, leaving the chain's liquidity as indexed",
			)
			return null
		}
		let balance: bigint
		try {
			balance = await ctx.getBalance(evmRpcUrl, chain, row.outputToken, row.providerId)
		} catch (err) {
			logger.warn(
				{ err, chain, solver: row.providerId, outputToken: row.outputToken },
				"Failed to re-read a pool bidder's balance, leaving the chain's liquidity as indexed",
			)
			return null
		}
		// A position pays out in whichever of its two currencies this leg delivers; one that has
		// moved entirely onto the other side of the price is worth zero here, which is ordinary.
		const inPositions = (positionsByProvider.get(row.providerId) ?? []).reduce(
			(total, state) =>
				total +
				positionAmountOfToken({
					info: state.info,
					liquidity: state.liquidity,
					sqrtPriceX96: state.sqrtPriceX96,
					outputToken: row.outputToken,
				}),
			0n,
		)
		const total = balance + inPositions
		readings.set(row.id, { balance: total, liquidity: total * 10n ** BigInt(POOL_RATE_DECIMALS - token.decimals) })
	}
	return readings
}

/**
 * Extends the `LiquidityProviderBalanceV2` series with what this refresh just read, so a provider's
 * latest balance row does not keep reporting inventory the pool rows already know is spent.
 *
 * Rows are keyed by Hyperbridge block because that is the clock the phantom sweep writes on; an
 * event borrows Hyperbridge's head, which keeps the series monotonic and "greatest blockNumber is
 * the current balance" true. A zero balance is not a row, matching the sweep, which skips tokens a
 * solver does not hold.
 *
 * An existing row for the same key is only ever raised, never lowered, matching the sweep's own
 * rule for two readings landing on one key. The cost is that a second event while Hyperbridge is
 * still on the same block does not lower the row; one block later it does.
 *
 * Future improvement (#1159 §3, deliberately not done because it changes the schema of a live
 * entity): give the entity a `trigger` enum and a nullable `transactionHash`, key event-triggered
 * rows `{chain}-{token}-{solver}-{blockNumber}` on the EVM block the event was read at, and order
 * "current liquidity" by `snapshotTime` rather than `blockNumber`, since the two block spaces are
 * not comparable. That removes the borrowed clock and makes each row say what produced it.
 */
async function recordProviderBalances(
	chain: string,
	rows: PoolBidder[],
	readings: Map<string, BidderReading>,
	blockNumber: bigint | null,
	snapshotTime: Date,
): Promise<void> {
	if (blockNumber === null) return

	for (const row of rows) {
		const balance = readings.get(row.id)?.balance ?? 0n
		if (balance === 0n) continue

		const id = `${chain}-${row.outputToken}-${blockNumber}-${row.providerId}`
		const existing = await LiquidityProviderBalanceV2.get(id)
		if (existing) {
			if (balance > existing.balance) {
				existing.balance = balance
				await existing.save()
			}
			continue
		}
		await LiquidityProviderBalanceV2.create({
			id,
			providerId: row.providerId,
			chain,
			blockNumber,
			tokenAddress: row.outputToken,
			balance,
			snapshotTime,
		}).save()
	}
}

/**
 * Writes one chain's re-read liquidity through the rows derived from it, mirroring the
 * reconciliation `updateLiquidityPools` performs on a snapshot: a bidder holding nothing loses its
 * row (every row is a bidder with capacity, so the row set can be counted as well as summed), and
 * a route keeps only the bidders that survived.
 *
 * The reading need not cover every bidder on this (pool, chain) — the provider-scoped entry point
 * re-reads one solver's rows — so the depths are re-summed over all of them, with the rows this
 * refresh did not touch contributing exactly what they already did.
 */
async function writeChainLiquidity(poolId: string, chain: string, readings: Map<string, BidderReading>): Promise<void> {
	// Everything backing this (pool, chain), read BEFORE the writes below: the published depth is a
	// sum over all of them, not just the rows this refresh re-read (the provider-scoped entry point
	// re-reads one solver's), and reading it back afterwards would rest on the store reflecting an
	// in-block removal.
	const all = await readAllPages((limit, offset) =>
		PoolBidder.getByFields(
			[
				["poolId", "=", poolId],
				["chain", "=", chain],
			],
			{ limit, offset, orderBy: "id", orderDirection: "ASC" },
		),
	)

	const survivors: PoolBidder[] = []
	for (const row of all) {
		const current = readings.get(row.id)
		// Not re-read this time: whatever the last snapshot or refresh left it at still stands.
		if (!current) {
			survivors.push(row)
			continue
		}
		if (current.liquidity === 0n) {
			await PoolBidder.remove(row.id)
			continue
		}
		if (current.liquidity !== row.liquidity) {
			row.liquidity = current.liquidity
			await row.save()
		}
		survivors.push(row)
	}

	// The directions this chain had bidders on before the refresh, so a direction whose rows all
	// just vanished is zeroed rather than skipped — that is the same "registered but unbacked" state
	// a snapshot writes, keeping the last known rate with no depth behind it.
	const directions = new Set(all.map((row) => row.direction))
	for (const direction of directions) {
		const chainRow = await PoolChainLiquidity.get(`${poolId}-${chain}-${direction}`)
		if (!chainRow) continue
		const directionBidders = survivors.filter((row) => row.direction === direction)
		const { depth, unrestrictedDepth, unrestrictedBidCount } = bidderDepths(directionBidders)
		chainRow.depth = depth
		// Counting rows, which is what the snapshot's bid count also amounts to: it counts backed
		// quotes, and every backed quote wrote exactly one of these rows.
		chainRow.bidCount = directionBidders.length
		chainRow.unrestrictedDepth = unrestrictedDepth
		chainRow.unrestrictedBidCount = unrestrictedBidCount
		await chainRow.save()
	}

	// Routes are derived from declarations, which no balance read can change, so the surviving
	// bidders can only shrink the set the snapshot wrote. Existing rows are therefore updated or
	// removed and never created: a route with no row is one no snapshot published, and inventing
	// it here would date it to a bid window this indexer never saw.
	const desired = new Map<string, { depth: bigint; bidCount: number }>()
	for (const bidder of survivors) {
		if (!bidder.acceptedSources) continue
		for (const sourceChain of new Set(bidder.acceptedSources)) {
			const id = `${poolId}-${chain}-${bidder.direction}-${sourceChain}`
			const route = desired.get(id) ?? { depth: 0n, bidCount: 0 }
			route.depth += bidder.liquidity
			route.bidCount += 1
			desired.set(id, route)
		}
	}
	// Route rows scale with bidders times declared sources, which nothing bounds.
	const existingRoutes = await readAllPages((limit, offset) =>
		PoolRoute.getByFields(
			[
				["poolId", "=", poolId],
				["chain", "=", chain],
			],
			{ limit, offset, orderBy: "id", orderDirection: "ASC" },
		),
	)
	for (const row of existingRoutes) {
		const route = desired.get(row.id)
		if (!route) {
			await PoolRoute.remove(row.id)
			continue
		}
		row.depth = route.depth
		row.bidCount = route.bidCount
		await row.save()
	}
}
