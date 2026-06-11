import { describe, it, expect, vi } from "vitest"
import { decodeFunctionData, parseUnits } from "viem"
import { Erc4626FundingPlanner } from "@/funding/erc4626/Erc4626FundingPlanner"
import { ERC4626_ABI } from "@/config/abis/Erc4626"
import type { Erc4626OutputFundingConfig } from "@/funding/types"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { HexString } from "@hyperbridge/sdk"

const CHAIN = "EVM-8453"
const SOLVER = `0x${"d".repeat(40)}` as HexString
const USDC = `0x${"a".repeat(40)}` as HexString
const VAULT_USDC = `0x${"b".repeat(40)}` as HexString
const USDT = `0x${"e".repeat(40)}` as HexString
const VAULT_USDT = `0x${"f".repeat(40)}` as HexString

const assetOf: Record<string, HexString> = { [VAULT_USDC.toLowerCase()]: USDC, [VAULT_USDT.toLowerCase()]: USDT }

/** Live balances the mocked client returns. Mutable so tests can vary scenarios. */
interface Balances {
	/** Solver's vault position in asset terms (shares are mocked 1:1). */
	positionAssets: bigint
	/** Vault's withdraw cap — already min(position, vault liquidity). */
	maxWithdrawable: bigint
}

/**
 * Planner for withdraw-planning tests, backed by a mocked public client.
 * `readContract` dispatches on functionName/address to return controlled
 * vault data and balances.
 */
function makeWithdrawPlanner(balances: Balances) {
	const fakeClient = {
		async readContract({
			address,
			functionName,
			args,
		}: {
			address: HexString
			functionName: string
			args?: readonly unknown[]
		}) {
			if (functionName === "asset") return USDC
			if (functionName === "decimals") return 6
			if (functionName === "maxWithdraw") return balances.maxWithdrawable
			// Shares are 1:1 with assets in this mock.
			if (functionName === "previewRedeem") return args?.[0] as bigint
			if (functionName === "balanceOf") {
				const who = (args?.[0] as string)?.toLowerCase()
				// vault.balanceOf(solver) → solver's share balance
				if (address.toLowerCase() === VAULT_USDC.toLowerCase() && who === SOLVER.toLowerCase()) {
					return balances.positionAssets
				}
				return 0n
			}
			throw new Error(`unexpected readContract: ${functionName}`)
		},
	}

	const clientManager = { getPublicClient: () => fakeClient } as unknown as ChainClientManager
	const config: Erc4626OutputFundingConfig = { vaultsByChain: { [CHAIN]: [{ vault: VAULT_USDC }] } }
	return new Erc4626FundingPlanner(clientManager, config)
}

/**
 * Planner for sweep/redeem tests, backed by mocked clients. `wallet` maps an
 * underlying asset to the solver's wallet balance (drives the sweep); `shares`
 * maps a vault to the solver's share balance (drives redeemAll). Unspecified
 * reads return large constants so hydrate succeeds.
 */
function makeSweepPlanner(
	wallet: Record<string, bigint>,
	vaults: Erc4626OutputFundingConfig["vaultsByChain"][string],
	shares: Record<string, bigint> = {},
) {
	const sendTransaction = vi.fn(async (_tx: { to: HexString; data: HexString; value: bigint }) => "0xtx" as HexString)

	const publicClient = {
		async readContract({
			address,
			functionName,
			args,
		}: {
			address: HexString
			functionName: string
			args?: readonly unknown[]
		}) {
			if (functionName === "asset") return assetOf[address.toLowerCase()]
			if (functionName === "decimals") return 6
			if (functionName === "maxWithdraw") return 10_000_000_000n
			// Shares are 1:1 with assets in this mock.
			if (functionName === "previewRedeem") return args?.[0] as bigint
			if (functionName === "balanceOf") {
				const who = (args?.[0] as string)?.toLowerCase()
				const addr = address.toLowerCase()
				if (who === SOLVER.toLowerCase()) {
					// asset.balanceOf(solver) → wallet balance (what the sweep reads)
					if (wallet[addr] !== undefined) return wallet[addr]
					// vault.balanceOf(solver) → share balance (what redeemAll reads)
					if (shares[addr] !== undefined) return shares[addr]
				}
				return 10_000_000_000n // large constant for hydrate reads
			}
			throw new Error(`unexpected readContract: ${functionName}`)
		},
		waitForTransactionReceipt: async () => ({ status: "success" }),
	}

	const walletClient = { chain: { id: 8453 }, sendTransaction }
	const clientManager = {
		getPublicClient: () => publicClient,
		getWalletClient: () => walletClient,
	} as unknown as ChainClientManager

	const planner = new Erc4626FundingPlanner(clientManager, { vaultsByChain: { [CHAIN]: vaults } })
	return { planner, sendTransaction }
}

const u = (n: string) => parseUnits(n, 6)

describe("Erc4626FundingPlanner", () => {
	it("plans a withdraw call for the full deficit when liquidity is ample", async () => {
		const planner = makeWithdrawPlanner({ positionAssets: 1_000_000n, maxWithdrawable: 1_000_000n })
		await planner.initialise(SOLVER)

		const need = 250_000n
		const { calls, credited } = await planner.planWithdrawalForToken(CHAIN, SOLVER, USDC.toLowerCase(), need)

		expect(credited).toBe(need)
		expect(calls).toHaveLength(1)
		expect(calls[0].target.toLowerCase()).toBe(VAULT_USDC.toLowerCase())
		expect(calls[0].value).toBe(0n)

		const decoded = decodeFunctionData({ abi: ERC4626_ABI, data: calls[0].data })
		expect(decoded.functionName).toBe("withdraw")
		expect(decoded.args[0]).toBe(need)
		expect((decoded.args[1] as string).toLowerCase()).toBe(SOLVER.toLowerCase())
		expect((decoded.args[2] as string).toLowerCase()).toBe(SOLVER.toLowerCase())
	})

	it("caps the withdraw at the vault's maxWithdraw", async () => {
		// Solver's position is 1M but the vault only honours 300k right now.
		const planner = makeWithdrawPlanner({ positionAssets: 1_000_000n, maxWithdrawable: 300_000n })
		await planner.initialise(SOLVER)

		const { calls, credited } = await planner.planWithdrawalForToken(CHAIN, SOLVER, USDC.toLowerCase(), 500_000n)

		expect(credited).toBe(300_000n)
		const decoded = decodeFunctionData({ abi: ERC4626_ABI, data: calls[0].data })
		expect(decoded.args[0]).toBe(300_000n)
	})

	it("returns a no-op for tokens not backed by a configured vault", async () => {
		const planner = makeWithdrawPlanner({ positionAssets: 1_000_000n, maxWithdrawable: 1_000_000n })
		await planner.initialise(SOLVER)

		const res = await planner.planWithdrawalForToken(CHAIN, SOLVER, USDT.toLowerCase(), 100_000n)
		expect(res.calls).toHaveLength(0)
		expect(res.credited).toBe(0n)
	})

	it("does not over-source across concurrent plans (consume accounting)", async () => {
		const planner = makeWithdrawPlanner({ positionAssets: 400_000n, maxWithdrawable: 400_000n })
		await planner.initialise(SOLVER)

		const first = await planner.planWithdrawalForToken(CHAIN, SOLVER, USDC.toLowerCase(), 300_000n)
		const second = await planner.planWithdrawalForToken(CHAIN, SOLVER, USDC.toLowerCase(), 300_000n)

		expect(first.credited).toBe(300_000n)
		// Only 100k remains unreserved after the first plan; refresh sees the same
		// on-chain position (withdrawal not yet executed) so consume must hold it back.
		expect(second.credited).toBe(100_000n)
	})

	it("returns null for exotic-token pricing (stable-only venue)", async () => {
		const planner = makeWithdrawPlanner({ positionAssets: 1n, maxWithdrawable: 1n })
		await planner.initialise(SOLVER)
		expect(await planner.getExoticTokenPrice(CHAIN, USDC)).toBeNull()
	})

	it("rejects invalid vault config", () => {
		expect(() => Erc4626FundingPlanner.validateConfig([{ chain: "", vault: VAULT_USDC }])).toThrow()
		expect(() => Erc4626FundingPlanner.validateConfig([{ chain: CHAIN, vault: "" }])).toThrow()
		expect(() => Erc4626FundingPlanner.validateConfig([{ chain: CHAIN, vault: VAULT_USDC }])).not.toThrow()
	})
})

describe("Erc4626FundingPlanner.sweepExcessToVault", () => {
	it("deposits the excess above threshold", async () => {
		const { planner, sendTransaction } = makeSweepPlanner({ [USDC.toLowerCase()]: u("5000") }, [
			{ vault: VAULT_USDC, threshold: "3000" },
		])
		await planner.initialise(SOLVER)
		await planner.sweepExcessToVault(CHAIN)

		expect(sendTransaction).toHaveBeenCalledOnce()
		expect(sendTransaction.mock.calls[0][0].to.toLowerCase()).toBe(SOLVER.toLowerCase())
	})

	it("does nothing when balance is at or below threshold", async () => {
		const { planner, sendTransaction } = makeSweepPlanner({ [USDC.toLowerCase()]: u("3000") }, [
			{ vault: VAULT_USDC, threshold: "3000" },
		])
		await planner.initialise(SOLVER)
		await planner.sweepExcessToVault(CHAIN)

		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("skips dust below minSweep", async () => {
		// excess = 5 USDC, below the default 10 minSweep.
		const { planner, sendTransaction } = makeSweepPlanner({ [USDC.toLowerCase()]: u("3005") }, [
			{ vault: VAULT_USDC, threshold: "3000" },
		])
		await planner.initialise(SOLVER)
		await planner.sweepExcessToVault(CHAIN)

		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("does not sweep a vault with no threshold (withdraw-only)", async () => {
		const { planner, sendTransaction } = makeSweepPlanner({ [USDC.toLowerCase()]: u("9999") }, [
			{ vault: VAULT_USDC },
		])
		await planner.initialise(SOLVER)
		await planner.sweepExcessToVault(CHAIN)

		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("batches both vaults' excess into a single transaction", async () => {
		const { planner, sendTransaction } = makeSweepPlanner(
			{ [USDC.toLowerCase()]: u("5000"), [USDT.toLowerCase()]: u("4000") },
			[
				{ vault: VAULT_USDC, threshold: "3000" },
				{ vault: VAULT_USDT, threshold: "3000" },
			],
		)
		await planner.initialise(SOLVER)
		await planner.sweepExcessToVault(CHAIN)

		expect(sendTransaction).toHaveBeenCalledOnce()
	})
})

describe("Erc4626FundingPlanner.redeemAll", () => {
	it("redeems the full share balance from every vault", async () => {
		const { planner, sendTransaction } = makeSweepPlanner(
			{ [USDC.toLowerCase()]: 0n },
			[{ vault: VAULT_USDC, threshold: "3000" }],
			{ [VAULT_USDC.toLowerCase()]: u("5000") },
		)
		await planner.initialise(SOLVER)
		await planner.redeemAll()

		expect(sendTransaction).toHaveBeenCalledOnce()
		expect(sendTransaction.mock.calls[0][0].to.toLowerCase()).toBe(SOLVER.toLowerCase())
	})

	it("does nothing when no vault holds shares", async () => {
		const { planner, sendTransaction } = makeSweepPlanner(
			{ [USDC.toLowerCase()]: 0n },
			[{ vault: VAULT_USDC, threshold: "3000" }],
			{ [VAULT_USDC.toLowerCase()]: 0n },
		)
		await planner.initialise(SOLVER)
		await planner.redeemAll()

		expect(sendTransaction).not.toHaveBeenCalled()
	})
})
