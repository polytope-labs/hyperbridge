// The Hyperbridge-side half of the fill-driven pool refresh: what folding the EVM nodes' inventory
// readings does to the rows a taker reads. The fold is the only writer of those rows, and it runs
// every block, so the properties that matter are that a reading applies exactly once, that a
// stale one never applies, and that re-running the fold on the same table changes nothing.

;(global as any).logger = { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() }

const records = new Map<string, any>()
;(global as any).store = {
	get: jest.fn(async (entity: string, id: string) => records.get(`${entity}:${id}`)),
	set: jest.fn(async (entity: string, id: string, props: any) => {
		records.set(`${entity}:${id}`, { ...props })
	}),
	remove: jest.fn(async (entity: string, id: string) => {
		records.delete(`${entity}:${id}`)
	}),
	getByField: jest.fn(async (entity: string, field: string, value: any, options: any = {}) =>
		page(
			rowsOf(entity).filter((row) => row[field] === value),
			options,
		),
	),
	getByFields: jest.fn(async (entity: string, filter: [string, string, any][], options: any = {}) =>
		page(
			sort(
				rowsOf(entity).filter((row) => filter.every(([field, , value]) => row[field] === value)),
				options,
			),
			options,
		),
	),
}

const rowsOf = (entity: string) =>
	[...records.entries()]
		.filter(([key]) => key.startsWith(`${entity}:`))
		.map(([, props]) => props)
		.sort((a, b) => (a.id < b.id ? -1 : 1))

const page = (rows: any[], options: { limit?: number; offset?: number }) =>
	rows.slice(options.offset ?? 0, (options.offset ?? 0) + (options.limit ?? rows.length))

const sort = (rows: any[], options: { orderBy?: string; orderDirection?: string }) => {
	if (!options.orderBy) return rows
	const direction = options.orderDirection === "DESC" ? -1 : 1
	return [...rows].sort((a, b) => (a[options.orderBy!] < b[options.orderBy!] ? -direction : direction))
}

import type { foldInventoryReadings as Fold } from "@/services/liquidityPool.service"

const BASE = "EVM-8453"
const ETHEREUM = "EVM-1"
const USDC_BASE = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
const POOL = "cNGN-USDC"
const SOLVER_A = "0x1111111111111111111111111111111111111111"
const SOLVER_B = "0x2222222222222222222222222222222222222222"

const SCALE = 10n ** 12n
const usdc = (whole: bigint) => whole * 10n ** 6n

// The pool's rows were written by a Hyperbridge snapshot at this block and time; the reading that
// gets folded was observed after it, and the fold itself runs a few Hyperbridge blocks later.
const SNAPSHOT_BLOCK = 4_200_000n
const SNAPSHOT_TIME = new Date("2026-08-01T12:00:00Z")
const FILL_TIME = new Date("2026-08-01T12:00:30Z")
const FILL_BLOCK = 30_000_000n
const FOLD_BLOCK = SNAPSHOT_BLOCK + 12n
const RATE = 715_000_000_000_000_000n

function plantPool(): void {
	records.set(`LiquidityPool:${POOL}`, {
		id: POOL,
		token0Symbol: "cNGN",
		token1Symbol: "USDC",
		sellRate: RATE,
		sellDepth: usdc(1500n) * SCALE,
		sellBidCount: 2,
		buyDepth: 0n,
		buyBidCount: 0,
		lastUpdatedBlock: SNAPSHOT_BLOCK,
		lastUpdatedAt: SNAPSHOT_TIME,
	})
	records.set(`PoolChainLiquidity:${POOL}-${BASE}-SELL`, {
		id: `${POOL}-${BASE}-SELL`,
		poolId: POOL,
		chain: BASE,
		direction: "SELL",
		rate: RATE,
		depth: usdc(1500n) * SCALE,
		bidCount: 2,
		unrestrictedDepth: usdc(500n) * SCALE,
		unrestrictedBidCount: 1,
		lastUpdatedBlock: SNAPSHOT_BLOCK,
		lastUpdatedAt: SNAPSHOT_TIME,
	})
	// Solver A declares Ethereum as an accepted source; solver B declares nothing, so its capacity
	// is only reachable through the chain row's unrestricted slice.
	plantBidder(SOLVER_A, usdc(1000n), [ETHEREUM])
	plantBidder(SOLVER_B, usdc(500n), undefined)
	records.set(`PoolRoute:${POOL}-${BASE}-SELL-${ETHEREUM}`, {
		id: `${POOL}-${BASE}-SELL-${ETHEREUM}`,
		poolId: POOL,
		chain: BASE,
		direction: "SELL",
		sourceChain: ETHEREUM,
		depth: usdc(1000n) * SCALE,
		bidCount: 1,
		lastUpdatedBlock: SNAPSHOT_BLOCK,
		lastUpdatedAt: SNAPSHOT_TIME,
	})
}

function plantBidder(
	solver: string,
	rawBalance: bigint,
	acceptedSources: string[] | undefined,
	extra: Record<string, unknown> = {},
): void {
	const id = `${POOL}-${BASE}-SELL-${USDC_BASE}-${solver}`
	records.set(`PoolBidder:${id}`, {
		id,
		poolId: POOL,
		providerId: solver,
		chain: BASE,
		direction: "SELL",
		outputToken: USDC_BASE,
		liquidity: rawBalance * SCALE,
		acceptedSources,
		lastUpdatedBlock: SNAPSHOT_BLOCK,
		lastUpdatedAt: SNAPSHOT_TIME,
		...extra,
	})
}

/** What an EVM node published: the solver's raw balance of the token on the chain, as of a time. */
function plantReading(
	solver: string,
	rawBalance: bigint,
	observedAt = FILL_TIME,
	chain = BASE,
	token = USDC_BASE,
): void {
	const id = `${chain}-${token}-${solver}`
	records.set(`SolverInventoryReading:${id}`, {
		id,
		chain,
		provider: solver,
		tokenAddress: token,
		balance: rawBalance,
		blockNumber: FILL_BLOCK,
		observedAt,
		trigger: "FILL",
	})
}

const read = (entity: string, id: string) => records.get(`${entity}:${id}`)
const bidder = (solver: string) => read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${solver}`)
const balanceRow = (solver: string, block = FOLD_BLOCK) =>
	read("LiquidityProviderBalanceV2", `${BASE}-${USDC_BASE}-${block}-${solver}`)

// The service memoizes which readings it has folded, so each test gets a fresh module — every
// `fold()` below is then a pure pass over the store, which is the property under test. The memo
// itself is exercised once, explicitly, at the end.
let foldInventoryReadings: typeof Fold
const freshModule = () => {
	jest.isolateModules(() => {
		foldInventoryReadings = require("@/services/liquidityPool.service").foldInventoryReadings
	})
}
const fold = (blockNumber = FOLD_BLOCK) => {
	freshModule()
	return foldInventoryReadings({ blockNumber })
}

beforeEach(() => {
	records.clear()
	;(global as any).store.get.mockClear()
	;(global as any).store.set.mockClear()
	;(global as any).store.remove.mockClear()
	;(global as any).store.getByFields.mockClear()
	plantPool()
})

describe("foldInventoryReadings", () => {
	it("republishes bidder, chain, route and pool depth from a reading", async () => {
		// Solver A filled 400 USDC worth of an order; solver B is untouched.
		plantReading(SOLVER_A, usdc(600n))

		await fold()

		expect(bidder(SOLVER_A).liquidity).toBe(usdc(600n) * SCALE)
		expect(bidder(SOLVER_A).refreshedAt).toEqual(FILL_TIME)
		expect(bidder(SOLVER_B).liquidity).toBe(usdc(500n) * SCALE)
		expect(bidder(SOLVER_B).refreshedAt).toBeUndefined()

		const chainRow = read("PoolChainLiquidity", `${POOL}-${BASE}-SELL`)
		expect(chainRow.depth).toBe(usdc(1100n) * SCALE)
		expect(chainRow.bidCount).toBe(2)
		// Solver B declares no accepted sources, so its capacity stays in the unrestricted slice.
		expect(chainRow.unrestrictedDepth).toBe(usdc(500n) * SCALE)
		expect(chainRow.unrestrictedBidCount).toBe(1)

		expect(read("PoolRoute", `${POOL}-${BASE}-SELL-${ETHEREUM}`).depth).toBe(usdc(600n) * SCALE)

		const pool = read("LiquidityPool", POOL)
		expect(pool.sellDepth).toBe(usdc(1100n) * SCALE)
		expect(pool.sellBidCount).toBe(2)
	})

	// The rate is the snapshot's, and so is the block and time that date it: nothing here observes
	// a quote, so nothing here may claim to have priced the pool.
	it("leaves the rate and the snapshot's provenance untouched", async () => {
		plantReading(SOLVER_A, usdc(600n))

		await fold()

		for (const [entity, id] of [
			["LiquidityPool", POOL],
			["PoolChainLiquidity", `${POOL}-${BASE}-SELL`],
			["PoolRoute", `${POOL}-${BASE}-SELL-${ETHEREUM}`],
			["PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_A}`],
		] as const) {
			expect(read(entity, id).lastUpdatedBlock).toBe(SNAPSHOT_BLOCK)
			expect(read(entity, id).lastUpdatedAt).toEqual(SNAPSHOT_TIME)
		}
		expect(read("LiquidityPool", POOL).sellRate).toBe(RATE)
		expect(read("PoolChainLiquidity", `${POOL}-${BASE}-SELL`).rate).toBe(RATE)
	})

	// Every bidder row is a bidder with capacity — that invariant is what lets consumers count the
	// rows as well as sum them — so a solver that spent its inventory loses its row, and the route
	// it was the only declarer of goes with it.
	it("drops a bidder that reads as holding nothing, and the route it alone backed", async () => {
		plantReading(SOLVER_A, 0n)

		await fold()

		expect(bidder(SOLVER_A)).toBeUndefined()
		expect(read("PoolRoute", `${POOL}-${BASE}-SELL-${ETHEREUM}`)).toBeUndefined()

		const chainRow = read("PoolChainLiquidity", `${POOL}-${BASE}-SELL`)
		expect(chainRow.depth).toBe(usdc(500n) * SCALE)
		expect(chainRow.bidCount).toBe(1)
		expect(read("LiquidityPool", POOL).sellDepth).toBe(usdc(500n) * SCALE)
	})

	// Depth zeroed, rate kept: the same state a snapshot writes for a direction nobody backed this
	// window. Nulling the rate would throw away the last thing known about the price.
	it("zeroes a direction whose bidders have all gone, keeping its last rate", async () => {
		plantReading(SOLVER_A, 0n)
		plantReading(SOLVER_B, 0n)

		await fold()

		const chainRow = read("PoolChainLiquidity", `${POOL}-${BASE}-SELL`)
		expect(chainRow.depth).toBe(0n)
		expect(chainRow.bidCount).toBe(0)
		expect(chainRow.unrestrictedDepth).toBe(0n)
		expect(chainRow.rate).toBe(RATE)
		expect(read("LiquidityPool", POOL).sellDepth).toBe(0n)
		expect(read("LiquidityPool", POOL).sellRate).toBe(RATE)
	})

	// The snapshot read this balance after the event did, so the reading is the older of the two.
	// A replayed event during a resync looks exactly like this.
	it("ignores a reading older than the bidder's snapshot", async () => {
		plantReading(SOLVER_A, 0n, new Date(SNAPSHOT_TIME.getTime() - 60_000))

		await fold()

		expect(bidder(SOLVER_A).liquidity).toBe(usdc(1000n) * SCALE)
		expect(read("LiquidityPool", POOL).sellDepth).toBe(usdc(1500n) * SCALE)
		expect((global as any).store.set).not.toHaveBeenCalled()
	})

	it("ignores a reading it has already folded", async () => {
		plantBidder(SOLVER_A, usdc(600n), [ETHEREUM], { refreshedAt: FILL_TIME })
		plantReading(SOLVER_A, usdc(600n))

		await fold()

		expect((global as any).store.set).not.toHaveBeenCalled()
	})

	it("applies a reading newer than the last one folded", async () => {
		plantBidder(SOLVER_A, usdc(600n), [ETHEREUM], { refreshedAt: FILL_TIME })
		const later = new Date(FILL_TIME.getTime() + 10_000)
		plantReading(SOLVER_A, usdc(200n), later)

		await fold()

		expect(bidder(SOLVER_A).liquidity).toBe(usdc(200n) * SCALE)
		expect(bidder(SOLVER_A).refreshedAt).toEqual(later)
		expect(read("LiquidityPool", POOL).sellDepth).toBe(usdc(700n) * SCALE)
	})

	// The fold runs every block over the whole reading table, so this is the property everything
	// rests on: the second pass over the same rows finds nothing newer than what the rows record.
	it("changes nothing when run again on the same table", async () => {
		plantReading(SOLVER_A, usdc(600n))
		await fold()
		;(global as any).store.set.mockClear()
		;(global as any).store.remove.mockClear()

		await fold(FOLD_BLOCK + 1n)

		expect((global as any).store.set).not.toHaveBeenCalled()
		expect((global as any).store.remove).not.toHaveBeenCalled()
		expect(read("LiquidityPool", POOL).sellDepth).toBe(usdc(1100n) * SCALE)
	})

	it("leaves another chain's rows alone", async () => {
		plantReading(SOLVER_A, 0n, FILL_TIME, ETHEREUM)

		await fold()

		expect(bidder(SOLVER_A).liquidity).toBe(usdc(1000n) * SCALE)
		expect((global as any).store.set).not.toHaveBeenCalled()
	})

	it("ignores a reading for a solver that backs no pool", async () => {
		plantReading(`0x${"ab".repeat(20)}`, usdc(900n))

		await fold()

		expect((global as any).store.set).not.toHaveBeenCalled()
	})

	it("does nothing when nothing has been published", async () => {
		await fold()

		expect((global as any).store.set).not.toHaveBeenCalled()
		expect((global as any).store.getByFields).not.toHaveBeenCalledWith(
			"PoolBidder",
			expect.anything(),
			expect.anything(),
		)
	})

	// The memo is a cost optimisation for the every-block cadence, not a correctness device: once
	// a reading is folded, the next block's pass costs one page of the reading table and no bidder
	// scan at all.
	it("skips the bidder scan on the next block when nothing new was published", async () => {
		plantReading(SOLVER_A, usdc(600n))
		freshModule()
		await foldInventoryReadings({ blockNumber: FOLD_BLOCK })
		;(global as any).store.getByFields.mockClear()

		await foldInventoryReadings({ blockNumber: FOLD_BLOCK + 1n })

		expect((global as any).store.getByFields).toHaveBeenCalledTimes(1)
		expect((global as any).store.getByFields).toHaveBeenCalledWith("SolverInventoryReading", [], expect.anything())
	})
})

// The pool rows are what a taker sizes against, but LiquidityProviderBalanceV2 is the series a
// provider's own liquidity is read from, and it would otherwise keep reporting inventory the pool
// rows already know is spent. The fold runs on a Hyperbridge block, so the series keeps its clock.
describe("foldInventoryReadings balance series", () => {
	it("records the folded balance at the fold's block with the reading's time, in raw units", async () => {
		plantReading(SOLVER_A, usdc(600n))

		await fold()

		expect(balanceRow(SOLVER_A)).toMatchObject({
			providerId: SOLVER_A,
			chain: BASE,
			blockNumber: FOLD_BLOCK,
			tokenAddress: USDC_BASE,
			balance: usdc(600n),
			snapshotTime: FILL_TIME,
		})
		expect(balanceRow(SOLVER_B)).toBeUndefined()
	})

	// The sweep skips tokens a solver does not hold, so a zero is an absent row there and here.
	it("writes no row for a solver that reads as holding nothing", async () => {
		plantReading(SOLVER_A, 0n)

		await fold()

		expect(balanceRow(SOLVER_A)).toBeUndefined()
	})

	// A snapshot closing on this same block may have written the key first, and the larger of two
	// readings is the complete one.
	it("does not lower a reading already recorded at that block", async () => {
		records.set(`LiquidityProviderBalanceV2:${BASE}-${USDC_BASE}-${FOLD_BLOCK}-${SOLVER_A}`, {
			id: `${BASE}-${USDC_BASE}-${FOLD_BLOCK}-${SOLVER_A}`,
			providerId: SOLVER_A,
			chain: BASE,
			blockNumber: FOLD_BLOCK,
			tokenAddress: USDC_BASE,
			balance: usdc(900n),
			snapshotTime: SNAPSHOT_TIME,
		})
		plantReading(SOLVER_A, usdc(600n))

		await fold()

		expect(balanceRow(SOLVER_A).balance).toBe(usdc(900n))
		// The pool rows still take the reading — they are not keyed by that block.
		expect(bidder(SOLVER_A).liquidity).toBe(usdc(600n) * SCALE)
	})

	it("records nothing for a reading that applied to no row", async () => {
		plantReading(SOLVER_A, usdc(600n), new Date(SNAPSHOT_TIME.getTime() - 60_000))

		await fold()

		expect(balanceRow(SOLVER_A)).toBeUndefined()
	})
})
