import { decodeAcceptedSourceChains, encodeAcceptedSourceChains } from "@/protocols/intents/phantom-aggregation"

describe("accepted source chains declaration", () => {
	it("round-trips a list of state machine ids", () => {
		const chains = ["EVM-1", "EVM-8453", "EVM-42161"]
		expect(decodeAcceptedSourceChains(encodeAcceptedSourceChains(chains))).toEqual(chains)
	})

	// An empty declaration is a solver deliberately accepting nothing; collapsing it to null would
	// read as the legacy "accepts everything covered" default — the exact opposite.
	it("keeps an explicit empty declaration distinct from an absent one", () => {
		expect(decodeAcceptedSourceChains(encodeAcceptedSourceChains([]))).toEqual([])
		expect(decodeAcceptedSourceChains("0x")).toBeNull()
		expect(decodeAcceptedSourceChains(undefined)).toBeNull()
		expect(decodeAcceptedSourceChains(null)).toBeNull()
	})

	it("returns null for a blob with a different version byte", () => {
		const encoded = encodeAcceptedSourceChains(["EVM-1"])
		expect(decodeAcceptedSourceChains(`0x02${encoded.slice(4)}`)).toBeNull()
	})

	it("returns null for a real paymaster-shaped blob", () => {
		// EntryPoint v0.8 layout: 20-byte paymaster address followed by two 16-byte gas words.
		const paymasterBlob = `0x${"01".repeat(20)}${"00".repeat(32)}`
		expect(decodeAcceptedSourceChains(paymasterBlob)).toBeNull()
	})

	it("returns null when the blob is truncated mid-entry", () => {
		const encoded = encodeAcceptedSourceChains(["EVM-8453"])
		expect(decodeAcceptedSourceChains(encoded.slice(0, encoded.length - 4))).toBeNull()
	})

	it("returns null when trailing bytes follow the declared entries", () => {
		const encoded = encodeAcceptedSourceChains(["EVM-1"])
		expect(decodeAcceptedSourceChains(`${encoded}ff`)).toBeNull()
	})

	it("returns null for non-hex input", () => {
		expect(decodeAcceptedSourceChains("0xzz")).toBeNull()
		expect(decodeAcceptedSourceChains("nonsense")).toBeNull()
	})

	it("rejects encoding an unrepresentable declaration", () => {
		expect(() => encodeAcceptedSourceChains([""])).toThrow()
		expect(() => encodeAcceptedSourceChains(Array.from({ length: 256 }, (_, i) => `EVM-${i}`))).toThrow()
	})
})
