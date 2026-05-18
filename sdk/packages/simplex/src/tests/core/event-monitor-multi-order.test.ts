import { describe, it, expect } from "vitest"
import { encodeFunctionData, stringToHex, type Hex } from "viem"
import { DecodedOrderPlacedLog, HexString, Order } from "@hyperbridge/sdk"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import { reconstructOrdersFromLogs } from "@/core/event-monitor"

const SOURCE = "EVM-1"
const DESTINATION = "EVM-10"

function pad32(addr: Hex): Hex {
	return `0x${addr.slice(2).padStart(64, "0")}` as Hex
}

const USDC_INPUT = pad32("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48") // bytes32-encoded
const USDC_OUTPUT = pad32("0x0b2c639c533813f4aa9d7837caf62653d097ff85")
const USER = pad32("0x1111111111111111111111111111111111111111")
const BENEFICIARY_1 = pad32("0xaaaa000000000000000000000000000000000001")
const BENEFICIARY_2 = pad32("0xbbbb000000000000000000000000000000000002")
const SESSION = "0x0000000000000000000000000000000000000000" as Hex

const TX_HASH = "0xdeadbeef".padEnd(66, "0") as HexString

function makeOrderStruct(args: {
	nonce: bigint
	beneficiary: Hex
	outputAmount: bigint
	predispatchCall: HexString
	outputCall: HexString
}) {
	return {
		user: USER,
		source: stringToHex(SOURCE),
		destination: stringToHex(DESTINATION),
		deadline: 1700000000n,
		nonce: args.nonce,
		fees: 0n,
		session: SESSION,
		predispatch: {
			assets: [],
			call: args.predispatchCall,
		},
		inputs: [{ token: USDC_INPUT, amount: 100_000_000n }],
		output: {
			beneficiary: args.beneficiary,
			assets: [{ token: USDC_OUTPUT, amount: args.outputAmount }],
			call: args.outputCall,
		},
	}
}

const GRAFFITI = `0x${"00".repeat(32)}` as Hex

function encodePlaceOrderCalldata(orderStruct: ReturnType<typeof makeOrderStruct>): HexString {
	return encodeFunctionData({
		abi: INTENT_GATEWAY_V2_ABI,
		functionName: "placeOrder",
		args: [orderStruct as any, GRAFFITI],
	}) as HexString
}

function makeOrderPlacedLog(args: {
	nonce: bigint
	beneficiary: Hex
	outputAmount: bigint
}): DecodedOrderPlacedLog {
	return {
		eventName: "OrderPlaced",
		transactionHash: TX_HASH,
		args: {
			user: USER as HexString,
			source: stringToHex(SOURCE),
			destination: stringToHex(DESTINATION),
			deadline: 1700000000n,
			nonce: args.nonce,
			fees: 0n,
			session: SESSION,
			beneficiary: args.beneficiary as HexString,
			predispatch: [],
			inputs: [{ token: USDC_INPUT as HexString, amount: 100_000_000n }],
			outputs: [{ token: USDC_OUTPUT as HexString, amount: args.outputAmount }],
		},
	} as unknown as DecodedOrderPlacedLog
}

describe("reconstructOrdersFromLogs", () => {
	it("pairs the K-th log in a tx with the K-th placeOrder calldata", async () => {
		const order1 = makeOrderStruct({
			nonce: 1n,
			beneficiary: BENEFICIARY_1,
			outputAmount: 100_000_000n,
			predispatchCall: "0x11",
			outputCall: "0x21",
		})
		const order2 = makeOrderStruct({
			nonce: 2n,
			beneficiary: BENEFICIARY_2,
			outputAmount: 200_000_000n,
			predispatchCall: "0x12",
			outputCall: "0x22",
		})

		const calldataByOccurrence: Record<number, HexString> = {
			0: encodePlaceOrderCalldata(order1),
			1: encodePlaceOrderCalldata(order2),
		}

		const log1 = makeOrderPlacedLog({
			nonce: 1n,
			beneficiary: BENEFICIARY_1,
			outputAmount: 100_000_000n,
		})
		const log2 = makeOrderPlacedLog({
			nonce: 2n,
			beneficiary: BENEFICIARY_2,
			outputAmount: 200_000_000n,
		})

		const reconstructed = await reconstructOrdersFromLogs([log1, log2], {
			getPlaceOrderCalldata: async (txHash, occurrenceIndex) => {
				expect(txHash).toBe(TX_HASH)
				return calldataByOccurrence[occurrenceIndex]
			},
		})

		expect(reconstructed).toHaveLength(2)
		expect(reconstructed[0].order.output.beneficiary.toLowerCase()).toBe(BENEFICIARY_1.toLowerCase())
		expect(reconstructed[0].order.predispatch.call).toBe("0x11")
		expect(reconstructed[0].order.output.call).toBe("0x21")
		expect(reconstructed[1].order.output.beneficiary.toLowerCase()).toBe(BENEFICIARY_2.toLowerCase())
		expect(reconstructed[1].order.predispatch.call).toBe("0x12")
		expect(reconstructed[1].order.output.call).toBe("0x22")
		expect(reconstructed[0].order.id).not.toBe(reconstructed[1].order.id)
	})

	it("would have produced a phantom commitment if the calldata helper always returned order 1 (regression guard)", async () => {
		const order1 = makeOrderStruct({
			nonce: 1n,
			beneficiary: BENEFICIARY_1,
			outputAmount: 100_000_000n,
			predispatchCall: "0x",
			outputCall: "0x",
		})

		const log2 = makeOrderPlacedLog({
			nonce: 2n,
			beneficiary: BENEFICIARY_2,
			outputAmount: 200_000_000n,
		})

		const reconstructed = await reconstructOrdersFromLogs([log2], {
			getPlaceOrderCalldata: async () => encodePlaceOrderCalldata(order1),
		})

		expect(reconstructed).toHaveLength(1)
		expect(reconstructed[0].order.output.beneficiary.toLowerCase()).toBe(BENEFICIARY_1.toLowerCase())
		expect(reconstructed[0].order.output.beneficiary.toLowerCase()).not.toBe(BENEFICIARY_2.toLowerCase())
	})

	it("isolates logs across different transactions", async () => {
		const tx1 = "0xaa".padEnd(66, "0") as HexString
		const tx2 = "0xbb".padEnd(66, "0") as HexString

		const order1 = makeOrderStruct({
			nonce: 1n,
			beneficiary: BENEFICIARY_1,
			outputAmount: 100_000_000n,
			predispatchCall: "0x",
			outputCall: "0x",
		})
		const order2 = makeOrderStruct({
			nonce: 2n,
			beneficiary: BENEFICIARY_2,
			outputAmount: 200_000_000n,
			predispatchCall: "0x",
			outputCall: "0x",
		})

		const log1 = {
			...makeOrderPlacedLog({ nonce: 1n, beneficiary: BENEFICIARY_1, outputAmount: 100_000_000n }),
			transactionHash: tx1,
		}
		const log2 = {
			...makeOrderPlacedLog({ nonce: 2n, beneficiary: BENEFICIARY_2, outputAmount: 200_000_000n }),
			transactionHash: tx2,
		}

		const reconstructed = await reconstructOrdersFromLogs(
			[log1 as DecodedOrderPlacedLog, log2 as DecodedOrderPlacedLog],
			{
				getPlaceOrderCalldata: async (txHash, occurrenceIndex) => {
					expect(occurrenceIndex).toBe(0)
					if (txHash === tx1) return encodePlaceOrderCalldata(order1)
					return encodePlaceOrderCalldata(order2)
				},
			},
		)

		expect(reconstructed).toHaveLength(2)
		expect(reconstructed.find((r) => r.transactionHash === tx1)!.order.output.beneficiary.toLowerCase()).toBe(
			BENEFICIARY_1.toLowerCase(),
		)
		expect(reconstructed.find((r) => r.transactionHash === tx2)!.order.output.beneficiary.toLowerCase()).toBe(
			BENEFICIARY_2.toLowerCase(),
		)
	})

	it("emits via onError and continues when one log's calldata fetch throws", async () => {
		const order2 = makeOrderStruct({
			nonce: 2n,
			beneficiary: BENEFICIARY_2,
			outputAmount: 200_000_000n,
			predispatchCall: "0x",
			outputCall: "0x",
		})

		const log1 = makeOrderPlacedLog({
			nonce: 1n,
			beneficiary: BENEFICIARY_1,
			outputAmount: 100_000_000n,
		})
		const log2 = makeOrderPlacedLog({
			nonce: 2n,
			beneficiary: BENEFICIARY_2,
			outputAmount: 200_000_000n,
		})

		const errors: { occurrenceIndex: number }[] = []
		const reconstructed = await reconstructOrdersFromLogs([log1, log2], {
			getPlaceOrderCalldata: async (_txHash, occurrenceIndex) => {
				if (occurrenceIndex === 0) throw new Error("rpc failure")
				return encodePlaceOrderCalldata(order2)
			},
			onError: (_err, _log, occurrenceIndex) => {
				errors.push({ occurrenceIndex })
			},
		})

		expect(errors).toEqual([{ occurrenceIndex: 0 }])
		expect(reconstructed).toHaveLength(1)
		expect((reconstructed[0].order as Order).nonce).toBe(2n)
	})
})
