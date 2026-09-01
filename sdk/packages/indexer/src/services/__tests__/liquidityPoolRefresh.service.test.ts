// The fill-driven half of the pool entities: which pools a fill traded through, and what
// re-reading its LPs' balances does to the rows a taker reads. Exercised against the real generated
// token registry, like the snapshot half — the registry's decimals are what turn a raw balance into
// published depth, so what matters is that the shipped data resolves.

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
			rowsOf(entity).filter((row) => filter.every(([field, , value]) => row[field] === value)),
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

import { poolsForFill, refreshPoolLiquidity } from "@/services/liquidityPool.service"

const BASE = "EVM-8453"
const ETHEREUM = "EVM-1"
const CNGN_BASE = "0x46c85152bfe9f96829aa94755d9f915f9b10ef5f"
const USDC_BASE = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
const USDC_ETHEREUM = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
const POOL = "cNGN-USDC"
const SOLVER_A = "0x1111111111111111111111111111111111111111"
const SOLVER_B = "0x2222222222222222222222222222222222222222"

// Both pool tokens are 6-decimal, so a raw balance is published at 1e12 times its value.
const SCALE = 10n ** 12n
const usdc = (whole: bigint) => whole * 10n ** 6n

// The pool's rows were written by a Hyperbridge snapshot at this block and time; the fill that
// triggers the refresh happens after it. Both are asserted to survive the refresh untouched — they
// are the provenance of the RATE, which no balance read re-derives.
const SNAPSHOT_BLOCK = 4_200_000n
const SNAPSHOT_TIME = new Date("2026-08-01T12:00:00Z")
const FILL_TIME = new Date("2026-08-01T12:00:30Z")
const RATE = 715_000_000_000_000_000n

const RPC_URLS = { [BASE]: "https://base.example", [ETHEREUM]: "https://ethereum.example" }

/** Raw (unscaled) balances the refresh will read, keyed `chain|token|solver`. */
let balances: Map<string, bigint>
/** Chains whose reads must throw, standing in for an unreachable RPC. */
let unreadable: Set<string>

const getBalance = jest.fn(async (_url: string, chain: string, token: string, solver: string) => {
	if (unreadable.has(chain)) throw new Error(`RPC unreachable for ${chain}`)
	return balances.get(`${chain}|${token}|${solver}`) ?? 0n
})

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

function plantBidder(solver: string, rawBalance: bigint, acceptedSources: string[] | undefined): void {
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
	})
	balances.set(`${BASE}|${USDC_BASE}|${solver}`, rawBalance)
}

const refresh = (filledAt = FILL_TIME) =>
	refreshPoolLiquidity({ poolIds: [POOL], filledAt, evmRpcUrls: RPC_URLS, getBalance })

const read = (entity: string, id: string) => records.get(`${entity}:${id}`)

describe("poolsForFill", () => {
	// The fill's two sides live on two chains — inputs escrowed on the source, outputs delivered on
	// the destination — and the same symbol carries different addresses and decimals on each, so the
	// pair can only be resolved one chain at a time.
	it("pairs the source chain's input symbol with the destination chain's output symbol", () => {
		expect(
			poolsForFill({
				sourceChain: ETHEREUM,
				inputTokens: [USDC_ETHEREUM],
				destChain: BASE,
				outputTokens: [CNGN_BASE],
			}),
		).toEqual([POOL])
	})

	// Event fields arrive as bytes32, and the registry is keyed by 20-byte addresses.
	it("resolves tokens given as left-padded bytes32", () => {
		expect(
			poolsForFill({
				sourceChain: ETHEREUM,
				inputTokens: [`0x000000000000000000000000${USDC_ETHEREUM.slice(2)}`],
				destChain: BASE,
				outputTokens: [`0x000000000000000000000000${CNGN_BASE.slice(2)}`],
			}),
		).toEqual([POOL])
	})

	// Most orders are in assets no pool tracks. Resolving them to nothing is what keeps the refresh
	// off the critical path of every fill the indexer sees.
	it("resolves nothing when either side is not registry-tracked", () => {
		expect(
			poolsForFill({
				sourceChain: ETHEREUM,
				inputTokens: ["0x1f9840a85d5af5bf1d1762f925bdaddc4201f984"],
				destChain: BASE,
				outputTokens: [CNGN_BASE],
			}),
		).toEqual([])
	})

	// A token on the wrong chain is not the same token: the registry is per chain, so the Base USDC
	// address means nothing on Ethereum and must not resolve to Ethereum's USDC.
	it("does not resolve a token address against another chain's registry", () => {
		expect(
			poolsForFill({
				sourceChain: ETHEREUM,
				inputTokens: [USDC_BASE],
				destChain: BASE,
				outputTokens: [CNGN_BASE],
			}),
		).toEqual([])
	})
})

describe("refreshPoolLiquidity", () => {
	beforeEach(() => {
		records.clear()
		balances = new Map()
		unreadable = new Set()
		getBalance.mockClear()
		plantPool()
	})

	it("republishes bidder, chain, route and pool depth from the re-read balances", async () => {
		// Solver A filled 400 USDC worth of the order; solver B is untouched.
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(600n))

		await refresh()

		expect(read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_A}`).liquidity).toBe(usdc(600n) * SCALE)
		expect(read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_B}`).liquidity).toBe(usdc(500n) * SCALE)

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

	// The rate is the snapshot's, and so is the block and time that date it. A fill carries an EVM
	// block number of another chain entirely, which would be meaningless against the Hyperbridge
	// blocks these rows age by.
	it("leaves the rate and the snapshot's provenance untouched", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(600n))

		await refresh()

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
	it("drops a bidder that now holds nothing, and the route it alone backed", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, 0n)

		await refresh()

		expect(read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_A}`)).toBeUndefined()
		expect(read("PoolRoute", `${POOL}-${BASE}-SELL-${ETHEREUM}`)).toBeUndefined()

		const chainRow = read("PoolChainLiquidity", `${POOL}-${BASE}-SELL`)
		expect(chainRow.depth).toBe(usdc(500n) * SCALE)
		expect(chainRow.bidCount).toBe(1)
		expect(read("LiquidityPool", POOL).sellDepth).toBe(usdc(500n) * SCALE)
	})

	// Depth zeroed, rate kept: the same state a snapshot writes for a direction nobody backed this
	// window. Nulling the rate would throw away the last thing known about the price.
	it("zeroes a direction whose bidders have all gone, keeping its last rate", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, 0n)
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_B}`, 0n)

		await refresh()

		const chainRow = read("PoolChainLiquidity", `${POOL}-${BASE}-SELL`)
		expect(chainRow.depth).toBe(0n)
		expect(chainRow.bidCount).toBe(0)
		expect(chainRow.unrestrictedDepth).toBe(0n)
		expect(chainRow.rate).toBe(RATE)
		expect(read("LiquidityPool", POOL).sellDepth).toBe(0n)
		expect(read("LiquidityPool", POOL).sellRate).toBe(RATE)
	})

	// A failed read is indistinguishable from a zero balance, so a chain that cannot be read in full
	// is not written at all — publishing the half that answered would report the rest as departed.
	it("leaves a chain exactly as indexed when a balance cannot be read", async () => {
		unreadable.add(BASE)

		await refresh()

		expect(read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_A}`).liquidity).toBe(usdc(1000n) * SCALE)
		expect(read("PoolChainLiquidity", `${POOL}-${BASE}-SELL`).depth).toBe(usdc(1500n) * SCALE)
		expect(read("PoolRoute", `${POOL}-${BASE}-SELL-${ETHEREUM}`).depth).toBe(usdc(1000n) * SCALE)
		expect(read("LiquidityPool", POOL).sellDepth).toBe(usdc(1500n) * SCALE)
	})

	// The guard that keeps a resync free: a replayed fill is older than the pool's current sample,
	// and re-reading balances at the chain head would replace fresher data with a partial view of it.
	it("skips a pool already sampled after the fill, without reading a balance", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, 0n)

		await refresh(new Date(SNAPSHOT_TIME.getTime() - 60_000))

		expect(getBalance).not.toHaveBeenCalled()
		expect(read("LiquidityPool", POOL).sellDepth).toBe(usdc(1500n) * SCALE)
	})

	it("does nothing for a pool the indexer has never sampled", async () => {
		records.delete(`LiquidityPool:${POOL}`)

		await refreshPoolLiquidity({ poolIds: [POOL], filledAt: FILL_TIME, evmRpcUrls: RPC_URLS, getBalance })

		expect(getBalance).not.toHaveBeenCalled()
	})
})
