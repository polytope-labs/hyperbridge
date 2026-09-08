// The EVM-side half of keeping pool depth honest between phantom snapshots. An event on this
// chain moved a solver's inventory, so the solver's balance is re-read here — pinned to the
// event's block, on the one chain this node has an RPC for — and published as a
// SolverInventoryReading row that only this chain's node ever writes. The pool rows themselves are
// recomputed from those readings by the Hyperbridge node (`foldInventoryReadings`), the single
// writer of the pool family, so no two nodes ever rewrite the same pool row.
import { getPoolToken } from "@/addresses/pool-tokens.addresses"
import { InventoryReadingTrigger, PoolBidder, SolverInventoryReading } from "@/configs/src/types"
import { POOL_RATE_DECIMALS } from "@/services/liquidityPool.service"
import { declaredV4Positions } from "@/services/solverPositions.service"
import { readAllPages } from "@/utils/store.helpers"
import { positionAmountOfToken } from "@hyperbridge/sdk/intents-helpers"
import type { SolverBalanceReader, V4PositionState } from "@hyperbridge/sdk/intents-helpers"

/** Reads one Uniswap V4 position on a chain; null when it does not exist (burned, or never minted). */
export type V4PositionReader = (chain: string, tokenId: bigint) => Promise<V4PositionState | null>

export interface InventoryReadContext {
	/** State machine ID of the chain the event happened on — the only chain read here. */
	chain: string
	/** HTTP RPC of `chain`; undefined leaves every bidder as indexed, since nothing can be read. */
	evmRpcUrl: string | undefined
	getBalance: SolverBalanceReader
	/**
	 * Reads one declared Uniswap V4 position, or null when it no longer exists. Chains with no
	 * configured V4 deployment always read null, which is simply a solver with no position value.
	 */
	readPosition: V4PositionReader
	/** Block of `chain` the reads are pinned to. */
	blockNumber: bigint
	/**
	 * Wall-clock time of the event that moved the inventory. A bidder row already sampled or
	 * refreshed after it is left alone: that reading saw balances this event had already moved, so
	 * re-reading would replace fresher data with an older view of it. This is also what keeps a
	 * historical resync cheap — every replayed event is older than the rows' current samples.
	 */
	observedAt: Date
	trigger: InventoryReadingTrigger
}

/**
 * Re-reads the inventory of every LP recorded as backing `poolIds` on this chain and publishes
 * the readings. Called when a fill has just consumed some of that inventory.
 */
export async function publishPoolInventory(params: { poolIds: string[] } & InventoryReadContext): Promise<void> {
	// Bidder rows scale with the number of solvers, which nothing bounds, and a truncated read
	// would look exactly like the missing solvers having withdrawn.
	const rows: PoolBidder[] = []
	for (const poolId of new Set(params.poolIds)) {
		rows.push(
			...(await readAllPages((limit, offset) =>
				PoolBidder.getByFields(
					[
						["poolId", "=", poolId],
						["chain", "=", params.chain],
					],
					{ limit, offset, orderBy: "id", orderDirection: "ASC" },
				),
			)),
		)
	}
	await publishReadings(rows, params)
}

/**
 * The same publication, selected by provider instead of by pool: every pool this solver backs on
 * this chain with one of `tokens` as its output token.
 *
 * This is the entry point for the events that move a solver's inventory without naming a pool — an
 * escrow release paying a filler back on the source chain, or a vault deposit or withdrawal moving
 * inventory between the raw and vault halves of the same total (and changing that total outright
 * when the counterparty is someone else).
 */
export async function publishProviderInventory(
	params: {
		provider: string
		/** Token addresses whose inventory moved; rows for any other output token are untouched. */
		tokens: string[]
	} & InventoryReadContext,
): Promise<void> {
	const wanted = new Set(params.tokens.map((token) => token.toLowerCase()))
	if (wanted.size === 0) return

	// Rows per solver per chain are bounded by the pools it backs, but nothing declares that bound.
	const rows = await readAllPages((limit, offset) =>
		PoolBidder.getByFields(
			[
				["providerId", "=", params.provider.toLowerCase()],
				["chain", "=", params.chain],
			],
			{ limit, offset, orderBy: "id", orderDirection: "ASC" },
		),
	)
	await publishReadings(
		rows.filter((row) => wanted.has(row.outputToken.toLowerCase())),
		params,
	)
}

/** One (solver, token) inventory to read: the bidder rows it backs share a single balance. */
interface ReadTarget {
	provider: string
	token: string
}

/**
 * Reads each distinct (solver, output token) behind `rows` and writes one reading per target.
 *
 * All-or-nothing: a bidder whose balance failed to read is indistinguishable from one holding
 * zero, and the fold drops a bidder that reads as zero, so a partial publication would report
 * the unread bidders as departed. Any failed read abandons every target untouched.
 *
 * Inventory here is the same total the phantom sweep weights a bid by — wallet ERC-20, redeemable
 * ERC-4626 vault positions, and the Uniswap V4 positions the solver declared. The declaration is
 * the solver's `SolverV4Positions` row, recorded when it last bid, since a bid is the only place a
 * position is ever named; each tokenId is then re-read on-chain rather than carried forward at its
 * last value, because simplex funds fills out of these positions and a carried value would keep
 * advertising the inventory a fill just spent.
 */
async function publishReadings(rows: PoolBidder[], ctx: InventoryReadContext): Promise<void> {
	if (!ctx.evmRpcUrl) return

	const targets = new Map<string, ReadTarget>()
	for (const row of rows) {
		// A row sampled or refreshed after this event already reflects what it moved; when every
		// row a target backs is that fresh, the read would be spent on nothing.
		if (row.lastUpdatedAt > ctx.observedAt) continue
		if (row.refreshedAt && row.refreshedAt > ctx.observedAt) continue
		const token = row.outputToken.toLowerCase()
		const registered = getPoolToken(ctx.chain, token)
		if (!registered || registered.decimals > POOL_RATE_DECIMALS) {
			logger.warn(
				{ chain: ctx.chain, outputToken: token },
				"Pool bidder's output token is no longer registry-tracked, leaving it as indexed",
			)
			continue
		}
		targets.set(`${row.providerId}|${token}`, { provider: row.providerId, token })
	}
	if (targets.size === 0) return

	const positionsByProvider = new Map<string, V4PositionState[]>()
	for (const provider of new Set([...targets.values()].map((target) => target.provider))) {
		const live: V4PositionState[] = []
		for (const tokenId of await declaredV4Positions(ctx.chain, provider)) {
			let state: V4PositionState | null
			try {
				state = await ctx.readPosition(ctx.chain, tokenId)
			} catch (err) {
				logger.warn(
					{ err, chain: ctx.chain, tokenId: tokenId.toString() },
					"Failed to re-read a declared Uniswap V4 position, leaving the chain's liquidity as indexed",
				)
				return
			}
			// Burned, or sold to someone else: either way it is no longer this solver's inventory —
			// and the recorded declaration cannot know that, so the owner check is what carries it.
			if (!state || state.owner.toLowerCase() !== provider.toLowerCase()) continue
			live.push(state)
		}
		positionsByProvider.set(provider, live)
	}

	const balances = new Map<string, bigint>()
	for (const [key, target] of targets) {
		let balance: bigint
		try {
			balance = await ctx.getBalance(ctx.evmRpcUrl, ctx.chain, target.token, target.provider)
		} catch (err) {
			logger.warn(
				{ err, chain: ctx.chain, solver: target.provider, outputToken: target.token },
				"Failed to re-read a pool bidder's balance, leaving the chain's liquidity as indexed",
			)
			return
		}
		// A position pays out in whichever of its two currencies this leg delivers; one that has
		// moved entirely onto the other side of the price is worth zero here, which is ordinary.
		const inPositions = (positionsByProvider.get(target.provider) ?? []).reduce(
			(total, state) =>
				total +
				positionAmountOfToken({
					info: state.info,
					liquidity: state.liquidity,
					sqrtPriceX96: state.sqrtPriceX96,
					outputToken: target.token,
				}),
			0n,
		)
		balances.set(key, balance + inPositions)
	}

	for (const [key, target] of targets) {
		await SolverInventoryReading.create({
			id: `${ctx.chain}-${target.token}-${target.provider}`,
			chain: ctx.chain,
			provider: target.provider,
			tokenAddress: target.token,
			balance: balances.get(key) ?? 0n,
			blockNumber: ctx.blockNumber,
			observedAt: ctx.observedAt,
			trigger: ctx.trigger,
		}).save()
	}
}
