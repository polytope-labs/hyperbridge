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

// The declaration lookup asks for the newest phantom order and the newest bid, so the stand-in has
// to honour ordering or the test would pass on a store that does not.
const sort = (rows: any[], options: { orderBy?: string; orderDirection?: string }) => {
	if (!options.orderBy) return rows
	const direction = options.orderDirection === "DESC" ? -1 : 1
	return [...rows].sort((a, b) => (a[options.orderBy!] < b[options.orderBy!] ? -direction : direction))
}

import { poolsForFill, refreshPoolLiquidity, refreshProviderLiquidity } from "@/services/liquidityPool.service"
import type { V4PositionState } from "@hyperbridge/sdk/intents-helpers"

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

// Hyperbridge's head when the fill lands: the block a refresh's balance rows are stamped with.
const HEAD_BLOCK = SNAPSHOT_BLOCK + 12n
const headBlock = jest.fn(async () => HEAD_BLOCK as bigint | null)

/** Declared V4 positions the reader will resolve, keyed `chain|tokenId`. */
let positions: Map<string, V4PositionState | null>
const readPosition = jest.fn(async (chain: string, tokenId: bigint) => positions.get(`${chain}|${tokenId}`) ?? null)

const context = (observedAt = FILL_TIME) => ({
	observedAt,
	evmRpcUrls: RPC_URLS,
	getBalance,
	readPosition,
	headBlock,
})

const refresh = (observedAt = FILL_TIME) => refreshPoolLiquidity({ poolIds: [POOL], ...context(observedAt) })

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
		positions = new Map()
		unreadable = new Set()
		getBalance.mockClear()
		readPosition.mockClear()
		headBlock.mockClear()
		headBlock.mockResolvedValue(HEAD_BLOCK)
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

		await refreshPoolLiquidity({ poolIds: [POOL], ...context() })

		expect(getBalance).not.toHaveBeenCalled()
	})
})

// The pool rows are what a taker sizes against, but LiquidityProviderBalanceV2 is the series a
// provider's own liquidity is read from, and it would otherwise keep reporting inventory the pool
// rows already know is spent. A fill has no Hyperbridge block of its own, so it borrows the head.
describe("refreshPoolLiquidity balance series", () => {
	beforeEach(() => {
		records.clear()
		balances = new Map()
		positions = new Map()
		unreadable = new Set()
		getBalance.mockClear()
		readPosition.mockClear()
		headBlock.mockClear()
		headBlock.mockResolvedValue(HEAD_BLOCK)
		plantPool()
	})

	const balanceRow = (solver: string, block = HEAD_BLOCK) =>
		read("LiquidityProviderBalanceV2", `${BASE}-${USDC_BASE}-${block}-${solver}`)

	it("records the re-read balance at Hyperbridge's head block, in raw token units", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(600n))

		await refresh()

		// Raw units, not the 1e18-normalized depth the pool rows carry.
		expect(balanceRow(SOLVER_A).balance).toBe(usdc(600n))
		expect(balanceRow(SOLVER_A).chain).toBe(BASE)
		expect(balanceRow(SOLVER_A).tokenAddress).toBe(USDC_BASE)
		expect(balanceRow(SOLVER_A).providerId).toBe(SOLVER_A)
		expect(balanceRow(SOLVER_A).snapshotTime).toEqual(FILL_TIME)
		expect(balanceRow(SOLVER_B).balance).toBe(usdc(500n))
	})

	// The sweep skips tokens a solver does not hold, so a zero is an absent row there and here.
	it("writes no row for a solver that now holds nothing", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, 0n)

		await refresh()

		expect(balanceRow(SOLVER_A)).toBeUndefined()
		expect(balanceRow(SOLVER_B)).toBeDefined()
	})

	// The sweep's own convention for a key two readings land on: the larger one is the complete
	// one, because the smaller is the one that could not see a solver's Uniswap V4 positions — and
	// a refresh never can.
	it("does not lower a reading already recorded at that block", async () => {
		records.set(`LiquidityProviderBalanceV2:${BASE}-${USDC_BASE}-${HEAD_BLOCK}-${SOLVER_A}`, {
			id: `${BASE}-${USDC_BASE}-${HEAD_BLOCK}-${SOLVER_A}`,
			providerId: SOLVER_A,
			chain: BASE,
			blockNumber: HEAD_BLOCK,
			tokenAddress: USDC_BASE,
			balance: usdc(900n),
			snapshotTime: SNAPSHOT_TIME,
		})
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(600n))

		await refresh()

		expect(balanceRow(SOLVER_A).balance).toBe(usdc(900n))
		// The pool rows still take the fresh reading — they are not keyed by that block.
		expect(read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_A}`).liquidity).toBe(usdc(600n) * SCALE)
	})

	// The series is secondary: an unreachable Hyperbridge node costs a data point, not the refresh.
	it("still refreshes the pool when Hyperbridge's head cannot be read", async () => {
		headBlock.mockResolvedValue(null)
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(600n))

		await refresh()

		expect(balanceRow(SOLVER_A)).toBeUndefined()
		expect(read("LiquidityPool", POOL).sellDepth).toBe(usdc(1100n) * SCALE)
	})

	it("records nothing for a chain whose balances could not be read", async () => {
		unreadable.add(BASE)

		await refresh()

		expect(balanceRow(SOLVER_A)).toBeUndefined()
		expect(balanceRow(SOLVER_B)).toBeUndefined()
	})
})

// The blind spot this closes: a bid is the only place a Uniswap V4 position is named, so before the
// tokenIds were persisted a refresh could not see that inventory — and simplex funds fills out of
// those positions, draining them inside the fill transaction while wallet and vault barely move.
describe("refreshPoolLiquidity with declared Uniswap V4 positions", () => {
	// A position holding 300 USDC-equivalent, priced entirely on the USDC side of its range.
	const positionState = (owner: string, amount: bigint): V4PositionState => ({
		owner,
		liquidity: amount,
		sqrtPriceX96: 1n << 96n,
		info: {
			currency0: USDC_BASE as `0x${string}`,
			currency1: CNGN_BASE as `0x${string}`,
			poolKeyEncoded: "0x" as `0x${string}`,
			tickLower: -60,
			tickUpper: 60,
		},
	})

	// What the phantom snapshot recorded from the solver's last bid: one row, the live declaration.
	const plantDeclaration = (solver: string, tokenIds: bigint[], chain = BASE) =>
		records.set(`SolverV4Positions:${solver}`, {
			id: solver,
			providerId: solver,
			chain,
			tokenIds,
			lastDeclaredBlock: SNAPSHOT_BLOCK,
			lastDeclaredAt: SNAPSHOT_TIME,
		})

	beforeEach(() => {
		records.clear()
		balances = new Map()
		positions = new Map()
		unreadable = new Set()
		getBalance.mockClear()
		readPosition.mockClear()
		headBlock.mockClear()
		headBlock.mockResolvedValue(HEAD_BLOCK)
		plantPool()
	})

	it("counts a declared position's withdrawable amount on top of the wallet and vault balance", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(600n))
		plantDeclaration(SOLVER_A, [77n])
		positions.set(`${BASE}|77`, positionState(SOLVER_A, usdc(400n)))

		await refresh()

		const bidder = read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_A}`)
		// The exact position amount is Uniswap's arithmetic, not ours; what matters is that it is
		// counted, so the row exceeds the wallet balance alone.
		expect(bidder.liquidity).toBeGreaterThan(usdc(600n) * SCALE)
		expect(read("LiquidityPool", POOL).sellDepth).toBe(bidder.liquidity + usdc(500n) * SCALE)
	})

	// The whole point of re-reading rather than carrying the last window's value forward: a fill
	// funded from the position drains it, and the refresh must see that.
	it("keeps only the wallet balance once the position has been drained", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(600n))
		plantDeclaration(SOLVER_A, [77n])
		positions.set(`${BASE}|77`, positionState(SOLVER_A, 0n))

		await refresh()

		expect(read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_A}`).liquidity).toBe(usdc(600n) * SCALE)
	})

	// Burned or sold, the position is no longer this solver's inventory. Nothing is stored to clean
	// up — the declaration lives in the bid — so it simply stops counting.
	it("counts nothing for a position that no longer exists", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(600n))
		plantDeclaration(SOLVER_A, [77n])
		positions.set(`${BASE}|77`, null)

		await refresh()

		expect(read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_A}`).liquidity).toBe(usdc(600n) * SCALE)
	})

	it("counts nothing for a position the solver no longer owns", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(600n))
		plantDeclaration(SOLVER_A, [77n])
		positions.set(`${BASE}|77`, positionState(SOLVER_B, usdc(400n)))

		await refresh()

		expect(read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_A}`).liquidity).toBe(usdc(600n) * SCALE)
	})

	// The row is the live declaration, so a solver that stopped declaring reads as declaring nothing
	// without anything having to be cleaned up.
	it("counts nothing when the solver's row declares no position", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(600n))
		plantDeclaration(SOLVER_A, [])

		await refresh()

		expect(readPosition).not.toHaveBeenCalled()
		expect(read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_A}`).liquidity).toBe(usdc(600n) * SCALE)
	})

	// A bid is per chain, so a declaration made on one chain is invisible to every other — otherwise
	// a refresh would credit a solver on Base with liquidity that only exists elsewhere.
	it("ignores a declaration recorded on another chain", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(600n))
		plantDeclaration(SOLVER_A, [77n], ETHEREUM)
		positions.set(`${BASE}|77`, positionState(SOLVER_A, usdc(400n)))

		await refresh()

		expect(readPosition).not.toHaveBeenCalled()
		expect(read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_A}`).liquidity).toBe(usdc(600n) * SCALE)
	})

	// One read per solver, whatever the shape of the pool rows: the declaration is a single keyed
	// row, which is the point of keying it by the LP.
	it("reads one declaration per solver, not one per bidder row", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(600n))
		plantDeclaration(SOLVER_A, [77n])
		positions.set(`${BASE}|77`, positionState(SOLVER_A, usdc(400n)))

		await refresh()

		expect(readPosition).toHaveBeenCalledTimes(1)
	})

	// A position that cannot be read is not a position worth zero, exactly as a balance that cannot
	// be read is not a balance of zero.
	it("leaves the chain as indexed when a position cannot be read", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(600n))
		plantDeclaration(SOLVER_A, [77n])
		readPosition.mockRejectedValueOnce(new Error("RPC unreachable"))

		await refresh()

		expect(read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_A}`).liquidity).toBe(usdc(1000n) * SCALE)
	})
})

// Escrow releases and vault ledger events name a solver and a token, never a pool. They reach the
// same refresh through the provider-scoped entry point.
describe("refreshProviderLiquidity", () => {
	beforeEach(() => {
		records.clear()
		balances = new Map()
		positions = new Map()
		unreadable = new Set()
		getBalance.mockClear()
		readPosition.mockClear()
		headBlock.mockClear()
		headBlock.mockResolvedValue(HEAD_BLOCK)
		plantPool()
	})

	// The depth must be re-summed from every bidder on the pool-chain, not just the one re-read, or
	// refreshing one solver would erase the others' contribution.
	it("re-reads one solver and keeps the other bidders' depth intact", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(1400n))

		await refreshProviderLiquidity({ chain: BASE, provider: SOLVER_A, tokens: [USDC_BASE], ...context() })

		expect(read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_A}`).liquidity).toBe(usdc(1400n) * SCALE)
		expect(read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_B}`).liquidity).toBe(usdc(500n) * SCALE)
		expect(read("PoolChainLiquidity", `${POOL}-${BASE}-SELL`).depth).toBe(usdc(1900n) * SCALE)
		expect(read("LiquidityPool", POOL).sellDepth).toBe(usdc(1900n) * SCALE)
		// Only the named solver's balance was read; the others cost nothing.
		expect(getBalance).toHaveBeenCalledTimes(1)
	})

	// An escrow release pays the solver back, so this direction is as important as a fill's.
	it("raises depth when the solver's inventory has grown", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_B}`, usdc(900n))

		await refreshProviderLiquidity({ chain: BASE, provider: SOLVER_B, tokens: [USDC_BASE], ...context() })

		expect(read("PoolChainLiquidity", `${POOL}-${BASE}-SELL`).depth).toBe(usdc(1900n) * SCALE)
		expect(read("PoolChainLiquidity", `${POOL}-${BASE}-SELL`).unrestrictedDepth).toBe(usdc(900n) * SCALE)
	})

	it("touches nothing for a token the solver backs no pool in", async () => {
		await refreshProviderLiquidity({ chain: BASE, provider: SOLVER_A, tokens: [CNGN_BASE], ...context() })

		expect(getBalance).not.toHaveBeenCalled()
		expect(read("PoolChainLiquidity", `${POOL}-${BASE}-SELL`).depth).toBe(usdc(1500n) * SCALE)
	})

	it("touches nothing for an address that backs no pool", async () => {
		await refreshProviderLiquidity({
			chain: BASE,
			provider: `0x${"ab".repeat(20)}`,
			tokens: [USDC_BASE],
			...context(),
		})

		expect(getBalance).not.toHaveBeenCalled()
		expect(read("PoolChainLiquidity", `${POOL}-${BASE}-SELL`).depth).toBe(usdc(1500n) * SCALE)
	})
})
