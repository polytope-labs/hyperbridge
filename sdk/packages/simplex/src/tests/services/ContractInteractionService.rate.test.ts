import { describe, it, expect, vi } from "vitest"
import { decodeERC7821ExecuteBatch, decodeFillOrder, type Order, type HexString } from "@hyperbridge/sdk"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import { CacheService } from "@/services/CacheService"

const gateway = "0x1111111111111111111111111111111111111111"
const solver = "0x2222222222222222222222222222222222222222"
const token = `0x${"0".repeat(24)}${"33".repeat(20)}` as HexString
const order = {
	id: `0x${"55".repeat(32)}`,
	user: token,
	source: "EVM-1",
	destination: "EVM-1",
	deadline: 100n,
	nonce: 0n,
	fees: 0n,
	session: solver,
	predispatch: { assets: [], call: "0x" },
	inputs: [{ token, amount: 1000n }],
	output: { beneficiary: token, assets: [{ token, amount: 1000n }], call: "0x" },
} as Order
function makeService(supported = true, cache = new CacheService()) {
	const client = {
		chain: { id: 1 },
		readContract: async ({ functionName }: { functionName: string }) =>
			functionName === "supportsRateFills" ? supported : 0n,
	}
	return new ContractInteractionService(
		{ getPublicClient: () => client } as never,
		{
			getIntentGatewayAddress: () => gateway,
			getConfiguredChainIds: () => [],
			getEntryPointAddress: () => gateway,
			getGasFeeBumpConfig: () => undefined,
		} as never,
		{ address: solver } as never,
		cache,
	)
}
describe("rate fill batches", () => {
	it("encodes the signed take and approves the signed output", async () => {
		const service = makeService()
		const outputs = [{ token, amount: 440n }],
			inputs = [{ token, amount: 400n }]
		const calldata = await service.buildApprovalAndFillCalldata(
			order,
			outputs,
			{ relayerFee: 0n, nativeDispatchFee: 0n, validUntil: 99n, outputs },
			0n,
			inputs,
		)
		const calls = decodeERC7821ExecuteBatch(calldata)!
		const decoded = decodeFillOrder(calls[calls.length - 1].data)
		expect(decoded?.method).toBe("fillOrderAtRate")
		expect(decoded?.inputs).toEqual(inputs)
		expect(decoded?.options.outputs).toEqual(outputs)
	})
	it("refuses to publish a rate bid from an unsupported account or gateway", async () => {
		const outputs = [{ token, amount: 440n }]
		await expect(
			makeService(false).buildApprovalAndFillCalldata(
				order,
				outputs,
				{ relayerFee: 0n, nativeDispatchFee: 0n, validUntil: 99n, outputs },
				0n,
				[{ token, amount: 400n }],
			),
		).rejects.toThrow(/rate/i)
	})
	it("keeps rate inputs with their output quote and invalidates old gas estimates", () => {
		const cache = new CacheService()
		cache.setFillerOutputs(order.id!, [{ token, amount: 440n }], [{ token, amount: 400n }])
		expect(cache.getFillerInputs(order.id!)).toEqual([{ token, amount: 400n }])
		cache.setGasEstimate(order.id!, 1n, 0n, 0n, 1n, 1n, 1n, 1n, 1n, 0n, 0n)
		cache.setFillerOutputs(order.id!, [{ token, amount: 1000n }])
		expect(cache.getFillerInputs(order.id!)).toEqual([])
		expect(cache.getGasEstimate(order.id!)).toBeNull()
	})
})

describe("rate quote cache races", () => {
	it.each(["expire", "refresh"])("rejects a quote cache %s while preparing its validity bound", async (change) => {
		const cache = new CacheService()
		cache.setFillerOutputs(order.id!, [{ token, amount: 440n }], [{ token, amount: 400n }])
		cache.setGasEstimate(order.id!, 1n, 0n, 0n, 1n, 1n, 1n, 1n, 1n, 0n, 0n)
		const service = makeService(true, cache)
		const now = Date.now()
		const clock = vi.spyOn(Date, "now").mockReturnValue(now)
		vi.spyOn(service as any, "getIntentGateway").mockResolvedValue({})
		vi.spyOn(service as any, "bidValidUntilBlock").mockImplementation(async () => {
			if (change === "expire") clock.mockReturnValue(now + 60_001)
			else cache.setFillerOutputs(order.id!, [{ token, amount: 330n }], [{ token, amount: 300n }])
			return 99n
		})
		const encode = vi
			.spyOn(service, "buildApprovalAndFillCalldata")
			.mockRejectedValue(new Error("unexpected encoding"))
		try {
			await expect(service.prepareBidUserOp(order, gateway, solver)).rejects.toThrow(/quote.*(expired|changed)/i)
			expect(encode).not.toHaveBeenCalled()
		} finally {
			vi.restoreAllMocks()
		}
	})
	it("does not cache a gas estimate for a replaced rate quote", async () => {
		const cache = new CacheService()
		const inputs = [{ token, amount: 400n }],
			outputs = [{ token, amount: 440n }]
		cache.setFillerOutputs(order.id!, outputs, inputs)
		const service = makeService(true, cache)
		const estimate = vi.fn(async (params) => {
			expect(params.inputs).toEqual(inputs)
			expect(params.outputs).toEqual(outputs)
			cache.setFillerOutputs(order.id!, [{ token, amount: 330n }], [{ token, amount: 300n }])
			return {
				totalGasInFeeToken: 1n,
				relayerFeeInSourceFeeToken: 0n,
				fillOptions: { relayerFee: 0n },
				callGasLimit: 1n,
			}
		})
		vi.spyOn(service as any, "getIntentGateway").mockResolvedValue({ estimateFillOrder: estimate })
		try {
			await expect(service.estimateGasFillPost(order)).rejects.toThrow(/quote.*changed/i)
			expect(estimate).toHaveBeenCalledOnce()
			expect(cache.getGasEstimate(order.id!)).toBeNull()
		} finally {
			vi.restoreAllMocks()
		}
	})
})
