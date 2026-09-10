import fs from "node:fs"
import path from "node:path"

// Exercise the actual handlers, generated models, ABI decoder and single-block log reader.
// Only the store, RPC boundary and unrelated inventory publisher are replaced. The reference
// model maintains onchain shares and cash flows independently of the indexer's ledger fields.
const records = new Map<string, any>()
const rows = (entity: string) =>
	[...records.entries()].filter(([key]) => key.startsWith(`${entity}:`)).map(([, row]) => row)
const mockState = new Map<
	string,
	{ shares: Map<string, bigint>; cash: Map<string, bigint>; numerator: bigint; denominator: bigint }
>()
const mockCodes = new Map<string, string>()
const mockContract = (vault: string) => {
	const state = mockState.get(`EVM-${(global as any).chainId}:${vault.toLowerCase()}`)!
	return {
		balanceOf: async (lp: string) => (state.shares.get(lp.toLowerCase()) ?? 0n).toString(),
		convertToAssets: async (shares: any) =>
			((BigInt(shares.toString()) * state.numerator) / state.denominator).toString(),
		totalAssets: async () => "1000000000000",
		totalSupply: async () => "1000000000000",
		decimals: async () => (state.denominator === 10n ** 12n ? 18 : 6),
	}
}
jest.mock("ethers", () => {
	const actual = jest.requireActual("ethers")
	return { ...actual, ethers: { ...actual.ethers, Contract: jest.fn((vault) => mockContract(vault)) } }
})
jest.mock("@/constants", () => ({
	ENV_CONFIG: { "EVM-8453": "https://base.example", "EVM-42161": "https://arb.example" },
}))
jest.mock("@/utils/rpc.helpers", () => ({
	getBlockTimestamp: jest.fn(),
	replaceWebsocketWithHttp: (url: string) => url,
}))
jest.mock("@/utils/substrate.helpers", () => ({ getHostStateMachine: (chain: string) => `EVM-${chain}` }))
jest.mock("@/utils/safeFetch", () => ({ safeFetch: jest.fn() }))
jest.mock("@/services/inventoryReading.service", () => ({ publishProviderInventory: jest.fn() }))
jest.mock("@/utils/solverBalance", () => ({ inventoryReadContext: jest.fn(() => ({})) }))

import { ethers } from "ethers"
import Erc4626Abi from "@/configs/abis/Erc4626.abi.json"
import { YieldVaultService as Service } from "@/services/yieldVault.service"
import { handleVaultDepositEvent } from "@/handlers/events/yieldVault/deposit.event.handler"
import { handleVaultWithdrawEvent } from "@/handlers/events/yieldVault/withdraw.event.handler"
import { handleVaultTransferEvent } from "@/handlers/events/yieldVault/transfer.event.handler"
import { handleVaultSnapshotIndexing } from "@/handlers/events/yieldVault/snapshot.block.handler"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { safeFetch } from "@/utils/safeFetch"
import { wrap } from "@/utils/event.utils"

const BASE = "EVM-8453"
const ARB = "EVM-42161"
const USDC = "0xc768c589647798a6ee01a91fde98ef2ed046dbd6"
const CNGN = "0xa82a3531021317240fb32e67f9c7bc091f737d3b"
const ARB_USDC = "0x7f6501d3b98ee91f9b9535e4b0ac710fb0f9e0bc"
const TOKEN = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
const address = (i: number) => `0x${i.toString(16).padStart(40, "0")}`
const A = address(1),
	B = address(2),
	C = address(3),
	ROUTER = address(9)
const ZERO = ethers.constants.AddressZero
const abi = new ethers.utils.Interface(Erc4626Abi)
let height = 100
let day = 20000n
let logs: any[] = []
let timestamp = day * 86400n
const state = (vault = USDC, chain = BASE) => mockState.get(`${chain}:${vault}`)!
const position = (lp: string, vault = USDC, chain = BASE) => records.get(`VaultLpPosition:${chain}-${vault}-${lp}`)
const latest = (lp: string, vault = USDC, chain = BASE) =>
	rows("VaultPositionSnapshot")
		.filter((row) => row.lp === lp && row.vault === vault && row.chain === chain)
		.sort((x, y) => Number(y.dayStartTimestamp - x.dayStartTimestamp))[0]
const register = (lp: string) => records.set(`LiquidityProvider:${lp}`, { id: lp })
const value = (shares: bigint, vault = USDC, chain = BASE) =>
	(shares * state(vault, chain).numerator) / state(vault, chain).denominator
const ceil = (n: bigint, d: bigint) => (n + d - 1n) / d

type Action = {
	kind: "deposit" | "mint" | "withdraw" | "redeem" | "share" | "token"
	lp?: string
	from?: string
	to?: string
	caller?: string
	amount: bigint
	vault?: string
}

function prepare(actions: Action[], chain = BASE): any[] {
	height++
	;(global as any).chainId = chain.slice(4)
	timestamp = day * 86400n + BigInt(height)
	logs = []
	for (const [i, action] of actions.entries()) {
		const vault = action.vault ?? USDC
		const s = state(vault, chain)
		const lp = action.lp ?? A
		const caller = action.caller ?? lp
		const receiver = action.to ?? lp
		const tx = `0x${(height * 1000 + i).toString(16).padStart(64, "0")}`
		const emit = (name: string, args: any[], contract = vault) =>
			logs.push({
				...abi.encodeEventLog(abi.getEvent(name), args),
				address: contract,
				transactionHash: tx,
				blockNumber: height,
				blockHash: `0x${height.toString(16).padStart(64, "0")}`,
				logIndex: logs.length,
			})
		const change = (owner: string, shares: bigint, cash: bigint) => {
			s.shares.set(owner, (s.shares.get(owner) ?? 0n) + shares)
			s.cash.set(owner, (s.cash.get(owner) ?? 0n) + cash)
			if (s.shares.get(owner)! < 0n) throw new Error("invalid test action: insufficient shares")
		}
		if (action.kind === "token") {
			// Plain underlying-token transfers never touch the vault share supply or LP cash basis.
			emit("Transfer", [action.from ?? A, receiver, action.amount], TOKEN)
		} else if (action.kind === "share") {
			const from = action.from ?? A
			const assets = value(action.amount, vault, chain)
			change(from, -action.amount, -assets)
			change(receiver, action.amount, assets)
			emit("Transfer", [from, receiver, action.amount])
		} else if (action.kind === "deposit" || action.kind === "mint") {
			const shares = action.kind === "mint" ? action.amount : (action.amount * s.denominator) / s.numerator
			const assets = action.kind === "deposit" ? action.amount : ceil(shares * s.numerator, s.denominator)
			change(lp, shares, assets)
			emit("Transfer", [ZERO, lp, shares])
			emit("Deposit", [caller, lp, assets, shares])
		} else {
			const shares = action.kind === "redeem" ? action.amount : ceil(action.amount * s.denominator, s.numerator)
			const assets = action.kind === "withdraw" ? action.amount : value(shares, vault, chain)
			change(lp, -shares, -assets)
			emit("Transfer", [lp, ZERO, shares])
			emit("Withdraw", [caller, receiver, lp, assets, shares])
			// ERC-20 underlying Transfer emitted by the redemption is not a share transfer.
			emit("Transfer", [vault, receiver, assets], TOKEN)
		}
	}
	return logs
}

async function dispatch(blockLogs = logs): Promise<void> {
	for (const log of blockLogs) {
		if (!Service.underlyingTokenFor(`EVM-${(global as any).chainId}`, log.address)) continue
		const decoded = abi.parseLog(log)
		const event = { ...log, args: decoded.args }
		if (decoded.name === "Deposit") await handleVaultDepositEvent(event as any)
		else if (decoded.name === "Withdraw") await handleVaultWithdrawEvent(event as any)
		else await handleVaultTransferEvent(event as any)
	}
}
async function block(actions: Action[], chain = BASE): Promise<void> {
	prepare(actions, chain)
	await dispatch()
}
async function snapshot(chain = BASE, nextDay = true): Promise<void> {
	if (nextDay) day++
	prepare([], chain)
	await handleVaultSnapshotIndexing({ number: height, timestamp } as any)
}
function checkOracle(vault = USDC, chain = BASE): void {
	for (const [lp, cash] of state(vault, chain).cash) {
		const p = position(lp, vault, chain)
		if (!p) continue
		expect(p.shares).toBe(state(vault, chain).shares.get(lp))
		expect(latest(lp, vault, chain)).toMatchObject({
			assetValue: value(p.shares, vault, chain),
			netPrincipal: cash,
			yieldEarned: value(p.shares, vault, chain) - cash,
		})
	}
}

beforeEach(() => {
	jest.restoreAllMocks()
	jest.clearAllMocks()
	records.clear()
	mockState.clear()
	mockCodes.clear()
	height = 100
	day = 20000n
	logs = []
	;(global as any).chainId = "8453"
	;(global as any).logger = { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() }
	;(global as any).api = { getCode: jest.fn(async (lp: string) => mockCodes.get(lp) ?? "0x") }
	;(global as any).store = {
		get: jest.fn(async (entity: string, id: string) => records.get(`${entity}:${id}`)),
		set: jest.fn(async (entity: string, id: string, props: any) => records.set(`${entity}:${id}`, { ...props })),
		getByFields: jest.fn(async (entity: string, filters: [string, string, any][], options: any) =>
			rows(entity)
				.filter((row) => filters.every(([field, , v]) => row[field] === v))
				.sort((x, y) => x.id.localeCompare(y.id))
				.slice(options.offset ?? 0, (options.offset ?? 0) + options.limit),
		),
	}
	for (const [chain, vault] of [
		[BASE, USDC],
		[BASE, CNGN],
		[ARB, ARB_USDC],
	]) {
		mockState.set(`${chain}:${vault}`, { shares: new Map(), cash: new Map(), numerator: 1n, denominator: 1n })
	}
	jest.spyOn(Service, "configuredVaults").mockImplementation((chain) =>
		(chain === BASE ? [USDC, CNGN] : [ARB_USDC]).map((vault) => ({
			vault,
			underlyingToken: Service.underlyingTokenFor(chain, vault)!,
		})),
	)
	jest.mocked(getBlockTimestamp)
		.mockReset()
		.mockImplementation(async () => timestamp)
	jest.mocked(safeFetch)
		.mockReset()
		.mockImplementation(async (_url, options) => {
			const filter = JSON.parse(options!.body!).params[0]
			expect(filter.fromBlock).toBe(`0x${height.toString(16)}`)
			expect(filter.toBlock).toBe(filter.fromBlock)
			return {
				ok: true,
				json: async () => ({
					result: logs
						.filter((log) => log.address === filter.address)
						.map((log) => ({
							...log,
							blockNumber: `0x${log.blockNumber.toString(16)}`,
							logIndex: `0x${log.logIndex.toString(16)}`,
						})),
				}),
			} as any
		})
	for (const lp of [A, B, C]) register(lp)
})

it("mints, deposits, partially withdraws, redeems fully, then deposits again without losing earned yield", async () => {
	await block([
		{ kind: "deposit", amount: 1000n },
		{ kind: "mint", amount: 100n },
	])
	state().numerator = 11n
	state().denominator = 10n
	await block([{ kind: "withdraw", amount: 330n, caller: ROUTER, to: B }])
	await snapshot()
	checkOracle()
	expect(latest(A).yieldEarned).toBe(110n)
	await block([{ kind: "redeem", amount: 800n }])
	await snapshot()
	expect(latest(A)).toMatchObject({ shares: 0n, yieldEarned: 110n })
	await block([{ kind: "deposit", amount: 110n }])
	await snapshot()
	checkOracle()
	expect(position(B)).toBeUndefined()
	expect(position(ROUTER)).toBeUndefined()
	expect(position(A)).toMatchObject({
		depositCount: 3,
		withdrawCount: 2,
		totalAssetsTransferredIn: 0n,
		totalAssetsTransferredOut: 0n,
	})
})

it.each([0n, 500n])(
	"Simplex redeem-to-recipient changes only the owner's principal (recipient starts with %s shares)",
	async (initial) => {
		await block([
			{ kind: "deposit", lp: A, amount: 1000n },
			...(initial ? [{ kind: "deposit" as const, lp: B, amount: initial }] : []),
		])
		state().numerator = 12n
		state().denominator = 10n
		await block([{ kind: "redeem", lp: A, amount: 200n, to: B }])
		await snapshot()
		checkOracle()
		expect(latest(A).yieldEarned).toBe(200n)
		expect(position(A).totalAssetsWithdrawn).toBe(240n)
		if (initial) expect(latest(B)).toMatchObject({ shares: 500n, netPrincipal: 500n, yieldEarned: 100n })
		else expect(position(B)).toBeUndefined()
		// Receiving underlying and later sweeping it creates exactly one new deposit.
		await block([{ kind: "deposit", lp: B, amount: 240n }])
		await snapshot()
		checkOracle()
		expect(position(B).totalAssetsDeposited).toBe(initial + 240n)
	},
)

it.each([0n, 500n])(
	"direct share transfers preserve both owners' prior yield (recipient starts with %s shares)",
	async (initial) => {
		await block([
			{ kind: "deposit", lp: A, amount: 1000n },
			...(initial ? [{ kind: "deposit" as const, lp: B, amount: initial }] : []),
		])
		state().numerator = 12n
		state().denominator = 10n
		await block([{ kind: "share", from: A, to: B, amount: 200n }])
		await snapshot()
		checkOracle()
		expect(latest(A).yieldEarned).toBe(200n)
		expect(latest(B).yieldEarned).toBe(initial / 5n)
		expect(position(B).totalAssetsTransferredIn).toBe(240n)
		await block([{ kind: "share", from: B, to: A, amount: 100n }])
		await snapshot()
		checkOracle()
	},
)

it("raw underlying sends and paymaster charges leave existing vault principal alone", async () => {
	await block([{ kind: "deposit", amount: 1000n }])
	const before = { ...position(A) }
	await block([
		{ kind: "token", from: A, to: B, amount: 999999n },
		{ kind: "token", from: B, to: A, amount: 500n },
		{ kind: "token", from: A, to: ROUTER, amount: 1n },
	])
	expect(position(A)).toEqual(before)
	expect(position(B)).toBeUndefined()
	await snapshot()
	checkOracle()
})

it("Simplex withdraw-shortfall plus underlying transfer and later sweep counts each vault event once", async () => {
	await block([{ kind: "deposit", amount: 1000n }])
	state().numerator = 11n
	state().denominator = 10n
	await block([
		{ kind: "withdraw", amount: 220n },
		{ kind: "token", from: A, to: B, amount: 300n },
		{ kind: "deposit", lp: B, amount: 220n },
	])
	await snapshot()
	checkOracle()
	expect(latest(A).yieldEarned).toBe(100n)
	expect(latest(B).yieldEarned).toBe(0n)
})

it("attributes third-party deposits to the share owner and third-party withdrawals to the burned-share owner", async () => {
	await block([{ kind: "deposit", lp: B, caller: ROUTER, amount: 1000n }])
	state().numerator = 11n
	state().denominator = 10n
	await block([{ kind: "withdraw", lp: B, caller: ROUTER, to: C, amount: 110n }])
	await snapshot()
	checkOracle()
	expect(latest(B).yieldEarned).toBe(100n)
	for (const lp of [A, C, ROUTER]) expect(position(lp)).toBeUndefined()
})

it("retains an existing recipient's deposit/withdrawal history when shares are added and removed", async () => {
	await block([
		{ kind: "deposit", lp: A, amount: 1000n },
		{ kind: "deposit", lp: B, amount: 500n },
	])
	state().numerator = 12n
	state().denominator = 10n
	await block([
		{ kind: "withdraw", lp: B, amount: 120n },
		{ kind: "share", from: A, to: B, amount: 200n },
		{ kind: "deposit", lp: B, amount: 120n },
		{ kind: "share", from: B, to: A, amount: 50n },
	])
	await snapshot()
	checkOracle()
	expect(position(B)).toMatchObject({
		totalAssetsDeposited: 620n,
		totalAssetsWithdrawn: 120n,
		totalAssetsTransferredIn: 240n,
		totalAssetsTransferredOut: 60n,
		depositCount: 2,
		withdrawCount: 1,
	})
	expect(latest(B).yieldEarned).toBe(100n)
})

it.each([
	[true, true],
	[true, false],
	[false, true],
	[false, false],
])("tracks the eligible sides of a share transfer (sender=%s, recipient=%s)", async (sender, recipient) => {
	if (!sender) records.delete(`LiquidityProvider:${A}`)
	if (!recipient) records.delete(`LiquidityProvider:${B}`)
	await block([{ kind: "deposit", lp: A, amount: 1000n }])
	state().numerator = 12n
	state().denominator = 10n
	await block([{ kind: "share", from: A, to: B, amount: 100n }])
	await snapshot()
	checkOracle()
	expect(!!position(A)).toBe(sender)
	expect(!!position(B)).toBe(recipient)
})

it("uses a new opening basis for pre-delegation shares and continues tracking after delegation is revoked", async () => {
	records.delete(`LiquidityProvider:${B}`)
	await block([
		{ kind: "deposit", lp: A, amount: 1000n },
		{ kind: "share", from: A, to: B, amount: 200n },
	])
	expect(position(B)).toBeUndefined()
	state().numerator = 12n
	state().denominator = 10n
	mockCodes.set(B, "0xef01007cb55539d1144f62422099c3fa3405092022c88c")
	state().cash.set(B, 240n) // B's measurement starts here; pre-tracking appreciation is excluded.
	await block([{ kind: "deposit", lp: B, amount: 120n }])
	expect(position(B)).toMatchObject({ openingShares: 200n, openingPrincipal: 240n })
	mockCodes.delete(B)
	await block([
		{ kind: "share", from: B, to: A, amount: 50n },
		{ kind: "redeem", lp: B, amount: 250n },
	])
	await snapshot()
	checkOracle()
	expect(latest(B)).toMatchObject({ shares: 0n, yieldEarned: 0n })
})

it.each([handleVaultDepositEvent, handleVaultWithdrawEvent, handleVaultTransferEvent])(
	"propagates undecodable capital events instead of acknowledging and losing them",
	async (handler) => {
		const event = {
			get args(): never {
				throw new Error("ABI decoding failed")
			},
		}
		await expect(handler(event as any)).rejects.toThrow("ABI decoding failed")
	},
)

it.each([handleVaultDepositEvent, handleVaultWithdrawEvent, handleVaultTransferEvent])(
	"rejects missing decoded capital arguments instead of silently skipping the event",
	async (handler) => {
		await expect(handler({} as any)).rejects.toThrow("Missing decoded")
	},
)

it("normalizes transaction hash casing for replay-safe ledger IDs", async () => {
	prepare([{ kind: "deposit", amount: 1000n }])
	for (const log of logs) log.transactionHash = `0x${"ab".repeat(32)}`
	await dispatch()
	await dispatch(logs.map((log) => ({ ...log, transactionHash: `0x${"AB".repeat(32)}` })))
	expect(position(A)).toMatchObject({ shares: 1000n, totalAssetsDeposited: 1000n, depositCount: 1 })
})

it.each([
	[ZERO, A, 100n],
	[A, ZERO, 100n],
	[A, A, 100n],
	[A, B, 0n],
])("ignores non-capital Transfer events before requesting timestamps (%s → %s, %s)", async (from, to, amount) => {
	jest.mocked(getBlockTimestamp).mockRejectedValueOnce(new Error("timestamp RPC unavailable"))
	const encoded = abi.encodeEventLog(abi.getEvent("Transfer"), [from, to, amount])
	await expect(
		handleVaultTransferEvent({
			address: USDC,
			args: abi.parseLog(encoded).args,
			blockNumber: height,
			blockHash: `0x${height.toString(16).padStart(64, "0")}`,
			transactionHash: `0x${"ab".repeat(32)}`,
			logIndex: 0,
		} as any),
	).resolves.toBeUndefined()
	expect(getBlockTimestamp).not.toHaveBeenCalled()
})

it("handles multiple transactions and share round trips in one block, including new owners", async () => {
	await block([
		{ kind: "deposit", lp: A, amount: 1000n },
		{ kind: "share", from: A, to: B, amount: 300n },
		{ kind: "share", from: B, to: C, amount: 100n },
		{ kind: "share", from: C, to: A, amount: 100n },
		{ kind: "withdraw", lp: B, amount: 50n },
		{ kind: "deposit", lp: A, amount: 100n },
	])
	await dispatch() // duplicate delivery of every log, including both sides of each transfer
	await snapshot()
	checkOracle()
	for (const lp of [A, B, C]) expect(latest(lp).yieldEarned).toBe(0n)
})

it.each([USDC, CNGN])("respects rounding and distinct share/underlying decimals for %s", async (vault) => {
	state(vault).numerator = 1003n
	state(vault).denominator = vault === CNGN ? 10n ** 12n : 1000n
	await block([
		{ kind: "deposit", vault, amount: 1000001n },
		{ kind: "mint", vault, lp: B, amount: 101n },
	])
	await block([
		{ kind: "share", vault, from: A, to: B, amount: 1n },
		{ kind: "withdraw", vault, amount: 333n },
	])
	await snapshot()
	checkOracle(vault)
	// Real floor/ceil loss remains signed; a zero-asset dust transfer still moves shares.
	expect(latest(A, vault).yieldEarned).toBeLessThanOrEqual(0n)
})

it("isolates the same LP across two vaults and chains", async () => {
	await block([
		{ kind: "deposit", amount: 100n },
		{ kind: "deposit", vault: CNGN, amount: 200n },
	])
	await block([{ kind: "deposit", vault: ARB_USDC, amount: 300n }], ARB)
	state(CNGN).numerator = 2n
	await block([{ kind: "share", from: A, to: B, vault: CNGN, amount: 50n }])
	await snapshot()
	await snapshot(ARB)
	checkOracle()
	checkOracle(CNGN)
	checkOracle(ARB_USDC, ARB)
	expect(latest(A).yieldEarned).toBe(0n)
	expect(latest(A, CNGN).yieldEarned).toBe(200n)
	expect(latest(A, ARB_USDC, ARB).yieldEarned).toBe(0n)
})

it("snapshots every LP across the store's 100-row page boundary", async () => {
	const owners = Array.from({ length: 205 }, (_, i) => address(i + 100))
	owners.forEach(register)
	for (const lp of owners) await block([{ kind: "deposit", lp, amount: 1000n }])
	await snapshot()
	expect(rows("VaultPositionSnapshot")).toHaveLength(205)
	checkOracle()
})

it("handles UTC day rollover and repeated daily snapshot invocations", async () => {
	await block([{ kind: "deposit", amount: 1000n }])
	await snapshot()
	state().numerator = 2n
	await snapshot(BASE, false)
	expect(latest(A).yieldEarned).toBe(0n)
	await snapshot()
	expect(latest(A).yieldEarned).toBe(1000n)
	expect(rows("VaultPositionSnapshot")).toHaveLength(2)
})

it("replays after a simulated block rollback without retaining orphaned principal", async () => {
	await block([{ kind: "deposit", amount: 1000n }])
	const beforeRecords = new Map([...records].map(([key, row]) => [key, { ...row }]))
	const beforeShares = new Map(state().shares),
		beforeCash = new Map(state().cash)
	await block([{ kind: "share", from: A, to: B, amount: 200n }])
	records.clear()
	beforeRecords.forEach((row, key) => records.set(key, row))
	state().shares = beforeShares
	state().cash = beforeCash
	height--
	await block([{ kind: "share", from: A, to: C, amount: 300n }])
	await snapshot()
	checkOracle()
	expect(position(B)).toBeUndefined()
})

it("retries both transfer sides after a simulated store failure and block rollback", async () => {
	await block([
		{ kind: "deposit", lp: A, amount: 1000n },
		{ kind: "deposit", lp: B, amount: 500n },
	])
	const before = new Map([...records].map(([key, row]) => [key, { ...row }]))
	prepare([{ kind: "share", from: A, to: B, amount: 200n }])
	let fail = true
	;(global as any).store.set.mockImplementation(async (entity: string, id: string, props: any) => {
		if (fail && entity === "VaultLpPosition" && id.endsWith(B)) {
			fail = false
			throw new Error("store write failed")
		}
		records.set(`${entity}:${id}`, { ...props })
	})
	await expect(dispatch()).rejects.toThrow("store write failed")
	// Model the indexer's transaction rollback, then replay the same canonical block.
	records.clear()
	before.forEach((row, key) => records.set(key, row))
	await dispatch()
	await snapshot()
	checkOracle()
	expect(rows("VaultLedgerEvent").filter((row) => row.eventType.startsWith("TRANSFER"))).toHaveLength(2)
})

it.each([new Error("store unavailable"), "store unavailable"])(
	"logs a snapshot store failure and retries on the next invocation: %p",
	async (error) => {
		await block([{ kind: "deposit", amount: 1000n }])
		;(global as any).store.getByFields.mockRejectedValueOnce(error)
		await snapshot()
		expect(latest(A)).toBeUndefined()
		expect((global as any).logger.error).toHaveBeenCalledWith(expect.stringContaining("store unavailable"))
		await snapshot(BASE, false)
		checkOracle()
	},
)

it("preserves the shared wrapper's default behavior for other indexer handlers", async () => {
	const event = {
		get args(): never {
			throw new Error("bad ABI")
		},
	}
	const handler = wrap(async (value: typeof event) => {
		void value.args
	})
	await expect(handler(event)).resolves.toBeUndefined()
	expect((global as any).logger.error).toHaveBeenCalledWith(expect.stringContaining("Error decoding event"))
})

it("registers only configured vault Transfer subscriptions in the manifest template", () => {
	const template = fs.readFileSync(path.resolve(__dirname, "../../../scripts/templates/evm-chain.yaml.hbs"), "utf8")
	const vaultSection = template.slice(
		template.indexOf("{{#each yieldVaults}}"),
		template.indexOf("{{/each}}", template.indexOf("{{#each yieldVaults}}")),
	)
	expect(vaultSection).toContain("address: '{{this.vault}}'")
	for (const handler of ["handleVaultDepositEvent", "handleVaultWithdrawEvent", "handleVaultTransferEvent"])
		expect(vaultSection).toContain(handler)
})

it.each(Array.from({ length: 25 }, (_, i) => i + 1))(
	"matches independent cash-flow accounting across 120 generated actions (seed %i)",
	async (seed) => {
		let random = seed
		const next = () => (random = (Math.imul(random, 1664525) + 1013904223) >>> 0)
		await block([A, B, C].map((lp) => ({ kind: "deposit", lp, amount: 1000000n })))
		for (let round = 0; round < 40; round++) {
			state().numerator = BigInt(900 + (next() % 400))
			state().denominator = 1000n
			const actions: Action[] = []
			for (let i = 0; i < 3; i++) {
				const lp = [A, B, C][next() % 3]
				const to = [A, B, C][next() % 3]
				const kind = (["deposit", "mint", "withdraw", "redeem", "share", "token"] as const)[next() % 6]
				actions.push({ kind, lp, from: lp, to, amount: BigInt((next() % 100) + 1) })
			}
			await block(actions)
			if (round % 7 === 0) await dispatch()
			await snapshot()
			checkOracle()
		}
	},
)
