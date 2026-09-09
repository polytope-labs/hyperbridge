import { describe, it, expect, vi, beforeEach } from "vitest"
import { encodeFunctionData, erc20Abi, maxUint256 } from "viem"
import { encodeERC7821ExecuteBatch, type HexString } from "@hyperbridge/sdk"

import { DelegationService } from "@/services/DelegationService"
import { resolvePendingPermit2Approval } from "@/services/paymaster/provider/simplex"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import type { Signer } from "@/services/wallet"

/**
 * Which delegation path `setupDelegation` tries first, and what the sponsored op carries.
 *
 * A pending Permit2 approval on a token with no EIP-2612 permit can only ride a native
 * set-code tx, so that goes first when the EOA covers it. A permit-capable token instead
 * bootstraps through the bundler — the op pays for itself with a permit and installs the
 * approval in its own callData — which is preferred even when native is available,
 * because it spends stablecoins rather than native. With nothing pending, the bundler
 * goes first and the op is a plain no-op.
 */

const { trySendSponsored } = vi.hoisted(() => ({ trySendSponsored: vi.fn() }))

vi.mock("@/services/paymaster", () => ({ hasPaymaster: () => true }))
vi.mock("@/services/paymaster/provider/simplex", () => ({ resolvePendingPermit2Approval: vi.fn() }))
vi.mock("@/services/UserOpSender", () => ({
	UserOpSender: class {
		canSponsor() {
			return true
		}
		trySendSponsored = trySendSponsored
	},
}))

const CHAIN = "EVM-56"
const SOLVER = "0x13E41CdE1D55880cbe031c69f206C2E9BC3c94C2" as HexString
const SOLVER_ACCOUNT = "0x00000000000000000000000000000000000000cc" as HexString
const PAYMASTER = "0x00000000000000000000000000000000000000aa" as HexString
const USDT = "0x55d398326f99059fF775485246999027B3197955" as HexString
const USDC = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831" as HexString
const PERMIT2 = "0x000000000022D473030F116dDEE9F6B43aC78BA3" as HexString
const GAS_PRICE = 1_000_000_000n
const DIRECT_TX_COST = 650_000n * GAS_PRICE

const configService = {
	getSolverAccountContractAddress: () => SOLVER_ACCOUNT,
	getChainId: () => 56,
	getSimplexPaymasterAddress: () => PAYMASTER,
	loggers: undefined,
} as unknown as FillerConfigService

const signer = {
	address: SOLVER,
	mode: "test",
	signAuthorization: async () => ({ r: "0x01" as HexString, s: "0x02" as HexString, yParity: 0 }),
} as unknown as Signer

function build(opts: { native: bigint; receiptStatus?: "success" | "reverted"; delegatedTo?: HexString }) {
	const sendTransaction = vi.fn(async () => ("0x" + "ab".repeat(32)) as HexString)
	const publicClient = {
		// A delegated EOA carries the EIP-7702 indicator followed by the delegate address.
		getCode: async () => (opts.delegatedTo ? `0xef0100${opts.delegatedTo.slice(2)}` : "0x"),
		getBalance: async () => opts.native,
		getGasPrice: async () => GAS_PRICE,
		getTransactionCount: async () => 0,
		waitForTransactionReceipt: async () => ({ status: opts.receiptStatus ?? "success", blockNumber: 1n }),
	}
	const clientManager = {
		loggers: undefined,
		getPublicClient: () => publicClient,
		getWalletClient: () => ({ chain: undefined, sendTransaction }),
	} as unknown as ChainClientManager
	return { service: new DelegationService(clientManager, configService, signer), sendTransaction }
}

const batchedApprove = encodeFunctionData({ abi: erc20Abi, functionName: "approve", args: [PERMIT2, maxUint256] })

/** The exact ERC-7821 payload the bootstrap op must carry — every byte, not just the addresses. */
const bootstrapCallData = encodeERC7821ExecuteBatch([
	{
		target: USDC,
		value: 0n,
		data: encodeFunctionData({ abi: erc20Abi, functionName: "approve", args: [PERMIT2, maxUint256] }),
	},
])

beforeEach(() => {
	trySendSponsored.mockReset()
	vi.mocked(resolvePendingPermit2Approval).mockReset()
})

describe("setupDelegation ordering", () => {
	it("sends the batched delegate+approve first when a Permit2 approval is pending and native covers it", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDT,
			spender: PERMIT2,
			permitCapable: false,
		})
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(trySendSponsored).not.toHaveBeenCalled()
		expect(sendTransaction).toHaveBeenCalledOnce()
		const [tx] = sendTransaction.mock.calls[0] as unknown as [{ to: string; data: string }]
		expect(tx.to).toBe(USDT)
		expect(tx.data).toBe(batchedApprove)
	})

	it("still tries the bundler first when the approval is pending but native is short", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDT,
			spender: PERMIT2,
			permitCapable: false,
		})
		trySendSponsored.mockResolvedValue({ txHash: "0x" + "cd".repeat(32) })
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST - 1n })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(trySendSponsored).toHaveBeenCalledOnce()
		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("tries the bundler first when nothing is pending", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		trySendSponsored.mockResolvedValue({ txHash: "0x" + "cd".repeat(32) })
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(trySendSponsored).toHaveBeenCalledOnce()
		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("bootstraps a permit-capable token through the bundler even when native covers a direct tx", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDC,
			spender: PERMIT2,
			permitCapable: true,
		})
		trySendSponsored.mockResolvedValue({ txHash: "0x" + "cd".repeat(32) })
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		// No native spent: the permit pays for the op that installs the allowance.
		expect(sendTransaction).not.toHaveBeenCalled()
		expect(trySendSponsored).toHaveBeenCalledOnce()

		const [req] = trySendSponsored.mock.calls[0] as unknown as [{ callData: HexString; permitBootstrap: boolean }]
		expect(req.permitBootstrap).toBe(true)
		// Exact bytes, not a substring match: both addresses appear in the encoding however the
		// call is mis-assembled, so `toContain` would pass on a wrong approve amount, a wrong
		// target, or a missing ERC-7821 wrapper. A 1-wei allowance in particular would strand
		// the chain forever — resolvePendingPermit2Approval refuses any non-zero allowance.
		expect(req.callData).toBe(bootstrapCallData)
	})

	it("sends a plain no-op op, with no permit bootstrap, once the allowance is in place", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		trySendSponsored.mockResolvedValue({ txHash: "0x" + "cd".repeat(32) })
		const { service } = build({ native: DIRECT_TX_COST })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		const [req] = trySendSponsored.mock.calls[0] as unknown as [{ callData: HexString; permitBootstrap: boolean }]
		expect(req.callData).toBe("0x")
		expect(req.permitBootstrap).toBe(false)
	})

	it("does not ask for a permit bootstrap on a no-permit token", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDT,
			spender: PERMIT2,
			permitCapable: false,
		})
		trySendSponsored.mockResolvedValue({ txHash: "0x" + "cd".repeat(32) })
		const { service } = build({ native: DIRECT_TX_COST - 1n })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		const [req] = trySendSponsored.mock.calls[0] as unknown as [{ callData: HexString; permitBootstrap: boolean }]
		expect(req.callData).toBe("0x")
		expect(req.permitBootstrap).toBe(false)
	})

	it("installs the Permit2 allowance on an already-delegated account instead of returning early", async () => {
		// The regression this guards: an account delegated by a release that charged EIP-2612
		// permits has NO Permit2 allowance, and setupDelegation used to return the moment
		// isDelegated held — stranding every upgraded solver on a native-funded approve.
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDC,
			spender: PERMIT2,
			permitCapable: true,
		})
		trySendSponsored.mockResolvedValue({ txHash: "0x" + "cd".repeat(32) })
		const { service, sendTransaction } = build({ native: 0n, delegatedTo: SOLVER_ACCOUNT })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(sendTransaction).not.toHaveBeenCalled()
		expect(trySendSponsored).toHaveBeenCalledOnce()
		const [req] = trySendSponsored.mock.calls[0] as unknown as [
			{ callData: HexString; permitBootstrap: boolean; eip7702Auth?: unknown },
		]
		expect(req.permitBootstrap).toBe(true)
		expect(req.callData).toBe(bootstrapCallData)
		// Already delegated, so the op carries no authorization — only the approve.
		expect(req.eip7702Auth).toBeUndefined()
	})

	it("sends nothing on an already-delegated account whose allowance is already in place", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(null)
		const { service, sendTransaction } = build({ native: 0n, delegatedTo: SOLVER_ACCOUNT })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(trySendSponsored).not.toHaveBeenCalled()
		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("leaves a no-permit token on an already-delegated account to the funded approve", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDT,
			spender: PERMIT2,
			permitCapable: false,
		})
		const { service, sendTransaction } = build({ native: 0n, delegatedTo: SOLVER_ACCOUNT })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		// Nothing to pay the op with, so no op — the first sponsored op sends the approve.
		expect(trySendSponsored).not.toHaveBeenCalled()
		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("still reports the account delegated when the allowance op fails", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDC,
			spender: PERMIT2,
			permitCapable: true,
		})
		trySendSponsored.mockRejectedValue(new Error("bundler down"))
		const { service } = build({ native: 0n, delegatedTo: SOLVER_ACCOUNT })

		// Best-effort: a failed bootstrap must not turn a delegated account into a setup failure.
		expect(await service.setupDelegation(CHAIN)).toBe(true)
	})

	it("falls through to the bundler when the batched tx reverts without delegating", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDT,
			spender: PERMIT2,
			permitCapable: false,
		})
		trySendSponsored.mockResolvedValue({ txHash: "0x" + "cd".repeat(32) })
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST, receiptStatus: "reverted" })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		// The batched attempt went out first, then the sponsored path took over.
		expect(sendTransaction).toHaveBeenCalledOnce()
		expect(trySendSponsored).toHaveBeenCalledOnce()
	})
})
