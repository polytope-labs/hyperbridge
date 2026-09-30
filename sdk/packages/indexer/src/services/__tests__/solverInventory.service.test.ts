;(global as any).logger = { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() }

const records = new Map<string, any>()
const rows = (entity: string) =>
	[...records.entries()].filter(([key]) => key.startsWith(`${entity}:`)).map(([, row]) => row)
;(global as any).store = {
	get: jest.fn(async (entity: string, id: string) => records.get(`${entity}:${id}`)),
	set: jest.fn(async (entity: string, id: string, props: any) => records.set(`${entity}:${id}`, { ...props })),
	getByFields: jest.fn(async (entity: string, filters: [string, string, any][], options: any) =>
		rows(entity)
			.filter((row) => filters.every(([field, , value]) => row[field] === value))
			.sort((a, b) => a.id.localeCompare(b.id))
			.slice(options.offset ?? 0, (options.offset ?? 0) + options.limit),
	),
}

// SubQuery's `cache` lives on the main thread; every worker proxies to that one object. Cloning on
// the way in and out is what the host channel does, and it keeps a handler from mutating it in place.
const cached = new Map<string, any>()
const cacheMock = {
	get: jest.fn(async (key: string) => structuredClone(cached.get(key))),
	set: jest.fn(async (key: string, value: any) => void cached.set(key, structuredClone(value))),
}
;(global as any).cache = cacheMock

/** On-chain state the pinned reads return, keyed `contract|holder`. */
const mockOnchain = new Map<string, bigint>()
/** Assets per share, per vault. */
const mockRates = new Map<string, bigint>()
/** The contract of every balanceOf read, those inside a multicall included. */
const mockBalanceReads: string[] = []
/** Holders whose balanceOf reverts. */
const mockReverting = new Set<string>()
jest.mock("@/yield-vault-addresses", () => ({
	YIELD_VAULT_ADDRESSES: {
		"EVM-8453": {
			"0x833589fcd6edb6e08f4c7c32d4f71b54bda02913": ["0xC768c589647798a6EE01A91FdE98EF2ed046DBD6"],
			"0x46c85152bfe9f96829aa94755d9f915f9b10ef5f": [],
		},
	},
}))
jest.mock("@/solver-account-addresses", () => ({
	SOLVER_ACCOUNT_ADDRESSES: { "EVM-8453": ["0xd5535d4DeB17F050e52B6efda2fDe00435f39279"] },
}))

import { ethers } from "ethers"
import Erc4626Abi from "@/configs/abis/Erc4626.abi.json"
import Multicall3Abi from "@/configs/abis/Multicall3.abi.json"
import { SolverDiscoveryTrigger, TrackedSolverStatus } from "@/configs/src/types"
import {
	AFTER_EVERY_LOG,
	applyTokenTransfer,
	applyVaultShareTransfer,
	discoverSolverFromFill,
	HEAD_INTERVAL_SECS,
	indexSolverInventoryBlock,
	parseDelegation,
	RECONCILE_INTERVAL_SECS,
	REVALUE_INTERVAL_SECS,
	type TransferInput,
} from "@/services/solverInventory.service"
import { MULTICALL3_ADDRESS, resetMulticallCache } from "@/utils/multicall"

const CHAIN = "EVM-8453"
const USDC = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
const CNGN = "0x46c85152bfe9f96829aa94755d9f915f9b10ef5f"
const VAULT = "0xc768c589647798a6ee01a91fde98ef2ed046dbd6"
const SOLVER = "0xce319986ca4d5d0893751a628d0db3dc8fc91d62"
const OTHER = "0x13e41cde1d55880cbe031c69f206c2e9bc3c94c2"
const ZERO = "0x0000000000000000000000000000000000000000"
const SOLVER_ACCOUNT = "0xd5535d4deb17f050e52b6efda2fde00435f39279"
const DELEGATED = `0xef0100${SOLVER_ACCOUNT.slice(2)}`
const T0 = 1_750_000_000n

const MULTICALL3 = MULTICALL3_ADDRESS.toLowerCase()
const erc4626 = new ethers.utils.Interface(Erc4626Abi)
const multicall3 = new ethers.utils.Interface(Multicall3Abi)

/** Answers an eth_call from the mocked state, and throws where the chain would revert. */
function answer(to: string, data: string): string {
	const target = to.toLowerCase()
	if (target === MULTICALL3) {
		const [calls] = multicall3.decodeFunctionData("aggregate3", data)
		const results = calls.map((call: any) => {
			try {
				return { success: true, returnData: answer(call.target, call.callData) }
			} catch {
				return { success: false, returnData: "0x" }
			}
		})
		return multicall3.encodeFunctionResult("aggregate3", [results])
	}
	const call = erc4626.parseTransaction({ data })
	if (call.name === "balanceOf") {
		const holder = String(call.args[0]).toLowerCase()
		mockBalanceReads.push(target)
		if (mockReverting.has(holder)) throw new Error("execution reverted")
		return erc4626.encodeFunctionResult("balanceOf", [(mockOnchain.get(`${target}|${holder}`) ?? 0n).toString()])
	}
	const shares = BigInt(call.args[0].toString())
	return erc4626.encodeFunctionResult("convertToAssets", [(shares * (mockRates.get(target) ?? 1n)).toString()])
}

let multicallDeployed = true
/** The code of any account but Multicall3's. */
let solverCode: jest.Mock
const getCode = () => solverCode
/** Every eth_call; a multicall counts once. */
const ethCalls = () => (global as any).api.call as jest.Mock
const inventory = (token = USDC, solver = SOLVER) => records.get(`SolverInventory:${CHAIN}-${token}-${solver}`)
const shares = (solver = SOLVER) => records.get(`SolverVaultShares:${CHAIN}-${VAULT}-${solver}`)
const tracked = (solver = SOLVER) => records.get(`TrackedSolver:${CHAIN}-${solver}`)
const head = () => records.get(`SolverInventoryHead:${CHAIN}`)
const block = (blockNumber: bigint, timestamp: bigint) => indexSolverInventoryBlock(CHAIN, blockNumber, timestamp)

const fill = (blockNumber = 100n, solver = SOLVER) =>
	discoverSolverFromFill({ chain: CHAIN, solver, blockNumber, transactionHash: "0xFILL", timestamp: T0 })

function transfer(
	overrides: Partial<Omit<TransferInput, "timestamp">> & { value?: bigint; token?: string; at?: bigint } = {},
) {
	const { at = T0, ...rest } = overrides
	return applyTokenTransfer({
		chain: CHAIN,
		token: USDC,
		from: OTHER,
		to: SOLVER,
		value: 10n,
		blockNumber: 102n,
		logIndex: 0,
		timestamp: async () => at,
		...rest,
	})
}

function shareTransfer(overrides: Partial<Omit<TransferInput, "timestamp">> & { shares?: bigint; at?: bigint } = {}) {
	const { at = T0, ...rest } = overrides
	return applyVaultShareTransfer({
		chain: CHAIN,
		vault: VAULT,
		from: ZERO,
		to: SOLVER,
		shares: 100n,
		blockNumber: 102n,
		logIndex: 0,
		timestamp: async () => at,
		...rest,
	})
}

/** Discovers SOLVER by a fill at block 100 and runs its genesis read at block 101. */
async function trackSolver(wallet = 1_000n, vaultShares = 0n): Promise<void> {
	mockOnchain.set(`${USDC}|${SOLVER}`, wallet)
	mockOnchain.set(`${VAULT}|${SOLVER}`, vaultShares)
	await fill()
	await block(101n, T0)
}

function queueWatchlist(version: number, ...solvers: string[]): void {
	records.set(`SolverWatchlist:${CHAIN}`, {
		id: CHAIN,
		chain: CHAIN,
		version,
		requestCount: version,
		blockNumber: 1n,
		updatedAt: new Date(0),
	})
	for (const solver of solvers) {
		records.set(`SolverDiscoveryRequest:${CHAIN}-${solver}`, {
			id: `${CHAIN}-${solver}`,
			chain: CHAIN,
			solver,
			version,
			blockNumber: 1n,
			requestedAt: new Date(0),
		})
	}
}

beforeEach(() => {
	records.clear()
	cached.clear()
	mockOnchain.clear()
	mockRates.clear()
	mockRates.set(VAULT, 2n)
	mockBalanceReads.length = 0
	mockReverting.clear()
	multicallDeployed = true
	jest.clearAllMocks()
	resetMulticallCache()
	solverCode = jest.fn(async (_address: string) => DELEGATED)
	;(global as any).api = {
		getCode: jest.fn(async (address: string) =>
			address.toLowerCase() === MULTICALL3 ? (multicallDeployed ? "0x6080604052" : "0x") : solverCode(address),
		),
		call: jest.fn(async ({ to, data }: { to: string; data: string }) => answer(to, data)),
	}
})

describe("discovery", () => {
	test("a fill queues its filler once, without reading anything on chain", async () => {
		await fill(100n, SOLVER.toUpperCase().replace("0X", "0x"))
		await fill(100n)

		expect(rows("TrackedSolver")).toHaveLength(1)
		expect(tracked()).toMatchObject({
			status: TrackedSolverStatus.PENDING,
			discoveredBy: SolverDiscoveryTrigger.FILL,
			discoveredByFill: "0xfill",
			discoveryBlock: 100n,
		})
		expect((global as any).api.getCode).not.toHaveBeenCalled()
		expect(ethCalls()).not.toHaveBeenCalled()
	})

	test("watchlist requests become tracked solvers, reading only the versions not yet consumed", async () => {
		queueWatchlist(1, SOLVER)
		await block(101n, T0)
		expect(tracked()).toMatchObject({
			status: TrackedSolverStatus.TRACKED,
			discoveredBy: SolverDiscoveryTrigger.WATCHLIST,
		})
		expect(records.get(`SolverWatchlistCursor:${CHAIN}`)).toMatchObject({ version: 1 })

		queueWatchlist(2, OTHER)
		jest.mocked(store.getByFields).mockClear()
		await block(102n, T0 + 2n)

		expect(tracked(OTHER)).toMatchObject({ status: TrackedSolverStatus.TRACKED })
		const requestQueries = jest
			.mocked(store.getByFields)
			.mock.calls.filter(([entity]) => entity === "SolverDiscoveryRequest")
			.map(([, filters]) => filters)
		expect(requestQueries).toEqual([
			[
				["chain", "=", CHAIN],
				["version", "=", 2],
			],
		])
	})

	test("a quiet block does not scan the request queue", async () => {
		queueWatchlist(1, SOLVER)
		await block(101n, T0)
		jest.mocked(store.getByFields).mockClear()

		await block(102n, T0 + 2n)

		expect(
			jest.mocked(store.getByFields).mock.calls.filter(([entity]) => entity === "SolverDiscoveryRequest"),
		).toEqual([])
	})
})

describe("genesis", () => {
	test("reads every supported token, vault and the delegation once, pinned to the block", async () => {
		await trackSolver(1_000n, 50n)

		expect(inventory()).toMatchObject({
			wallet: 1_000n,
			vaultShares: 50n,
			vaults: 100n,
			balance: 1_100n,
			genesisBlock: 101n,
			lastReadBlock: 101n,
			blockNumber: 101n,
			lastLogIndex: AFTER_EVERY_LOG,
			discoveredBy: SolverDiscoveryTrigger.FILL,
		})
		// A token the solver holds none of is still a row: zero is an answer.
		expect(inventory(CNGN)).toMatchObject({ wallet: 0n, vaults: 0n, balance: 0n })
		expect(shares()).toMatchObject({ shares: 50n, assets: 100n, tokenAddress: USDC })
		expect(records.get(`SolverDelegation:${CHAIN}-${SOLVER}`)).toMatchObject({
			delegated: true,
			delegate: SOLVER_ACCOUNT,
			blockNumber: 101n,
		})
		expect(tracked()).toMatchObject({ status: TrackedSolverStatus.TRACKED, genesisBlock: 101n })
	})

	test("an undelegated solver is recorded as delegated: false and not checked again on the next block", async () => {
		getCode().mockResolvedValue("0x")
		await trackSolver()
		expect(records.get(`SolverDelegation:${CHAIN}-${SOLVER}`)).toMatchObject({ delegated: false })

		await block(102n, T0 + 2n)

		expect(getCode()).toHaveBeenCalledTimes(1)
	})

	test("a failed read leaves the solver pending, with nothing written, and the next block reads again", async () => {
		getCode().mockRejectedValueOnce(new Error("rpc down"))
		mockOnchain.set(`${USDC}|${SOLVER}`, 1_000n)
		await fill()

		await block(101n, T0)
		expect(tracked()).toMatchObject({ status: TrackedSolverStatus.PENDING })
		expect(inventory()).toBeUndefined()
		expect(records.get(`SolverDelegation:${CHAIN}-${SOLVER}`)).toBeUndefined()

		await block(102n, T0 + 2n)
		expect(tracked()).toMatchObject({ status: TrackedSolverStatus.TRACKED, genesisBlock: 102n })
		expect(inventory()).toMatchObject({ wallet: 1_000n })
	})

	test("the discovering fill's block is counted once: events before the read are in it, events of its block are skipped", async () => {
		await fill(100n)
		// The fill's own block: the solver is still pending, so its transfers are not applied…
		await transfer({ blockNumber: 100n, logIndex: 7, value: 40n })
		expect(inventory()).toBeUndefined()

		// …because the genesis read at 101 already holds them, along with everything else in 101.
		mockOnchain.set(`${USDC}|${SOLVER}`, 1_040n)
		await block(101n, T0 + 2n)
		expect(inventory()).toMatchObject({ wallet: 1_040n })

		await transfer({ blockNumber: 101n, logIndex: 3, value: 25n })
		expect(inventory()).toMatchObject({ wallet: 1_040n })

		await transfer({ blockNumber: 102n, logIndex: 0, value: 25n })
		expect(inventory()).toMatchObject({ wallet: 1_065n, balance: 1_065n, blockNumber: 102n, lastLogIndex: 0 })
	})

	test("a burst of solvers is read in two eth_calls, balances then valuations, plus one getCode each", async () => {
		const solvers = Array.from({ length: 20 }, (_, i) => `0x${(0xa000 + i).toString(16).padStart(40, "0")}`)
		for (const [i, solver] of solvers.entries()) {
			mockOnchain.set(`${USDC}|${solver}`, 1_000n + BigInt(i))
			mockOnchain.set(`${VAULT}|${solver}`, BigInt(i))
		}
		queueWatchlist(1, ...solvers)

		await block(101n, T0)

		expect(ethCalls()).toHaveBeenCalledTimes(2)
		expect(ethCalls().mock.calls.every(([tx]) => tx.to === MULTICALL3_ADDRESS)).toBe(true)
		expect(getCode()).toHaveBeenCalledTimes(solvers.length)
		for (const [i, solver] of solvers.entries()) {
			expect(tracked(solver)).toMatchObject({ status: TrackedSolverStatus.TRACKED })
			expect(inventory(USDC, solver)).toMatchObject({
				wallet: 1_000n + BigInt(i),
				vaultShares: BigInt(i),
				vaults: 2n * BigInt(i),
			})
		}
	})

	test("a read that fails for one solver leaves only that solver pending", async () => {
		mockOnchain.set(`${USDC}|${SOLVER}`, 1_000n)
		mockOnchain.set(`${USDC}|${OTHER}`, 700n)
		mockReverting.add(SOLVER)
		await fill(100n, SOLVER)
		await fill(100n, OTHER)

		await block(101n, T0)
		expect(tracked()).toMatchObject({ status: TrackedSolverStatus.PENDING })
		expect(inventory()).toBeUndefined()
		expect(tracked(OTHER)).toMatchObject({ status: TrackedSolverStatus.TRACKED })
		expect(inventory(USDC, OTHER)).toMatchObject({ wallet: 700n })

		mockReverting.clear()
		await block(102n, T0 + 2n)
		expect(tracked()).toMatchObject({ status: TrackedSolverStatus.TRACKED, genesisBlock: 102n })
		expect(inventory()).toMatchObject({ wallet: 1_000n })
	})

	test("a chain without Multicall3 makes the same reads as individual calls", async () => {
		multicallDeployed = false

		await trackSolver(1_000n, 50n)

		expect(inventory()).toMatchObject({ wallet: 1_000n, vaultShares: 50n, vaults: 100n, balance: 1_100n })
		// USDC, its vault and CNGN's balanceOf, then the vault shares' convertToAssets.
		expect(ethCalls()).toHaveBeenCalledTimes(4)
		expect(ethCalls().mock.calls.some(([tx]) => tx.to === MULTICALL3_ADDRESS)).toBe(false)
	})
})

describe("events", () => {
	test("transfers between untracked addresses cost no store read and no RPC", async () => {
		await trackSolver()
		await transfer({ from: OTHER, to: "0x18f23e630077b1da3ed97c0469d0504a93fad9e2" })
		jest.mocked(store.get).mockClear()
		jest.mocked(store.getByFields).mockClear()
		cacheMock.get.mockClear()
		ethCalls().mockClear()
		;(global as any).api.getCode.mockClear()

		for (let i = 0; i < 2_000; i++) {
			const from = `0x${(i + 1).toString(16).padStart(40, "0")}`
			const to = `0x${(i + 2).toString(16).padStart(40, "0")}`
			await transfer({ from, to, blockNumber: 200n + BigInt(i) })
			await shareTransfer({ from, to, blockNumber: 200n + BigInt(i) })
		}

		// The cached set is the whole cost. A keyed store read in its place would be a Postgres
		// findOne per address, every time, because `cacheModel` never caches a miss.
		expect(cacheMock.get).toHaveBeenCalledTimes(2_000 * 2)
		expect(store.get).not.toHaveBeenCalled()
		expect(store.getByFields).not.toHaveBeenCalled()
		expect(ethCalls()).not.toHaveBeenCalled()
		expect((global as any).api.getCode).not.toHaveBeenCalled()
		expect(inventory()).toMatchObject({ wallet: 1_000n })
	})

	test("a cached set that lost a concurrent seed repairs itself on the next head advance", async () => {
		await trackSolver(1_000n, 0n)
		// The race the rebuild exists for: another worker's seed reached the store, but this worker's
		// cached set was rebuilt just before that write and so never learned the solver.
		cached.set(`solver-inventory:tracked:${CHAIN}`, [])

		// A block inside the head's throttle rebuilds nothing, so the spend is still dropped.
		await block(300n, T0 + BigInt(HEAD_INTERVAL_SECS) - 1n)
		await transfer({ from: SOLVER, to: OTHER, value: 400n, blockNumber: 301n })
		expect(inventory()).toMatchObject({ wallet: 1_000n })

		// The next head advance rebuilds the set from the store, and the solver is seen again.
		await block(302n, T0 + BigInt(HEAD_INTERVAL_SECS))
		await transfer({ from: SOLVER, to: OTHER, value: 400n, blockNumber: 303n })
		expect(inventory()).toMatchObject({ wallet: 600n, balance: 600n })
	})

	test("a solver another worker discovered still has its transfers applied", async () => {
		// SubQuery runs a mapping worker per thread, each thread its own module registry, and the
		// block dispatcher rotates batches across all of them. The tracked set used to be
		// module-level state, so the worker that discovered a solver was the only one whose set
		// learned about it; every other worker dropped that solver's transfers, with no store read
		// behind the miss, until the daily reconciliation. `isolateModulesAsync` gives a second
		// registry over the same store — a second worker — and drives the sequence that lost a fill.
		await jest.isolateModulesAsync(async () => {
			const other =
				require("@/services/solverInventory.service") as typeof import("@/services/solverInventory.service")
			const move = (from: string, to: string, value: bigint, blockNumber: bigint) =>
				other.applyTokenTransfer({
					chain: CHAIN,
					token: USDC,
					from,
					to,
					value,
					blockNumber,
					logIndex: 0,
					timestamp: async () => T0,
				})

			// The second worker handles a batch before the solver is known anywhere.
			await move(OTHER, "0x18f23e630077b1da3ed97c0469d0504a93fad9e2", 10n, 99n)
			// The first worker discovers it, writing the rows every worker shares.
			await trackSolver(1_000n, 0n)
			// A batch carrying the solver's outgoing transfer lands on the second worker.
			await move(SOLVER, OTHER, 400n, 300n)
		})

		expect(inventory()).toMatchObject({ wallet: 600n, balance: 600n, blockNumber: 300n })
	})

	test("incoming and outgoing transfers move wallet and balance, and observedAt never moves backwards", async () => {
		await trackSolver(1_000n, 10n)

		await transfer({ from: OTHER, to: SOLVER, value: 300n, blockNumber: 102n, at: T0 + 10n })
		await transfer({ from: SOLVER, to: OTHER, value: 100n, blockNumber: 103n, at: T0 + 5n })

		expect(inventory()).toMatchObject({ wallet: 1_200n, vaults: 20n, balance: 1_220n, blockNumber: 103n })
		expect(inventory().observedAt).toEqual(new Date(Number(T0 + 10n) * 1000))
	})

	test("replaying an applied log changes nothing", async () => {
		await trackSolver()
		await transfer({ value: 5n, blockNumber: 102n, logIndex: 4 })
		await transfer({ value: 5n, blockNumber: 102n, logIndex: 4 })
		await transfer({ value: 5n, blockNumber: 102n, logIndex: 2 })

		expect(inventory()).toMatchObject({ wallet: 1_005n })
	})

	test("vault mints, transfers and burns move vaultShares, valued in assets", async () => {
		await trackSolver(1_000n, 0n)

		await shareTransfer({ from: ZERO, to: SOLVER, shares: 100n, blockNumber: 102n })
		expect(shares()).toMatchObject({ shares: 100n, assets: 200n })
		expect(inventory()).toMatchObject({ vaultShares: 100n, vaults: 200n, balance: 1_200n })

		await shareTransfer({ from: SOLVER, to: OTHER, shares: 30n, blockNumber: 103n })
		await shareTransfer({ from: SOLVER, to: ZERO, shares: 10n, blockNumber: 104n })
		expect(inventory()).toMatchObject({ vaultShares: 60n, vaults: 120n, balance: 1_120n })
		expect(shares(OTHER)).toBeUndefined()
	})
})

describe("head and refresh", () => {
	test("the head advances at most every 30 seconds of block time", async () => {
		await block(101n, T0)
		expect(head()).toMatchObject({ blockNumber: 101n })

		await block(102n, T0 + 10n)
		expect(head()).toMatchObject({ blockNumber: 101n })

		await block(103n, T0 + 31n)
		expect(head()).toMatchObject({ blockNumber: 103n, observedAt: new Date(Number(T0 + 31n) * 1000) })
	})

	test("vault shares are revalued hourly without re-reading the wallet or the delegation", async () => {
		await trackSolver(1_000n, 50n)
		mockRates.set(VAULT, 3n)
		mockBalanceReads.length = 0

		await block(500n, T0 + BigInt(REVALUE_INTERVAL_SECS) + 5n)

		expect(inventory()).toMatchObject({ wallet: 1_000n, vaultShares: 50n, vaults: 150n, balance: 1_150n })
		expect(inventory().refreshedAt).toEqual(new Date(Number(T0 + BigInt(REVALUE_INTERVAL_SECS) + 5n) * 1000))
		expect(mockBalanceReads).toEqual([])
		expect(getCode()).toHaveBeenCalledTimes(1)
	})

	test("reconciliation re-anchors the running totals to a pinned read and records the drift", async () => {
		await trackSolver(1_000n, 50n)
		// A Transfer the indexer never saw, and shares that moved without one.
		mockOnchain.set(`${USDC}|${SOLVER}`, 900n)
		mockOnchain.set(`${VAULT}|${SOLVER}`, 55n)
		getCode().mockResolvedValue("0x")
		const at = T0 + BigInt(RECONCILE_INTERVAL_SECS) + 60n

		await block(9_000n, at)

		expect(inventory()).toMatchObject({
			wallet: 900n,
			vaultShares: 55n,
			vaults: 110n,
			balance: 1_010n,
			lastReadBlock: 9_000n,
			blockNumber: 9_000n,
			lastLogIndex: AFTER_EVERY_LOG,
			genesisBlock: 101n,
			lastReconciledDrift: -100n + 10n,
			lastReconciledAt: new Date(Number(at) * 1000),
		})
		expect(records.get(`SolverDelegation:${CHAIN}-${SOLVER}`)).toMatchObject({
			delegated: false,
			blockNumber: 9_000n,
		})
		expect(logger.warn).toHaveBeenCalledWith(expect.stringContaining("Reconciliation drift -90"))

		// The read at 9000 already holds that block's logs.
		await transfer({ value: 50n, blockNumber: 9_000n, logIndex: 1 })
		expect(inventory()).toMatchObject({ wallet: 900n })
	})

	test("a page of due solvers is reconciled and revalued in three eth_calls", async () => {
		const solvers = Array.from({ length: 6 }, (_, i) => `0x${(0xb000 + i).toString(16).padStart(40, "0")}`)
		for (const solver of solvers) {
			mockOnchain.set(`${USDC}|${solver}`, 1_000n)
			mockOnchain.set(`${VAULT}|${solver}`, 50n)
			await fill(100n, solver)
		}
		await block(101n, T0)
		const at = T0 + BigInt(RECONCILE_INTERVAL_SECS) + 60n
		// Half the page was reconciled recently, so it is only due a revaluation.
		for (const solver of solvers.slice(0, 3)) {
			tracked(solver).reconciledAt = new Date(Number(at - 60n) * 1000)
		}
		mockRates.set(VAULT, 3n)
		ethCalls().mockClear()
		getCode().mockClear()

		await block(9_000n, at)

		// The reconciliations' balances and valuations, and the revaluations' valuations.
		expect(ethCalls()).toHaveBeenCalledTimes(3)
		expect(getCode()).toHaveBeenCalledTimes(3)
		for (const solver of solvers) {
			expect(inventory(USDC, solver)).toMatchObject({ wallet: 1_000n, vaults: 150n, balance: 1_150n })
		}
		expect(inventory(USDC, solvers[0]).lastReadBlock).toBe(101n)
		expect(inventory(USDC, solvers[5]).lastReadBlock).toBe(9_000n)
	})

	test("a solver whose read keeps failing does not pin the page to it", async () => {
		const solvers = Array.from({ length: 26 }, (_, i) => `0x${(0xc000 + i).toString(16).padStart(40, "0")}`)
		for (const solver of solvers) {
			mockOnchain.set(`${USDC}|${solver}`, 1_000n)
			await fill(100n, solver)
		}
		// Genesis runs 20 solvers a block.
		await block(101n, T0)
		await block(102n, T0 + 2n)
		expect(rows("TrackedSolver").every((row) => row.status === TrackedSolverStatus.TRACKED)).toBe(true)
		for (const solver of solvers) mockOnchain.set(`${USDC}|${solver}`, 900n)
		mockReverting.add(solvers[0])
		const at = T0 + BigInt(RECONCILE_INTERVAL_SECS) + 60n

		// The first page carries the failing solver, and still hands the next pass the page behind it.
		await block(9_000n, at)
		expect(head()).toMatchObject({ refreshOffset: 25 })
		expect(inventory(USDC, solvers[0])).toMatchObject({ wallet: 1_000n })
		expect(inventory(USDC, solvers[1])).toMatchObject({ wallet: 900n })

		await block(9_100n, at + 31n)
		expect(inventory(USDC, solvers[25])).toMatchObject({ wallet: 900n })
		expect(head()).toMatchObject({ refreshOffset: 0 })

		// Still due, the failed solver is read again when the cycle comes round.
		mockReverting.clear()
		await block(9_200n, at + 62n)
		expect(inventory(USDC, solvers[0])).toMatchObject({ wallet: 900n })
	})

	test("a failed refresh read writes nothing and resumes the same page next time", async () => {
		await trackSolver(1_000n, 50n)
		mockOnchain.set(`${USDC}|${SOLVER}`, 900n)
		getCode().mockRejectedValueOnce(new Error("rpc down"))
		const at = T0 + BigInt(RECONCILE_INTERVAL_SECS) + 60n

		await block(9_000n, at)
		expect(inventory()).toMatchObject({ wallet: 1_000n, lastReadBlock: 101n })
		expect(head()).toMatchObject({ refreshOffset: 0 })

		await block(9_020n, at + 40n)
		expect(inventory()).toMatchObject({ wallet: 900n, lastReadBlock: 9_020n })
	})
})

describe("parseDelegation", () => {
	test("counts only an exact 7702 designator for a known SolverAccount", () => {
		expect(parseDelegation(CHAIN, DELEGATED.toUpperCase().replace("0X", "0x"))).toEqual({
			delegated: true,
			delegate: SOLVER_ACCOUNT,
		})
		expect(parseDelegation(CHAIN, `0xef0100${OTHER.slice(2)}`)).toEqual({ delegated: false, delegate: OTHER })
		expect(parseDelegation(CHAIN, `${DELEGATED}00`)).toEqual({ delegated: false })
		expect(parseDelegation(CHAIN, "0x6080604052")).toEqual({ delegated: false })
		expect(parseDelegation("EVM-1", DELEGATED)).toEqual({ delegated: false, delegate: SOLVER_ACCOUNT })
	})
})
