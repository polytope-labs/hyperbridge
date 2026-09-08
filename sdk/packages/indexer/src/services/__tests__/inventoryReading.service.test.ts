// The EVM-side half of the fill-driven pool refresh: what an event on one chain publishes about
// its solvers' inventory, and what it must NOT touch. The pool rows are the Hyperbridge node's to
// write, so the assertion running through every test here is that none of them change.
// Exercised against the real generated token registry, like the snapshot half — the registry is
// what decides which output tokens are worth reading at all.

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

import { InventoryReadingTrigger } from "@/configs/src/types"
import { publishPoolInventory, publishProviderInventory } from "@/services/inventoryReading.service"
import type { V4PositionState } from "@hyperbridge/sdk/intents-helpers"

const BASE = "EVM-8453"
const ETHEREUM = "EVM-1"
const CNGN_BASE = "0x46c85152bfe9f96829aa94755d9f915f9b10ef5f"
const USDC_BASE = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
const POOL = "cNGN-USDC"
const SOLVER_A = "0x1111111111111111111111111111111111111111"
const SOLVER_B = "0x2222222222222222222222222222222222222222"

// Both pool tokens are 6-decimal, so a raw balance is published at 1e12 times its value.
const SCALE = 10n ** 12n
const usdc = (whole: bigint) => whole * 10n ** 6n

const SNAPSHOT_BLOCK = 4_200_000n
const SNAPSHOT_TIME = new Date("2026-08-01T12:00:00Z")
const FILL_TIME = new Date("2026-08-01T12:00:30Z")
const FILL_BLOCK = 30_000_000n
const RATE = 715_000_000_000_000_000n

const RPC_URL = "https://base.example"

/** Raw (unscaled) balances the publisher will read, keyed `chain|token|solver`. */
let balances: Map<string, bigint>
/** Chains whose reads must throw, standing in for an unreachable RPC. */
let unreadable: Set<string>

const getBalance = jest.fn(async (_url: string, chain: string, token: string, solver: string) => {
	if (unreadable.has(chain)) throw new Error(`RPC unreachable for ${chain}`)
	return balances.get(`${chain}|${token}|${solver}`) ?? 0n
})

/** Declared V4 positions the reader will resolve, keyed `chain|tokenId`. */
let positions: Map<string, V4PositionState | null>
const readPosition = jest.fn(async (chain: string, tokenId: bigint) => positions.get(`${chain}|${tokenId}`) ?? null)

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
	balances.set(`${BASE}|${USDC_BASE}|${solver}`, rawBalance)
}

const plantDeclaration = (solver: string, tokenIds: bigint[], chain = BASE) =>
	records.set(`SolverV4Positions:${solver}`, {
		id: solver,
		providerId: solver,
		chain,
		tokenIds,
		lastDeclaredBlock: SNAPSHOT_BLOCK,
		lastDeclaredAt: SNAPSHOT_TIME,
	})

const context = (observedAt = FILL_TIME, overrides: { evmRpcUrl?: string } = {}) => ({
	chain: BASE,
	evmRpcUrl: RPC_URL as string | undefined,
	getBalance,
	readPosition,
	blockNumber: FILL_BLOCK,
	observedAt,
	trigger: InventoryReadingTrigger.FILL,
	...overrides,
})

const publish = (observedAt = FILL_TIME) => publishPoolInventory({ poolIds: [POOL], ...context(observedAt) })

const read = (entity: string, id: string) => records.get(`${entity}:${id}`)
const reading = (solver: string, token = USDC_BASE) => read("SolverInventoryReading", `${BASE}-${token}-${solver}`)

/** The rows only the Hyperbridge node may write, as planted: the publisher must leave every one. */
function expectPoolFamilyUntouched(): void {
	expect(read("LiquidityPool", POOL).sellDepth).toBe(usdc(1500n) * SCALE)
	expect(read("PoolChainLiquidity", `${POOL}-${BASE}-SELL`).depth).toBe(usdc(1500n) * SCALE)
	expect(read("PoolRoute", `${POOL}-${BASE}-SELL-${ETHEREUM}`).depth).toBe(usdc(1000n) * SCALE)
	expect(read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_A}`).liquidity).toBe(usdc(1000n) * SCALE)
	expect(read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_B}`).liquidity).toBe(usdc(500n) * SCALE)
	for (const entity of [
		"LiquidityPool",
		"PoolChainLiquidity",
		"PoolBidder",
		"PoolRoute",
		"LiquidityProviderBalanceV2",
	]) {
		expect((global as any).store.set).not.toHaveBeenCalledWith(entity, expect.anything(), expect.anything())
		expect((global as any).store.remove).not.toHaveBeenCalledWith(entity, expect.anything())
	}
}

beforeEach(() => {
	records.clear()
	balances = new Map()
	positions = new Map()
	unreadable = new Set()
	getBalance.mockClear()
	readPosition.mockClear()
	;(global as any).store.get.mockClear()
	;(global as any).store.set.mockClear()
	;(global as any).store.remove.mockClear()
	plantPool()
})

describe("publishPoolInventory", () => {
	it("publishes one reading per solver and output token, in raw units, and touches no pool row", async () => {
		// Solver A filled 400 USDC worth of the order; solver B is untouched.
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(600n))

		await publish()

		expect(reading(SOLVER_A)).toMatchObject({
			chain: BASE,
			provider: SOLVER_A,
			tokenAddress: USDC_BASE,
			balance: usdc(600n),
			blockNumber: FILL_BLOCK,
			observedAt: FILL_TIME,
			trigger: InventoryReadingTrigger.FILL,
		})
		expect(reading(SOLVER_B).balance).toBe(usdc(500n))
		expectPoolFamilyUntouched()
	})

	// Zero is the reading that tells the fold to drop the bidder, so unlike the balance series it
	// must be a row.
	it("publishes a zero reading for a solver that now holds nothing", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, 0n)

		await publish()

		expect(reading(SOLVER_A).balance).toBe(0n)
		expectPoolFamilyUntouched()
	})

	// A reading is a (solver, token) fact; several bidder rows sharing it share one read.
	it("reads each solver and token once, whatever the shape of the pool rows", async () => {
		const other = `cNGN-USDC-other`
		records.set(`PoolBidder:${other}-${BASE}-BUY-${USDC_BASE}-${SOLVER_A}`, {
			...read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_A}`),
			id: `${other}-${BASE}-BUY-${USDC_BASE}-${SOLVER_A}`,
			poolId: other,
			direction: "BUY",
		})

		await publishPoolInventory({ poolIds: [POOL, other], ...context() })

		expect(getBalance).toHaveBeenCalledTimes(2)
	})

	// A failed read is indistinguishable from a zero balance, and a zero reading drops the bidder,
	// so a partial publication would report the unread solvers as departed.
	it("publishes nothing when a balance cannot be read", async () => {
		unreadable.add(BASE)

		await publish()

		expect(reading(SOLVER_A)).toBeUndefined()
		expect(reading(SOLVER_B)).toBeUndefined()
	})

	it("publishes nothing without an RPC for the chain", async () => {
		await publishPoolInventory({ poolIds: [POOL], ...context(FILL_TIME, { evmRpcUrl: undefined }) })

		expect(getBalance).not.toHaveBeenCalled()
		expect(reading(SOLVER_A)).toBeUndefined()
	})

	// The guard that keeps a resync free: a replayed event is older than the rows' current sample,
	// and the reading it would publish is one the fold would ignore anyway.
	it("skips a solver whose rows were all sampled after the event, without reading a balance", async () => {
		await publish(new Date(SNAPSHOT_TIME.getTime() - 60_000))

		expect(getBalance).not.toHaveBeenCalled()
		expect(reading(SOLVER_A)).toBeUndefined()
	})

	it("skips a solver whose rows were already refreshed after the event", async () => {
		plantBidder(SOLVER_A, usdc(1000n), [ETHEREUM], { refreshedAt: new Date(FILL_TIME.getTime() + 1_000) })

		await publish()

		expect(reading(SOLVER_A)).toBeUndefined()
		expect(reading(SOLVER_B)).toBeDefined()
	})

	it("leaves an output token the registry no longer tracks unread", async () => {
		const unknown = `0x${"cd".repeat(20)}`
		records.set(`PoolBidder:${POOL}-${BASE}-SELL-${unknown}-${SOLVER_A}`, {
			...read("PoolBidder", `${POOL}-${BASE}-SELL-${USDC_BASE}-${SOLVER_A}`),
			id: `${POOL}-${BASE}-SELL-${unknown}-${SOLVER_A}`,
			outputToken: unknown,
		})

		await publish()

		expect(reading(SOLVER_A, unknown)).toBeUndefined()
		expect(reading(SOLVER_A)).toBeDefined()
	})

	it("does nothing for a pool nobody backs on this chain", async () => {
		await publishPoolInventory({ poolIds: ["USDC-USDT"], ...context() })

		expect(getBalance).not.toHaveBeenCalled()
	})
})

// The blind spot this closes: a bid is the only place a Uniswap V4 position is named, so before the
// tokenIds were persisted a refresh could not see that inventory — and simplex funds fills out of
// those positions, draining them inside the fill transaction while wallet and vault barely move.
describe("publishPoolInventory with declared Uniswap V4 positions", () => {
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

	beforeEach(() => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(600n))
	})

	it("counts a declared position's withdrawable amount on top of the wallet and vault balance", async () => {
		plantDeclaration(SOLVER_A, [77n])
		positions.set(`${BASE}|77`, positionState(SOLVER_A, usdc(400n)))

		await publish()

		// The exact position amount is Uniswap's arithmetic, not ours; what matters is that it is
		// counted, so the reading exceeds the wallet balance alone.
		expect(reading(SOLVER_A).balance).toBeGreaterThan(usdc(600n))
	})

	// The whole point of re-reading rather than carrying the last window's value forward: a fill
	// funded from the position drains it, and the reading must see that.
	it("keeps only the wallet balance once the position has been drained", async () => {
		plantDeclaration(SOLVER_A, [77n])
		positions.set(`${BASE}|77`, positionState(SOLVER_A, 0n))

		await publish()

		expect(reading(SOLVER_A).balance).toBe(usdc(600n))
	})

	it("counts nothing for a position that no longer exists", async () => {
		plantDeclaration(SOLVER_A, [77n])
		positions.set(`${BASE}|77`, null)

		await publish()

		expect(reading(SOLVER_A).balance).toBe(usdc(600n))
	})

	it("counts nothing for a position the solver no longer owns", async () => {
		plantDeclaration(SOLVER_A, [77n])
		positions.set(`${BASE}|77`, positionState(SOLVER_B, usdc(400n)))

		await publish()

		expect(reading(SOLVER_A).balance).toBe(usdc(600n))
	})

	it("counts nothing when the solver's row declares no position", async () => {
		plantDeclaration(SOLVER_A, [])

		await publish()

		expect(readPosition).not.toHaveBeenCalled()
		expect(reading(SOLVER_A).balance).toBe(usdc(600n))
	})

	// A bid is per chain, so a declaration made on one chain is invisible to every other.
	it("ignores a declaration recorded on another chain", async () => {
		plantDeclaration(SOLVER_A, [77n], ETHEREUM)
		positions.set(`${BASE}|77`, positionState(SOLVER_A, usdc(400n)))

		await publish()

		expect(readPosition).not.toHaveBeenCalled()
		expect(reading(SOLVER_A).balance).toBe(usdc(600n))
	})

	// The declaration is written by the Hyperbridge node. `get` serves this process's own cache,
	// which no other node's write ever invalidates, so the read has to go through the store.
	it("reads the declaration through a store query, not the process cache", async () => {
		plantDeclaration(SOLVER_A, [77n])
		positions.set(`${BASE}|77`, positionState(SOLVER_A, usdc(400n)))

		await publish()

		expect((global as any).store.get).not.toHaveBeenCalledWith("SolverV4Positions", expect.anything())
		expect(readPosition).toHaveBeenCalledTimes(1)
	})

	// A position that cannot be read is not a position worth zero, exactly as a balance that cannot
	// be read is not a balance of zero.
	it("publishes nothing when a position cannot be read", async () => {
		plantDeclaration(SOLVER_A, [77n])
		readPosition.mockRejectedValueOnce(new Error("RPC unreachable"))

		await publish()

		expect(reading(SOLVER_A)).toBeUndefined()
		expect(reading(SOLVER_B)).toBeUndefined()
	})
})

// Escrow releases and vault ledger events name a solver and a token, never a pool. They reach the
// same publication through the provider-scoped entry point.
describe("publishProviderInventory", () => {
	it("reads the named solver only", async () => {
		balances.set(`${BASE}|${USDC_BASE}|${SOLVER_A}`, usdc(1400n))

		await publishProviderInventory({ provider: SOLVER_A, tokens: [USDC_BASE], ...context() })

		expect(reading(SOLVER_A).balance).toBe(usdc(1400n))
		expect(reading(SOLVER_B)).toBeUndefined()
		expect(getBalance).toHaveBeenCalledTimes(1)
		expectPoolFamilyUntouched()
	})

	it("touches nothing for a token the solver backs no pool in", async () => {
		await publishProviderInventory({ provider: SOLVER_A, tokens: [CNGN_BASE], ...context() })

		expect(getBalance).not.toHaveBeenCalled()
	})

	it("touches nothing for an address that backs no pool", async () => {
		await publishProviderInventory({ provider: `0x${"ab".repeat(20)}`, tokens: [USDC_BASE], ...context() })

		expect(getBalance).not.toHaveBeenCalled()
	})
})
