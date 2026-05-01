import { describe, expect, it } from "vitest"
import {
	bytes20ToBytes32,
	bytes32ToBytes20,
	encodeStateMachineId,
	normalizeAddressForStateMachine,
	normalizeStateMachineId,
} from "@/utils"

const ADDR_20 = "0xEa4f68301aCec0dc9Bbe10F15730c59FB79d237E"
const ADDR_32 = "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E"

describe("normalization helpers", () => {
	it("accepts 20-byte and 32-byte addresses when converting to bytes20", () => {
		expect(bytes32ToBytes20(ADDR_20)).toBe(ADDR_20)
		expect(bytes32ToBytes20(ADDR_32)).toBe(ADDR_20.toLowerCase())
	})

	it("accepts 20-byte and 32-byte addresses when converting to bytes32", () => {
		expect(bytes20ToBytes32(ADDR_20)).toBe(ADDR_32)
		expect(bytes20ToBytes32(ADDR_32)).toBe(ADDR_32)
	})

	it("normalizes addresses based on destination state machine", () => {
		expect(normalizeAddressForStateMachine(ADDR_20, "EVM-8453")).toBe(ADDR_32)
		expect(normalizeAddressForStateMachine(ADDR_20, "SUBSTRATE-hyperbridge")).toBe(ADDR_20)
	})

	it("round-trips state machine ids between string and hex", () => {
		const encoded = encodeStateMachineId("EVM-8453")
		expect(normalizeStateMachineId(encoded)).toBe("EVM-8453")
		expect(normalizeStateMachineId("EVM-8453")).toBe("EVM-8453")
	})
})
