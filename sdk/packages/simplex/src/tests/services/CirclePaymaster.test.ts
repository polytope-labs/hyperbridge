import { describe, it, expect } from "vitest"
import type { PublicClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"

import { buildCirclePaymasterData } from "@/services/paymaster/provider/circle"
import { VERIFICATION_GAS_LIMIT_CIRCLE } from "@/services/paymaster/types"
import type { FillerConfigService } from "@/services/FillerConfigService"

/**
 * Regression test for the Polygon AA33 delegation failure: a re-delegation passes a
 * tightened paymasterVerificationGasLimit (110k) on the assumption that the Circle
 * paymaster allowance is already in place. When the allowance is depleted (or never
 * set), the paymasterData carries a full EIP-2612 permit that executes during
 * validation and needs ~113k on its own — so the tightened override must only be
 * honored on the allowance-reuse branch, never on the permit branch.
 */

const USDC = "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359" as HexString
const PAYMASTER = "0x0578cFB241215b77442a541325d6A4E6dFE700Ec" as HexString
const SOLVER = "0x13E41CdE1D55880cbe031c69f206C2E9BC3c94C2" as HexString
const CHAIN = "EVM-137"
const TIGHTENED_LIMIT = 110_000n

const configService = {
	getUsdcAsset: () => USDC,
	getUsdcDecimals: () => 6,
	getChainId: () => 137,
} as unknown as FillerConfigService

function mockClient(allowance: bigint): PublicClient {
	return {
		readContract: async ({ functionName }: { functionName: string }) => {
			switch (functionName) {
				case "allowance":
					return allowance
				case "name":
					return "USD Coin"
				case "version":
					return "2"
				case "nonces":
					return 0n
				default:
					throw new Error(`unexpected readContract: ${functionName}`)
			}
		},
	} as unknown as PublicClient
}

const signer = {
	signTypedData: async () => ("0x" + "11".repeat(65)) as HexString,
}

describe("buildCirclePaymasterData verification gas limit", () => {
	it("honors a tightened limit when the existing allowance covers the threshold", async () => {
		const pm = await buildCirclePaymasterData(
			mockClient(5_000_000n),
			signer,
			SOLVER,
			PAYMASTER,
			CHAIN,
			configService,
			TIGHTENED_LIMIT,
		)

		// Allowance-reuse branch: no permit, single mode byte
		expect(pm.paymasterData).toBe("0x00")
		expect(pm.paymasterVerificationGasLimit).toBe(TIGHTENED_LIMIT)
	})

	it("restores the Circle default when a permit must execute during validation", async () => {
		const pm = await buildCirclePaymasterData(
			mockClient(0n),
			signer,
			SOLVER,
			PAYMASTER,
			CHAIN,
			configService,
			TIGHTENED_LIMIT,
		)

		// Permit branch: mode byte + token + amount + signature
		expect(pm.paymasterData.length).toBeGreaterThan(4)
		expect(pm.paymasterVerificationGasLimit).toBe(VERIFICATION_GAS_LIMIT_CIRCLE)
	})
})
