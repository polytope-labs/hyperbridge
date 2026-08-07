import { IntentGateway } from "@/protocols/intents/IntentGateway"
import { bumpGasPrice, convertGasToFeeToken } from "@/protocols/intents/utils"
import type { FillOrderEstimate, HexString, Order } from "@/types"
import { describe, expect, it, vi } from "vitest"

const TOKEN = "0x0000000000000000000000000000000000000001" as HexString
const WETH = "0x0000000000000000000000000000000000000002" as HexString

function makeOrder(overrides: Partial<Order> = {}): Order {
	return {
		user: TOKEN,
		source: "EVM-1",
		destination: "EVM-42161",
		deadline: 100n,
		nonce: 0n,
		fees: 0n,
		session: TOKEN,
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: TOKEN, amount: 1_000n }],
		output: { beneficiary: TOKEN, assets: [{ token: TOKEN, amount: 990n }], call: "0x" },
		...overrides,
	}
}

function makeEstimate(): FillOrderEstimate {
	return {
		fillOptions: { relayerFee: 0n, nativeDispatchFee: 0n, outputs: [] },
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

function makeGateway(stateMachineId: string, estimate: FillOrderEstimate) {
	const estimateFillOrder = vi.fn().mockResolvedValue(estimate)
	const gateway = {
		gasEstimator: { estimateFillOrder },
		source: {
			config: { stateMachineId: "EVM-1" },
			getFeeTokenWithDecimals: vi.fn().mockResolvedValue({ address: TOKEN, decimals: 6 }),
		},
		dest: { config: { stateMachineId } },
	}

	return { gateway, estimateFillOrder }
}

describe("SDK order-fee gas-price policy", () => {
	it("applies the requested percentage without understating it through integer truncation", () => {
		expect(bumpGasPrice(100n, 10n)).toBe(110n)
		expect(bumpGasPrice(101n, 10n)).toBe(112n)
		expect(bumpGasPrice(101n, 0n)).toBe(101n)
		expect(() => bumpGasPrice(100n, -1n)).toThrow("Gas price bump percent cannot be negative")
	})

	it("bumps the gas price before converting gas into the fee token", async () => {
		const findBestProtocolWithAmountIn = vi.fn().mockResolvedValue({ amountOut: 220n })
		const client = {}
		const chain = {
			client,
			configService: {
				getWrappedNativeAssetWithDecimals: () => ({ asset: WETH, decimals: 18 }),
			},
			getFeeTokenWithDecimals: vi.fn().mockResolvedValue({ address: TOKEN, decimals: 6 }),
		}
		const ctx = {
			source: chain,
			dest: chain,
			feeTokenCache: new Map(),
			swap: { findBestProtocolWithAmountIn },
		}

		const amount = await convertGasToFeeToken(ctx as never, 2n, "dest", "EVM-1", 100n, 10n)

		expect(amount).toBe(220n)
		expect(findBestProtocolWithAmountIn).toHaveBeenCalledWith(client, WETH, TOKEN, 220n, "EVM-1", {
			selectedProtocol: "v2",
		})
	})

	it("enables the 10% bump for cross-chain SDK quotes but leaves direct estimates unchanged", async () => {
		const estimate = makeEstimate()
		const { gateway, estimateFillOrder } = makeGateway("EVM-42161", estimate)
		const order = makeOrder()

		const quote = await IntentGateway.prototype.quoteOrderFees.call(gateway as never, order)

		expect(estimateFillOrder.mock.calls[0][1]).toEqual({ orderFeeGasPriceBumpPercent: 10n })
		expect(quote).toEqual({ fees: 126n, nativeValue: 1_020n, feeToken: TOKEN, estimate })

		estimateFillOrder.mockClear()
		const params = { order }
		await IntentGateway.prototype.estimateFillOrder.call(gateway as never, params)

		expect(estimateFillOrder.mock.calls).toEqual([[params]])
	})

	it("does not bump gas prices for same-chain SDK quotes", async () => {
		const estimate = { ...makeEstimate(), relayerFeeInSourceFeeToken: 0n }
		const { gateway, estimateFillOrder } = makeGateway("EVM-1", estimate)

		const quote = await IntentGateway.prototype.quoteOrderFees.call(
			gateway as never,
			makeOrder({ destination: "EVM-1" }),
		)

		expect(estimateFillOrder.mock.calls[0][1]).toEqual({ orderFeeGasPriceBumpPercent: 0n })
		expect(quote.fees).toBe(200n)
	})
})
