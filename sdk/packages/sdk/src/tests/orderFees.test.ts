import { IntentGateway } from "@/protocols/intents/IntentGateway"
import type { FillOrderEstimate, HexString, Order } from "@/types"
import { describe, expect, it, vi } from "vitest"

const TOKEN = "0x0000000000000000000000000000000000000001" as HexString

function makeOrder(source: string, destination: string): Order {
	return {
		user: TOKEN,
		source,
		destination,
		deadline: 100n,
		nonce: 0n,
		fees: 0n,
		session: TOKEN,
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: TOKEN, amount: 1_000n }],
		output: { beneficiary: TOKEN, assets: [{ token: TOKEN, amount: 990n }], call: "0x" },
	}
}

function makeEstimate(): FillOrderEstimate {
	return {
		fillOptions: { relayerFee: 0n, nativeDispatchFee: 0n, validUntil: 0n, outputs: [] },
		callGasLimit: 1n,
		verificationGasLimit: 1n,
		preVerificationGas: 1n,
		paymasterVerificationGasLimit: 0n,
		paymasterPostOpGasLimit: 0n,
		maxFeePerGas: 1n,
		maxPriorityFeePerGas: 1n,
		totalGasCostWei: 1_000n,
		totalGasInFeeToken: 100n,
		relayerFeeInSourceFeeToken: 20n,
	}
}

function makeGateway(sourceStateMachineId: string, destinationStateMachineId: string) {
	const estimateFillOrder = vi.fn().mockResolvedValue(makeEstimate())
	return {
		gateway: {
			gasEstimator: { estimateFillOrder },
			source: {
				config: { stateMachineId: sourceStateMachineId },
				getFeeTokenWithDecimals: vi.fn().mockResolvedValue({ address: TOKEN, decimals: 6 }),
			},
			dest: { config: { stateMachineId: destinationStateMachineId } },
		},
		estimateFillOrder,
	}
}

describe("IntentGateway order-fee gas-price policy", () => {
	it.each([
		["EVM-1", "EVM-42161", 50n],
		["EVM-42161", "EVM-1", 10n],
		["EVM-42161", "EVM-8453", 10n],
		["EVM-1", "EVM-1", 0n],
	])(
		"prices source %s and destination %s with the expected source-chain headroom",
		async (source, destination, expectedBump) => {
			const { gateway, estimateFillOrder } = makeGateway(source, destination)

			await IntentGateway.prototype.quoteOrderFees.call(gateway as never, makeOrder(source, destination))

			expect(estimateFillOrder.mock.calls[0][1]).toEqual({
				orderFeeGasPriceBumpPercent: expectedBump,
			})
		},
	)
})
