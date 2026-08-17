import { describe, it, expect } from "vitest"
import { stringToHex, type Hex } from "viem"
import { type DecodedOrderPlacedLog, type HexString, type Order, orderCommitment, normalizeStateMachineId } from "@hyperbridge/sdk"
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
const GRAFFITI = `0x${"00".repeat(32)}` as Hex

function makeOrderPlacedLog(args: {
	nonce: bigint
	beneficiary: Hex
	outputAmount: bigint
	predispatchCall?: HexString
	outputCall?: HexString
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
			predispatchCall: args.predispatchCall,
			outputCall: args.outputCall,
			graffiti: GRAFFITI as HexString,
		},
	} as unknown as DecodedOrderPlacedLog
}

describe("reconstructOrdersFromLogs", () => {
	it("reconstructs the complete order from the log alone", () => {
		const log = makeOrderPlacedLog({
			nonce: 1n,
			beneficiary: BENEFICIARY_1,
			outputAmount: 100_000_000n,
			predispatchCall: "0x11",
			outputCall: "0x21",
		})

		const reconstructed = reconstructOrdersFromLogs([log])

		expect(reconstructed).toHaveLength(1)
		const { order } = reconstructed[0]
		expect(order.output.beneficiary.toLowerCase()).toBe(BENEFICIARY_1.toLowerCase())
		expect(order.predispatch.call).toBe("0x11")
		expect(order.output.call).toBe("0x21")

		const expected: Order = {
			user: USER as HexString,
			source: normalizeStateMachineId(stringToHex(SOURCE)) as HexString,
			destination: normalizeStateMachineId(stringToHex(DESTINATION)) as HexString,
			deadline: 1700000000n,
			nonce: 1n,
			fees: 0n,
			session: SESSION as HexString,
			predispatch: { assets: [], call: "0x11" },
			inputs: [{ token: USDC_INPUT as HexString, amount: 100_000_000n }],
			output: {
				beneficiary: BENEFICIARY_1 as HexString,
				assets: [{ token: USDC_OUTPUT as HexString, amount: 100_000_000n }],
				call: "0x21",
			},
		}
		expect(order.id).toBe(orderCommitment(expected))
	})

	it("keeps orders self-contained when a single tx emits multiple OrderPlaced logs", () => {
		const log1 = makeOrderPlacedLog({
			nonce: 1n,
			beneficiary: BENEFICIARY_1,
			outputAmount: 100_000_000n,
			predispatchCall: "0x11",
			outputCall: "0x21",
		})
		const log2 = makeOrderPlacedLog({
			nonce: 2n,
			beneficiary: BENEFICIARY_2,
			outputAmount: 200_000_000n,
			predispatchCall: "0x12",
			outputCall: "0x22",
		})

		const reconstructed = reconstructOrdersFromLogs([log1, log2])

		expect(reconstructed).toHaveLength(2)
		expect(reconstructed[0].order.output.beneficiary.toLowerCase()).toBe(BENEFICIARY_1.toLowerCase())
		expect(reconstructed[0].order.predispatch.call).toBe("0x11")
		expect(reconstructed[0].order.output.call).toBe("0x21")
		expect(reconstructed[1].order.output.beneficiary.toLowerCase()).toBe(BENEFICIARY_2.toLowerCase())
		expect(reconstructed[1].order.predispatch.call).toBe("0x12")
		expect(reconstructed[1].order.output.call).toBe("0x22")
		expect(reconstructed[0].order.id).not.toBe(reconstructed[1].order.id)
	})

	it("emits via onError and continues when a log is missing its call payloads", () => {
		const legacyLog = makeOrderPlacedLog({
			nonce: 1n,
			beneficiary: BENEFICIARY_1,
			outputAmount: 100_000_000n,
		})
		const currentLog = makeOrderPlacedLog({
			nonce: 2n,
			beneficiary: BENEFICIARY_2,
			outputAmount: 200_000_000n,
			predispatchCall: "0x",
			outputCall: "0x",
		})

		const errors: unknown[] = []
		const reconstructed = reconstructOrdersFromLogs([legacyLog, currentLog], {
			onError: (err) => {
				errors.push(err)
			},
		})

		expect(errors).toHaveLength(1)
		expect(String(errors[0])).toContain("missing its call payloads")
		expect(reconstructed).toHaveLength(1)
		expect((reconstructed[0].order as Order).nonce).toBe(2n)
	})
})
