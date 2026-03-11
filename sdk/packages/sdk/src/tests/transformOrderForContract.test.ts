import { describe, it, expect } from "vitest"
import { transformOrderForContract } from "@/protocols/intentsV2/utils"
import type { OrderV2, HexString } from "@/types"

const ADDR_20 = "0xEa4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString
const ADDR_32 = "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString
const NATIVE  = "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString

function makeOrder(overrides: Partial<OrderV2> = {}): OrderV2 {
	return {
		user: "0x" as HexString,
		source: "EVM-1",
		destination: "EVM-42161",
		deadline: 100n,
		nonce: 0n,
		fees: 0n,
		session: "0x0000000000000000000000000000000000000000" as HexString,
		predispatch: { assets: [], call: "0x" as HexString },
		inputs: [{ token: ADDR_20, amount: 1000n }],
		output: {
			beneficiary: ADDR_20,
			assets: [{ token: ADDR_20, amount: 990n }],
			call: "0x" as HexString,
		},
		...overrides,
	}
}

describe("transformOrderForContract", () => {
	it("left-pads 20-byte input token to bytes32", () => {
		const result = transformOrderForContract(makeOrder())
		expect(result.inputs[0].token).toBe(ADDR_32)
	})

	it("left-pads 20-byte output asset token to bytes32", () => {
		const result = transformOrderForContract(makeOrder())
		expect(result.output.assets[0].token).toBe(ADDR_32)
	})

	it("left-pads 20-byte output beneficiary to bytes32", () => {
		const result = transformOrderForContract(makeOrder())
		expect(result.output.beneficiary).toBe(ADDR_32)
	})

	it("left-pads 20-byte predispatch asset token to bytes32", () => {
		const order = makeOrder({
			predispatch: {
				assets: [{ token: ADDR_20, amount: 500n }],
				call: "0x" as HexString,
			},
		})
		const result = transformOrderForContract(order)
		expect(result.predispatch.assets[0].token).toBe(ADDR_32)
	})

	it("leaves already-padded 32-byte token unchanged", () => {
		const order = makeOrder({
			inputs: [{ token: ADDR_32, amount: 1000n }],
		})
		const result = transformOrderForContract(order)
		expect(result.inputs[0].token).toBe(ADDR_32)
	})

	it("handles native token (bytes32 zero) correctly", () => {
		const order = makeOrder({
			inputs: [{ token: NATIVE, amount: 1000n }],
		})
		const result = transformOrderForContract(order)
		expect(result.inputs[0].token).toBe(NATIVE)
	})

	it("hex-encodes string source and destination", () => {
		const result = transformOrderForContract(makeOrder())
		expect(result.source).toMatch(/^0x/)
		expect(result.destination).toMatch(/^0x/)
	})

	it("preserves already-hex source and destination", () => {
		const order = makeOrder({ source: "0xaabbcc" as HexString, destination: "0xddeeff" as HexString })
		const result = transformOrderForContract(order)
		expect(result.source).toBe("0xaabbcc")
		expect(result.destination).toBe("0xddeeff")
	})

	it("strips id and transactionHash", () => {
		const order = makeOrder({ id: "0xdeadbeef", transactionHash: "0xcafe" as HexString })
		const result = transformOrderForContract(order)
		expect("id" in result).toBe(false)
		expect("transactionHash" in result).toBe(false)
	})

	it("preserves amounts unchanged", () => {
		const result = transformOrderForContract(makeOrder())
		expect(result.inputs[0].amount).toBe(1000n)
		expect(result.output.assets[0].amount).toBe(990n)
	})
})
