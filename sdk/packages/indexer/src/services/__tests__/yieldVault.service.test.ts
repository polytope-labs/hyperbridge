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

const balances = new Map<string, bigint>()
let price: (shares: bigint) => bigint = (shares) => shares
const mockContract = {
	balanceOf: jest.fn(async (lp: string) => (balances.get(lp) ?? 0n).toString()),
	convertToAssets: jest.fn(async (shares: any) => price(BigInt(shares.toString())).toString()),
	totalAssets: jest.fn(async () => "1000000"),
	totalSupply: jest.fn(async () => "1000000"),
	decimals: jest.fn(async () => 6),
}
jest.mock("ethers", () => {
	const actual = jest.requireActual("ethers")
	return { ...actual, ethers: { ...actual.ethers, Contract: jest.fn(() => mockContract) } }
})
jest.mock("@/utils/vaultAccounting", () => ({
	...jest.requireActual("@/utils/vaultAccounting"),
	readVaultBlockMovements: jest.fn(async () => []),
}))
jest.mock("@/services/inventoryReading.service", () => ({ publishProviderInventory: jest.fn() }))
jest.mock("@/utils/solverBalance", () => ({ inventoryReadContext: jest.fn(() => ({})) }))

import { ethers } from "ethers"
import { VaultLedgerEventType as Type } from "@/configs/src/types"
import {
	YieldVaultService as Service,
	type VaultLedgerInput,
	type VaultTransferInput,
} from "@/services/yieldVault.service"
import { readVaultBlockMovements } from "@/utils/vaultAccounting"
import { publishProviderInventory } from "@/services/inventoryReading.service"

const CHAIN = "EVM-8453"
const VAULT = "0xc768c589647798a6ee01a91fde98ef2ed046dbd6"
const TOKEN = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
const LP = "0xce319986ca4d5d0893751a628d0db3dc8fc91d62"
const OTHER = "0x13e41cde1d55880cbe031c69f206c2e9bc3c94c2"
const THIRD = "0x18f23e630077b1da3ed97c0469d0504a93fad9e2"
const position = (lp = LP) => records.get(`VaultLpPosition:${CHAIN}-${VAULT}-${lp}`)
const getSnapshot = (lp = LP) => rows("VaultPositionSnapshot").find((row) => row.lp === lp)
const movements = jest.mocked(readVaultBlockMovements)

function ledger(overrides: Partial<VaultLedgerInput> = {}): VaultLedgerInput {
	return {
		chain: CHAIN,
		vault: VAULT,
		lp: LP,
		caller: LP,
		assets: 20n,
		shares: 20n,
		eventType: Type.DEPOSIT,
		blockNumber: 50n,
		transactionHash: "0xabc",
		logIndex: 2,
		timestamp: 86400n,
		...overrides,
	}
}
function transfer(overrides: Partial<VaultTransferInput> = {}): VaultTransferInput {
	return {
		chain: CHAIN,
		vault: VAULT,
		from: OTHER,
		to: LP,
		shares: 100n,
		blockNumber: 50n,
		transactionHash: "0xdef",
		logIndex: 3,
		timestamp: 86400n,
		...overrides,
	}
}
function blockEvents(...events: VaultLedgerInput[]): void {
	movements.mockResolvedValue(
		events.map((e) => ({
			transactionHash: e.transactionHash,
			logIndex: e.logIndex,
			lp: e.lp,
			shares: e.eventType === Type.DEPOSIT || e.eventType === Type.TRANSFER_IN ? e.shares : -e.shares,
		})),
	)
}
function legacy(lp = LP, shares = 100n, deposited = 100n): void {
	const id = `${CHAIN}-${VAULT}-${lp}`
	records.set(`VaultLpPosition:${id}`, {
		id,
		chain: CHAIN,
		vault: VAULT,
		underlyingToken: TOKEN,
		lp,
		shares,
		totalAssetsDeposited: deposited,
		totalAssetsWithdrawn: 0n,
		depositCount: 1,
		withdrawCount: 0,
		createdAt: new Date(0),
		lastUpdatedAt: new Date(0),
	})
	balances.set(lp, shares)
}
async function snapshot(): Promise<void> {
	movements.mockResolvedValue([])
	await Service.snapshotChain(CHAIN, 100n, 86500n)
}

beforeEach(() => {
	jest.clearAllMocks()
	jest.restoreAllMocks()
	records.clear()
	balances.clear()
	price = (shares) => shares
	movements.mockReset().mockResolvedValue([])
	mockContract.balanceOf.mockReset().mockImplementation(async (lp) => (balances.get(lp) ?? 0n).toString())
	mockContract.convertToAssets
		.mockReset()
		.mockImplementation(async (shares) => price(BigInt(shares.toString())).toString())
	;(global as any).api = { getCode: jest.fn(async () => "0x") }
	jest.spyOn(Service, "configuredVaults").mockReturnValue([{ vault: VAULT, underlyingToken: TOKEN }])
	records.set(`LiquidityProvider:${LP}`, { id: LP })
})

it("seeds shares received before delegation as opening capital at the first tracked event", async () => {
	records.delete(`LiquidityProvider:${LP}`)
	await Service.recordTransfer(transfer())
	expect(position()).toBeUndefined()
	;(global as any).api.getCode.mockResolvedValue("0xef01007cb55539d1144f62422099c3fa3405092022c88c")
	balances.set(LP, 1020n)
	blockEvents(ledger())
	await Service.recordLedger(ledger())
	expect(position()).toMatchObject({
		openingShares: 1000n,
		openingPrincipal: 1000n,
		openingBlock: 50n,
		shares: 1020n,
		totalAssetsDeposited: 20n,
		depositCount: 1,
	})
	price = (shares) => (shares * 101n) / 100n
	await snapshot()
	expect(getSnapshot()).toMatchObject({ netPrincipal: 1020n, assetValue: 1030n, yieldEarned: 10n })
})

it("backs out all later same-block movements when establishing the opening balance", async () => {
	const deposit = ledger()
	const incoming = ledger({ eventType: Type.TRANSFER_IN, shares: 30n, assets: 30n, logIndex: 4 })
	const withdrawal = ledger({ eventType: Type.WITHDRAW, shares: 10n, assets: 10n, logIndex: 6 })
	balances.set(LP, 1040n)
	blockEvents(deposit, incoming, withdrawal)
	await Service.recordLedger(deposit)
	await Service.recordLedger(incoming)
	await Service.recordLedger(withdrawal)
	expect(position()).toMatchObject({
		openingShares: 1000n,
		openingPrincipal: 1000n,
		shares: 1040n,
		totalAssetsDeposited: 20n,
		totalAssetsWithdrawn: 10n,
		totalAssetsTransferredIn: 30n,
	})
	await snapshot()
	expect(getSnapshot().yieldEarned).toBe(0n)
})

it("establishes opening capital even when the first tracked event withdraws the entire position", async () => {
	const withdrawal = ledger({ eventType: Type.WITHDRAW, assets: 110n, shares: 100n })
	price = (shares) => (shares * 110n) / 100n
	blockEvents(withdrawal)
	await Service.recordLedger(withdrawal)
	expect(position()).toMatchObject({
		openingShares: 100n,
		openingPrincipal: 110n,
		shares: 0n,
		totalAssetsWithdrawn: 110n,
		totalAssetsDeposited: 0n,
	})
	await snapshot()
	expect(getSnapshot().yieldEarned).toBe(0n)
})

it("rejects impossible opening balances without recording the event", async () => {
	blockEvents(ledger())
	await expect(Service.recordLedger(ledger())).rejects.toThrow("Negative vault opening shares")
	expect(rows("VaultLedgerEvent")).toHaveLength(0)
	expect(position()).toBeUndefined()
})

it("keeps the reported wallet's transferred USDC principal out of yield", async () => {
	legacy(LP, 0n, 0n)
	const values = new Map([
		[174220559508n, 199506902673n],
		[57243722070n, 65552073822n],
		[231481948014n, 265087704647n],
	])
	price = (shares) => values.get(shares) ?? shares
	await Service.recordTransfer(transfer({ shares: 174220559508n }))
	await Service.recordTransfer(transfer({ from: THIRD, shares: 57243722070n, logIndex: 4 }))
	await Service.recordLedger(ledger({ assets: 20230586n, shares: 17666436n, logIndex: 5 }))
	balances.set(LP, 231481948014n)
	await snapshot()
	expect(position()).toMatchObject({
		shares: 231481948014n,
		totalAssetsDeposited: 20230586n,
		totalAssetsTransferredIn: 265058976495n,
	})
	expect(getSnapshot()).toMatchObject({ netPrincipal: 265079207081n, yieldEarned: 8497566n })
})

it("accounts for both tracked owners once and preserves earned yield when all shares leave", async () => {
	legacy(OTHER, 100n, 100n)
	price = (shares) => (shares * 110n) / 100n
	balances.set(OTHER, 0n)
	balances.set(LP, 100n)
	const event = transfer()
	blockEvents(
		ledger({
			lp: LP,
			eventType: Type.TRANSFER_IN,
			shares: 100n,
			transactionHash: event.transactionHash,
			logIndex: 3,
		}),
		ledger({
			lp: OTHER,
			eventType: Type.TRANSFER_OUT,
			shares: 100n,
			transactionHash: event.transactionHash,
			logIndex: 3,
		}),
	)
	await Service.recordTransfer(event)
	await Service.recordTransfer(event)
	expect(rows("VaultLedgerEvent")).toHaveLength(2)
	expect(position(OTHER)).toMatchObject({ shares: 0n, totalAssetsTransferredOut: 110n, withdrawCount: 0 })
	expect(position()).toMatchObject({
		shares: 100n,
		openingPrincipal: 0n,
		totalAssetsTransferredIn: 110n,
		depositCount: 0,
	})
	expect(publishProviderInventory).toHaveBeenCalledTimes(2)
	await snapshot()
	expect(getSnapshot(OTHER).yieldEarned).toBe(10n)
	expect(getSnapshot().yieldEarned).toBe(0n)
})

it.each([{ from: ethers.constants.AddressZero }, { to: ethers.constants.AddressZero }, { from: LP }, { shares: 0n }])(
	"ignores mint, burn, self and zero-share transfers: %p",
	async (overrides) => {
		await Service.recordTransfer(transfer(overrides))
		expect(rows("VaultLedgerEvent")).toHaveLength(0)
		expect(mockContract.convertToAssets).not.toHaveBeenCalled()
	},
)

it("counts mint/deposit and burn/withdraw only once", async () => {
	balances.set(LP, 20n)
	blockEvents(ledger())
	await Service.recordTransfer(transfer({ from: ethers.constants.AddressZero, shares: 20n }))
	await Service.recordLedger(ledger())
	await Service.recordLedger(ledger())
	await Service.recordTransfer(transfer({ from: LP, to: ethers.constants.AddressZero, shares: 20n }))
	await Service.recordLedger(ledger({ eventType: Type.WITHDRAW, logIndex: 10 }))
	expect(position()).toMatchObject({
		shares: 0n,
		totalAssetsDeposited: 20n,
		totalAssetsWithdrawn: 20n,
		totalAssetsTransferredIn: 0n,
		totalAssetsTransferredOut: 0n,
		depositCount: 1,
		withdrawCount: 1,
	})
})

it("does not persist an event if the opening balance RPC is incomplete, then succeeds on retry", async () => {
	balances.set(LP, 20n)
	await expect(Service.recordLedger(ledger())).rejects.toThrow("missing the triggering log")
	expect(rows("VaultLedgerEvent")).toHaveLength(0)
	expect(position()).toBeUndefined()
	blockEvents(ledger())
	await Service.recordLedger(ledger())
	expect(position().shares).toBe(20n)
})

it("retries delegation RPC failures instead of silently discarding a transfer", async () => {
	;(global as any).api.getCode.mockRejectedValue(new Error("RPC unavailable"))
	await expect(Service.recordTransfer(transfer())).rejects.toThrow("RPC unavailable")
	expect(rows("VaultLedgerEvent")).toHaveLength(0)
})

it("withholds snapshots for unexplained legacy share balances", async () => {
	legacy(LP, 17666436n, 20230586n)
	legacy(OTHER)
	balances.set(LP, 231481948014n)
	await snapshot()
	expect(getSnapshot()).toBeUndefined()
	expect(getSnapshot(OTHER)).toBeDefined()
	expect(rows("VaultSnapshot")).toHaveLength(1)
	expect((global as any).logger.error).toHaveBeenCalledWith(
		expect.stringContaining("principal requires reconciliation"),
	)
})

it("defers snapshots before same-block logs even when their net share movement is zero", async () => {
	legacy()
	blockEvents(
		ledger({ assets: 10n, shares: 10n }),
		ledger({ eventType: Type.WITHDRAW, assets: 11n, shares: 10n, logIndex: 4 }),
	)
	await Service.snapshotChain(CHAIN, 50n, 86400n)
	expect(getSnapshot()).toBeUndefined()
	expect(rows("VaultSnapshot")).toHaveLength(0)
	await Service.recordLedger(ledger({ assets: 10n, shares: 10n }))
	await Service.recordLedger(ledger({ eventType: Type.WITHDRAW, assets: 11n, shares: 10n, logIndex: 4 }))
	await snapshot()
	expect(getSnapshot().yieldEarned).toBe(1n)
	expect(rows("VaultSnapshot")).toHaveLength(1)
})

it("retries failed LP snapshots without rewriting already completed LPs", async () => {
	legacy()
	legacy(OTHER)
	mockContract.balanceOf.mockImplementation(async (lp) => {
		if (lp === OTHER) throw new Error("RPC unavailable")
		return "100"
	})
	await snapshot()
	expect(getSnapshot()).toBeDefined()
	expect(getSnapshot(OTHER)).toBeUndefined()
	expect(rows("VaultSnapshot")).toHaveLength(0)
	mockContract.balanceOf.mockResolvedValue("100")
	price = (shares) => shares * 2n
	await snapshot()
	expect(getSnapshot().yieldEarned).toBe(0n)
	expect(getSnapshot(OTHER).yieldEarned).toBe(100n)
	expect(rows("VaultSnapshot")).toHaveLength(1)
})

it("preserves genuine losses instead of clamping yield to zero", async () => {
	legacy()
	price = (shares) => (shares * 9n) / 10n
	await snapshot()
	expect(getSnapshot().yieldEarned).toBe(-10n)
})

it("does not repeat transfer valuation after both owners' ledger entries already exist", async () => {
	legacy(OTHER)
	legacy(LP, 0n, 0n)
	await Service.recordTransfer(transfer())
	mockContract.convertToAssets.mockRejectedValueOnce(new Error("RPC unavailable"))
	await expect(Service.recordTransfer(transfer())).resolves.toBeUndefined()
	expect(rows("VaultLedgerEvent")).toHaveLength(2)
})

it("keeps principal when best-effort inventory publication fails", async () => {
	legacy()
	jest.mocked(publishProviderInventory).mockRejectedValueOnce(new Error("inventory RPC unavailable"))
	await Service.recordLedger(ledger())
	expect(position()).toMatchObject({ shares: 120n, totalAssetsDeposited: 120n })
	expect(rows("VaultLedgerEvent")).toHaveLength(1)
})

it("ignores unknown chains and vaults without reading balances or writing accounting", async () => {
	await Service.recordLedger(ledger({ chain: "EVM-999" }))
	await Service.recordLedger(ledger({ vault: OTHER }))
	await Service.recordTransfer(transfer({ vault: OTHER }))
	expect(rows("VaultLedgerEvent")).toHaveLength(0)
	expect(mockContract.balanceOf).not.toHaveBeenCalled()
})

it.each(["0x", "0x60006000", "0xef0100" + "ab".repeat(20), "0xef0100"])(
	"rejects unrelated or malformed delegations: %s",
	async (code) => {
		;(global as any).api.getCode.mockResolvedValue(code)
		expect(await Service.isDelegatedSolver(CHAIN, OTHER)).toBe(false)
	},
)

it("matches configured vaults and SolverAccount addresses case-insensitively", async () => {
	jest.mocked(Service.configuredVaults).mockRestore()
	expect(Service.configuredVaults("EVM-999")).toEqual([])
	expect(Service.configuredVaults(CHAIN)).toContainEqual({ vault: VAULT, underlyingToken: TOKEN })
	expect(Service.underlyingTokenFor(CHAIN, ethers.utils.getAddress(VAULT))).toBe(TOKEN)
	;(global as any).api.getCode.mockResolvedValue("0xEF01007CB55539D1144F62422099C3FA3405092022C88C")
	expect(await Service.isDelegatedSolver(CHAIN, LP)).toBe(true)
	expect(await Service.isDelegatedSolver("EVM-999", LP)).toBe(false)
})

it.each([new Error("vault RPC unavailable"), "vault RPC unavailable"])(
	"does not close the daily gate after a vault RPC failure: %p",
	async (error) => {
		legacy()
		movements.mockRejectedValueOnce(error)
		await Service.snapshotChain(CHAIN, 100n, 86500n)
		expect(getSnapshot()).toBeUndefined()
		expect(rows("VaultSnapshot")).toHaveLength(0)
		await snapshot()
		expect(getSnapshot()).toBeDefined()
	},
)
