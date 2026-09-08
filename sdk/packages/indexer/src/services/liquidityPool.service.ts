// Turns per-leg phantom snapshots into the pair-centric pool entities. A pair has one price
// across chains per direction — solvers rebalance and arbitrage away per-chain drift — so each
// chain's sample only differs in depth, and the pool's single buy/sell number is the
// depth-weighted merge of the chains' latest samples.
//
// Everything in here runs on the Hyperbridge node, and only there. The pool family (LiquidityPool,
// PoolChainLiquidity, PoolBidder, PoolRoute, LiquidityProviderBalanceV2) has that one writer: the
// EVM nodes publish SolverInventoryReading rows instead (`inventoryReading.service.ts`), and
// `foldInventoryReadings` below is how those reach the pool rows. Every SubQuery node keeps a
// process-local read cache that no other node's write invalidates, and flushes whole rows with no
// locking, so two nodes writing one pool row would overwrite each other from stale copies.
import { getPoolToken, poolSlug, sortPoolSymbols } from "@/addresses/pool-tokens.addresses"
import {
	LiquidityPool,
	LiquidityProvider,
	LiquidityProviderBalanceV2,
	PoolBidder,
	PoolChainLiquidity,
	PoolRoute,
	SolverInventoryReading,
} from "@/configs/src/types"
import { readAllPages } from "@/utils/store.helpers"
import { bytes32ToBytes20 } from "@/utils/transfer.helpers"
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
 * `foldInventoryReadings` from the stored rows, and a disagreement between the two would show up as
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
// Readings already folded, keyed by row id and observation time, so a quiet block costs one page
// of the reading table and nothing else. Correctness does not rest on it — a reading is applied
// only if it postdates the bidder row's snapshot and refresh times, which the store records — so a
// restart simply re-examines everything once.
const foldedReadings = new Map<string, number>()

/**
 * Folds the readings the EVM nodes have published into the pool rows: every bidder row backed by
 * a reading newer than the row's own snapshot and refresh times takes the reading's inventory,
 * and every (pool, chain) that changed has its chain row, routes and pool re-derived. Idempotent
 * per reading, so re-running it on the same table is a no-op.
 *
 * Only depths, bid counts and the bidder rows themselves move. `lastUpdatedBlock` and
 * `lastUpdatedAt` are left exactly as the snapshot wrote them: they record which Hyperbridge block
 * priced the pool. Rates are not re-derived either — nothing here observes a new quote — though a
 * pool's merged rate can still shift, because the chains are weighted by the depths this changed.
 *
 * `blockNumber` is the Hyperbridge block being processed: the staleness reference for the pool
 * merge, and the block the balance series rows are keyed by.
 */
export async function foldInventoryReadings(params: { blockNumber: bigint }): Promise<void> {
	const { blockNumber } = params

	// The whole table, paged: the store offers no "newer than" filter, and the set is bounded by
	// bidders times tokens times chains, so the pass is a handful of pages at most.
	const readings = await readAllPages((limit, offset) =>
		SolverInventoryReading.getByFields([], { limit, offset, orderBy: "id", orderDirection: "ASC" }),
	)
	const fresh = readings.filter((reading) => (foldedReadings.get(reading.id) ?? -1) < reading.observedAt.getTime())
	if (fresh.length === 0) return

	const byChain = new Map<string, Map<string, SolverInventoryReading>>()
	for (const reading of fresh) {
		const targets = byChain.get(reading.chain) ?? new Map<string, SolverInventoryReading>()
		byChain.set(reading.chain, targets)
		targets.set(`${reading.provider.toLowerCase()}|${reading.tokenAddress.toLowerCase()}`, reading)
	}

	const touchedPools = new Set<string>()
	const applied = new Map<string, SolverInventoryReading>()
	for (const [chain, targets] of byChain) {
		// Every bidder on the chain, grouped by pool: a pool-chain is the unit that gets republished,
		// and reading it whole is what lets a direction whose bidders all vanished be zeroed.
		const rows = await readAllPages((limit, offset) =>
			PoolBidder.getByFields([["chain", "=", chain]], { limit, offset, orderBy: "id", orderDirection: "ASC" }),
		)
		const byPool = new Map<string, PoolBidder[]>()
		for (const row of rows) byPool.set(row.poolId, [...(byPool.get(row.poolId) ?? []), row])

		for (const [poolId, poolRows] of byPool) {
			const directions = new Set(poolRows.map((row) => row.direction))
			const survivors: PoolBidder[] = []
			let changed = false
			for (const row of poolRows) {
				const reading = targets.get(`${row.providerId.toLowerCase()}|${row.outputToken.toLowerCase()}`)
				if (!reading || !readingIsNewer(reading, row)) {
					survivors.push(row)
					continue
				}
				const liquidity = normalizedLiquidity(chain, row.outputToken, reading.balance)
				if (liquidity === null) {
					survivors.push(row)
					continue
				}
				changed = true
				applied.set(reading.id, reading)
				// Every row is a bidder with capacity, so a bidder holding nothing loses its row.
				if (liquidity === 0n) {
					await PoolBidder.remove(row.id)
					continue
				}
				row.liquidity = liquidity
				row.refreshedAt = reading.observedAt
				await row.save()
				survivors.push(row)
			}
			if (!changed) continue
			await republishChainRows(poolId, chain, survivors, directions)
			touchedPools.add(poolId)
		}
	}

	for (const reading of applied.values()) {
		await recordProviderBalance(reading, blockNumber)
	}

	for (const poolId of touchedPools) {
		const pool = await LiquidityPool.get(poolId)
		if (!pool) continue
		const chainRows = await PoolChainLiquidity.getByPoolId(poolId, { limit: CHAIN_ROWS_LIMIT })
		if (chainRows.length === 0) continue
		mergeChainRowsIntoPool(pool, chainRows, blockNumber)
		await pool.save()
	}

	// Recorded whether or not a reading applied to anything: one older than every row it could
	// back stays older, and one backing no row gains nothing from being looked at again.
	for (const reading of fresh) foldedReadings.set(reading.id, reading.observedAt.getTime())
}

/**
 * Whether a reading postdates everything the row already reflects. The snapshot time is the
 * phantom sweep's own read of this balance; the refresh time is the last reading folded in.
 */
function readingIsNewer(reading: SolverInventoryReading, row: PoolBidder): boolean {
	if (reading.observedAt <= row.lastUpdatedAt) return false
	if (row.refreshedAt && reading.observedAt <= row.refreshedAt) return false
	return true
}

/**
 * A raw balance as the 18-decimal liquidity the pool rows carry, or null when the token is no
 * longer registry-tracked — in which case the row is left as indexed rather than guessed at.
 */
function normalizedLiquidity(chain: string, outputToken: string, balance: bigint): bigint | null {
	const token = getPoolToken(chain, outputToken)
	if (!token || token.decimals > POOL_RATE_DECIMALS) {
		logger.warn(
			{ chain, outputToken },
			"Pool bidder's output token is no longer registry-tracked, leaving it as indexed",
		)
		return null
	}
	return balance * 10n ** BigInt(POOL_RATE_DECIMALS - token.decimals)
}

/**
 * Extends the `LiquidityProviderBalanceV2` series with a folded reading, so a provider's latest
 * balance row does not keep reporting inventory the pool rows already know is spent.
 *
 * Rows are keyed by Hyperbridge block because that is the clock the phantom sweep writes on, and
 * the fold runs on a Hyperbridge block of its own, so the series stays monotonic and "greatest
 * blockNumber is the current balance" stays true. `snapshotTime` is the reading's own observation
 * time. A zero balance is not a row, matching the sweep, which skips tokens a solver does not hold.
 *
 * An existing row for the same key is only ever raised, never lowered, matching the sweep's own
 * rule for two readings landing on one key: a snapshot closing on this same block may have written
 * it first, and the larger of two readings is the complete one.
 */
async function recordProviderBalance(reading: SolverInventoryReading, blockNumber: bigint): Promise<void> {
	if (reading.balance === 0n) return

	const provider = reading.provider.toLowerCase()
	const token = reading.tokenAddress.toLowerCase()
	const id = `${reading.chain}-${token}-${blockNumber}-${provider}`
	const existing = await LiquidityProviderBalanceV2.get(id)
	if (existing) {
		if (reading.balance > existing.balance) {
			existing.balance = reading.balance
			await existing.save()
		}
		return
	}
	await LiquidityProviderBalanceV2.create({
		id,
		providerId: provider,
		chain: reading.chain,
		blockNumber,
		tokenAddress: token,
		balance: reading.balance,
		snapshotTime: reading.observedAt,
	}).save()
}

/**
 * Rewrites one (pool, chain)'s chain rows and routes from its surviving bidder rows, mirroring the
 * reconciliation `updateLiquidityPools` performs on a snapshot. `directions` is the set the chain
 * had bidders on BEFORE the fold, so a direction whose rows all just vanished is zeroed rather than
 * skipped — that is the same "registered but unbacked" state a snapshot writes, keeping the last
 * known rate with no depth behind it.
 */
async function republishChainRows(
	poolId: string,
	chain: string,
	survivors: PoolBidder[],
	directions: Set<string>,
): Promise<void> {
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
