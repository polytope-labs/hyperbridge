// The one home for `SolverV4Positions`: the phantom snapshot writes it, the liquidity refresh reads
// it. A bid's declaration is the only place a Uniswap V4 position is ever named, so this entity is
// the whole bridge between "a solver said it funds fills from these" and any later re-read of them.
import { LiquidityProvider, SolverV4Positions } from "@/configs/src/types"
import type { SolverV4Position } from "@hyperbridge/sdk/intents-helpers"

/**
 * The tokenIds `solver` declared in its latest bid on `chain`, or none.
 *
 * One keyed read, no scan: the row is the solver's current declaration, replaced whole each time it
 * bids. A row recorded on another chain reads as none here — a bid is per chain, and no other
 * chain's reads can see its positions.
 */
export async function declaredV4Positions(chain: string, solver: string): Promise<bigint[]> {
	const declaration = await SolverV4Positions.get(solver.toLowerCase())
	if (!declaration || declaration.chain !== chain) return []
	return declaration.tokenIds
}

/**
 * Records what this bid window's solvers declared: one row per solver, replaced wholesale.
 *
 * `bidders` is every solver that bid, not just the ones with positions, so a solver that bid
 * without declaring anything has its row emptied rather than left standing on a stale declaration —
 * which is exactly how this window's leg weights already treat it. A solver absent from the window
 * keeps its last row: silence is not a withdrawal, and a position it has since parted with reads
 * back as no longer owned when the refresh values it.
 */
export async function recordDeclaredPositions(params: {
	chain: string
	bidders: Iterable<string>
	/** Verified declarations from this window's aggregation, across every solver. */
	positions: SolverV4Position[]
	blockNumber: bigint
	declaredAt: Date
}): Promise<void> {
	const { chain, positions, blockNumber, declaredAt } = params

	const declared = new Map<string, bigint[]>()
	for (const position of positions) {
		const solver = position.solver.toLowerCase()
		declared.set(solver, [...(declared.get(solver) ?? []), position.tokenId])
	}

	// A solver holding nothing anywhere sweeps no balances, so its only trace is what it declared.
	for (const solver of new Set([...params.bidders].map((address) => address.toLowerCase()))) {
		const tokenIds = declared.get(solver) ?? []
		const existing = await SolverV4Positions.get(solver)

		// One row per solver assumes a solver declares on one chain, which holds while Uniswap V4 is
		// configured for a single chain. Overwriting another chain's declaration would silently
		// delete real inventory, so it says so rather than doing it quietly.
		if (existing && existing.chain !== chain) {
			logger.warn(
				{ solver, recordedChain: existing.chain, chain },
				"Solver declared Uniswap V4 positions on two chains — SolverV4Positions needs a (chain, solver) key",
			)
			continue
		}
		if (!existing && tokenIds.length === 0) continue
		if (!(await LiquidityProvider.get(solver))) {
			await LiquidityProvider.create({ id: solver }).save()
		}

		await SolverV4Positions.create({
			id: solver,
			providerId: solver,
			chain,
			tokenIds,
			lastDeclaredBlock: blockNumber,
			lastDeclaredAt: declaredAt,
		}).save()
	}
}
