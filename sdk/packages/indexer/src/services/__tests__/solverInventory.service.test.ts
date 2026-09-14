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

/** On-chain state the pinned reads return, keyed `contract|holder`. */
const mockOnchain = new Map<string, bigint>()
/** Assets per share, per vault. */
const mockRates = new Map<string, bigint>()
const mockBalanceReads: string[] = []
jest.mock("ethers", () => {
	const actual = jest.requireActual("ethers")
	return {
		...actual,
		ethers: {
			...actual.ethers,
			Contract: jest.fn((address: string) => ({
				balanceOf: jest.fn(async (holder: string) => {
					mockBalanceReads.push(address.toLowerCase())
					return (mockOnchain.get(`${address.toLowerCase()}|${holder.toLowerCase()}`) ?? 0n).toString()
				}),
				convertToAssets: jest.fn(async (shares: string) =>
					(BigInt(shares) * (mockRates.get(address.toLowerCase()) ?? 1n)).toString(),
				),
			})),
		},
	}
})
jest.mock("@/yield-vault-addresses", () => ({
	YIELD_VAULT_ADDRESSES: {
		"EVM-8453": {
			"0x833589fcd6edb6e08f4c7c32d4f71b54bda02913": ["0xC768c589647798a6EE01A91FdE98EF2ed046DBD6"],
			"0x46c85152bfe9f96829aa94755d9f915f9b10ef5f": [],
		},
	},
}))
jest.mock("@/solver-account-addresses", () => ({
	SOLVER_ACCOUNT_ADDRESSES: { "EVM-8453": ["0x7cb55539d1144F62422099c3FA3405092022c88C"] },
}))

import { ethers } from "ethers"
import { SolverDiscoveryTrigger, TrackedSolverStatus } from "@/configs/src/types"
import {
	AFTER_EVERY_LOG,
	applyTokenTransfer,
	applyVaultShareTransfer,
	discoverSolverFromFill,
	indexSolverInventoryBlock,
	parseDelegation,
	RECONCILE_INTERVAL_SECS,
	resetSolverInventoryCache,
	REVALUE_INTERVAL_SECS,
	type TransferInput,
} from "@/services/solverInventory.service"

const CHAIN = "EVM-8453"
const USDC = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
const CNGN = "0x46c85152bfe9f96829aa94755d9f915f9b10ef5f"
const VAULT = "0xc768c589647798a6ee01a91fde98ef2ed046dbd6"
const SOLVER = "0xce319986ca4d5d0893751a628d0db3dc8fc91d62"
const OTHER = "0x13e41cde1d55880cbe031c69f206c2e9bc3c94c2"
const ZERO = "0x0000000000000000000000000000000000000000"
const SOLVER_ACCOUNT = "0x7cb55539d1144f62422099c3fa3405092022c88c"
const DELEGATED = `0xef0100${SOLVER_ACCOUNT.slice(2)}`
const T0 = 1_750_000_000n

const getCode = () => (global as any).api.getCode as jest.Mock
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
	mockOnchain.clear()
	mockRates.clear()
	mockRates.set(VAULT, 2n)
	mockBalanceReads.length = 0
	jest.clearAllMocks()
	resetSolverInventoryCache()
	;(global as any).api = { getCode: jest.fn(async () => DELEGATED) }
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
		expect(getCode()).not.toHaveBeenCalled()
		expect(ethers.Contract).not.toHaveBeenCalled()
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
})

describe("events", () => {
	test("transfers between untracked addresses cost no store read and no RPC", async () => {
		await trackSolver()
		await transfer({ from: OTHER, to: "0x18f23e630077b1da3ed97c0469d0504a93fad9e2" })
		jest.mocked(store.get).mockClear()
		jest.mocked(store.getByFields).mockClear()
		jest.mocked(ethers.Contract).mockClear()
		getCode().mockClear()

		for (let i = 0; i < 2_000; i++) {
			const from = `0x${(i + 1).toString(16).padStart(40, "0")}`
			const to = `0x${(i + 2).toString(16).padStart(40, "0")}`
			await transfer({ from, to, blockNumber: 200n + BigInt(i) })
			await shareTransfer({ from, to, blockNumber: 200n + BigInt(i) })
		}

		expect(store.get).not.toHaveBeenCalled()
		expect(store.getByFields).not.toHaveBeenCalled()
		expect(ethers.Contract).not.toHaveBeenCalled()
		expect(getCode()).not.toHaveBeenCalled()
		expect(inventory()).toMatchObject({ wallet: 1_000n })
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
