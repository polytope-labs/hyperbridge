import { describe, expect, it, vi } from "vitest"
import type { EstimateFillOrderParams, HexString, Order } from "@hyperbridge/sdk"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import { privateKeySigner } from "@/services/wallet"

/**
 * The gas estimate a bid is signed from.
 *
 * The SDK can answer a failed bundler estimate with fixed gas limits. A bid signed
 * with those does not execute, so the filler asks for the bundler's own figures and
 * passes on the order when there are none.
 */

const SOLVER_KEY = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" as HexString
const order = { id: "0xorder", source: "EVM-8453", destination: "EVM-8453" } as Order

function service(estimateFillOrder: ReturnType<typeof vi.fn>): ContractInteractionService {
	const clientManager = { getPublicClient: () => ({}) }
	const configService = { loggers: undefined, getGasFeeBumpConfig: () => undefined }
	// biome-ignore lint/suspicious/noExplicitAny: only the estimate path is exercised
	const contract = new ContractInteractionService(
		clientManager as any,
		configService as any,
		privateKeySigner(SOLVER_KEY),
	)
	// biome-ignore lint/suspicious/noExplicitAny: the gateway helper is stubbed
	;(contract as any).getIntentGateway = async () => ({ estimateFillOrder })
	return contract
}

const ESTIMATE = {
	totalGasInFeeToken: 10n,
	relayerFeeInSourceFeeToken: 0n,
	fillOptions: { relayerFee: 0n },
	callGasLimit: 1_000_000n,
	verificationGasLimit: 100_000n,
	preVerificationGas: 50_000n,
	maxFeePerGas: 1n,
	maxPriorityFeePerGas: 1n,
	totalGasCostWei: 1n,
}

describe("the gas estimate a bid is signed from", () => {
	it("requires the bundler's own estimate", async () => {
		const estimateFillOrder = vi.fn().mockResolvedValue(ESTIMATE)
		const contract = service(estimateFillOrder)

		await contract.estimateGasFillPost(order)

		expect(estimateFillOrder.mock.calls[0][0]).toMatchObject({ order, requireBundlerEstimate: true })
		expect(contract.cacheService.getGasEstimate(order.id!)?.callGasLimit).toBe(1_000_000n)
	})

	it("logs each retry of a transiently failed bundler estimate as a warning", async () => {
		const error = new Error("Bundler error: internal error: rpc provider error")
		const estimateFillOrder = vi.fn(async ({ onBundlerRetry }: EstimateFillOrderParams) => {
			onBundlerRetry?.({ attempt: 1, maxAttempts: 3, delayMs: 250, error })
			return ESTIMATE
		})
		const contract = service(estimateFillOrder)
		// The service's logger is private.
		const warn = vi.spyOn((contract as any).logger, "warn")

		await contract.estimateGasFillPost(order)

		expect(warn).toHaveBeenCalledWith(
			{ orderId: order.id, attempt: 1, maxAttempts: 3, delayMs: 250, err: error },
			"Bundler gas estimate failed transiently, retrying",
		)
	})

	it("leaves nothing to bid with when the bundler estimate fails", async () => {
		const contract = service(vi.fn().mockRejectedValue(new Error("Bundler gas estimation failed: AA23 reverted")))

		await expect(contract.estimateGasFillPost(order)).rejects.toThrow(
			"Bundler gas estimation failed: AA23 reverted",
		)
		expect(contract.cacheService.getGasEstimate(order.id!)).toBeFalsy()
	})
})
