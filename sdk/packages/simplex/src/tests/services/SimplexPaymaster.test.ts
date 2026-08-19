import { describe, it, expect, vi } from "vitest"
import { encodePacked, type PublicClient, type WalletClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"

import { buildSimplexPaymasterData } from "@/services/paymaster/provider/simplex"
import { VERIFICATION_GAS_LIMIT_APPROVE, THRESHOLD_USD } from "@/services/paymaster/types"
import type { FillerConfigService } from "@/services/FillerConfigService"

/**
 * Approve-mode bootstrap tests (#1070): on chains whose fee token has no
 * EIP-2612 permit, the Simplex paymaster needs a one-time on-chain approve paid
 * in native. An unfunded solver must fail fast with one actionable line —
 * before any estimateGas round-trip — and a solver with an existing allowance
 * must never need native at all.
 */

const USDC = "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d" as HexString // BSC pegged, 18 decimals
const PAYMASTER = "0x0578cFB241215b77442a541325d6A4E6dFE700Ec" as HexString
const SOLVER = "0x13E41CdE1D55880cbe031c69f206C2E9BC3c94C2" as HexString
const CHAIN = "EVM-56"
const DECIMALS = 18

const configService = {
	getChainId: () => 56,
	getUsdcAsset: () => USDC,
	getUsdcDecimals: () => DECIMALS,
	getUsdtAsset: () => "0x" as HexString,
	getUsdtDecimals: () => DECIMALS,
} as unknown as FillerConfigService

const signer = {
	signTypedData: async () => ("0x" + "11".repeat(65)) as HexString,
}

function mockClient(opts: { allowance?: bigint; native?: bigint }): PublicClient {
	return {
		readContract: async ({ functionName }: { functionName: string }) => {
			switch (functionName) {
				case "balanceOf":
					return 5n * 10n ** BigInt(DECIMALS)
				case "allowance":
					return opts.allowance ?? 0n
				default:
					throw new Error(`unexpected readContract: ${functionName}`)
			}
		},
		getBalance: async () => opts.native ?? 0n,
		getGasPrice: async () => 1_000_000_000n,
		waitForTransactionReceipt: async () => ({ status: "success" }),
	} as unknown as PublicClient
}

function mockWalletClient() {
	const writeContract = vi.fn(async () => ("0x" + "ee".repeat(32)) as HexString)
	const walletClient = {
		chain: undefined,
		// A viem WalletClient really does carry `account` — this is not a Signer stub.
		account: { address: SOLVER },
		writeContract,
	} as unknown as WalletClient
	return { walletClient, writeContract }
}

describe("buildSimplexPaymasterData approve-mode native pre-check", () => {
	it("fails fast with an actionable message when the EOA cannot fund the approve", async () => {
		const { walletClient, writeContract } = mockWalletClient()

		await expect(
			buildSimplexPaymasterData(
				mockClient({ native: 0n }),
				walletClient,
				signer,
				SOLVER,
				PAYMASTER,
				CHAIN,
				configService,
				true,
			),
		).rejects.toThrow(/one-time funded approval .* send native dust/)

		// The pre-check must reject before any tx (and its estimateGas) is attempted.
		expect(writeContract).not.toHaveBeenCalled()
	})

	it("sends the capped approve and returns approve-mode data when the EOA is funded", async () => {
		const { walletClient, writeContract } = mockWalletClient()

		const pm = await buildSimplexPaymasterData(
			mockClient({ native: 10n ** 18n }),
			walletClient,
			signer,
			SOLVER,
			PAYMASTER,
			CHAIN,
			configService,
			true,
		)

		expect(writeContract).toHaveBeenCalledOnce()
		expect(pm?.paymasterData).toBe(encodePacked(["uint8", "address"], [1, USDC]))
		expect(pm?.paymasterVerificationGasLimit).toBe(VERIFICATION_GAS_LIMIT_APPROVE)
	})

	it("needs no native once the allowance is in place (steady state)", async () => {
		const { walletClient, writeContract } = mockWalletClient()

		const pm = await buildSimplexPaymasterData(
			mockClient({ allowance: THRESHOLD_USD * 10n ** BigInt(DECIMALS), native: 0n }),
			walletClient,
			signer,
			SOLVER,
			PAYMASTER,
			CHAIN,
			configService,
			true,
		)

		expect(writeContract).not.toHaveBeenCalled()
		expect(pm?.paymasterData).toBe(encodePacked(["uint8", "address"], [1, USDC]))
	})
})
