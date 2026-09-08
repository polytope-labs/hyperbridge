import { describe, it, expect, vi, beforeEach } from "vitest"
import { encodeFunctionData, erc20Abi, maxUint256 } from "viem"
import type { HexString } from "@hyperbridge/sdk"

import { DelegationService } from "@/services/DelegationService"
import { resolvePendingPermit2Approval } from "@/services/paymaster/provider/simplex"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import type { Signer } from "@/services/wallet"

/**
 * Which delegation path `setupDelegation` tries first. A no-permit fee token with no
 * Permit2 allowance cannot be sponsored until a funded approve lands, so when that
 * approval is pending and the EOA can pay for one set-code tx, the direct tx with the
 * approve batched in must go before the bundler; otherwise the bundler goes first.
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

function build(opts: { native: bigint; receiptStatus?: "success" | "reverted" }) {
	const sendTransaction = vi.fn(async () => ("0x" + "ab".repeat(32)) as HexString)
	const publicClient = {
		getCode: async () => "0x",
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

beforeEach(() => {
	trySendSponsored.mockReset()
	vi.mocked(resolvePendingPermit2Approval).mockReset()
})

describe("setupDelegation ordering", () => {
	it("sends the batched delegate+approve first when a Permit2 approval is pending and native covers it", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({ token: USDT, spender: PERMIT2 })
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		expect(trySendSponsored).not.toHaveBeenCalled()
		expect(sendTransaction).toHaveBeenCalledOnce()
		const [tx] = sendTransaction.mock.calls[0] as unknown as [{ to: string; data: string }]
		expect(tx.to).toBe(USDT)
		expect(tx.data).toBe(batchedApprove)
	})

	it("still tries the bundler first when the approval is pending but native is short", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({ token: USDT, spender: PERMIT2 })
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

	it("falls through to the bundler when the batched tx reverts without delegating", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({ token: USDT, spender: PERMIT2 })
		trySendSponsored.mockResolvedValue({ txHash: "0x" + "cd".repeat(32) })
		const { service, sendTransaction } = build({ native: DIRECT_TX_COST, receiptStatus: "reverted" })

		expect(await service.setupDelegation(CHAIN)).toBe(true)

		// The batched attempt went out first, then the sponsored path took over.
		expect(sendTransaction).toHaveBeenCalledOnce()
		expect(trySendSponsored).toHaveBeenCalledOnce()
	})
})
