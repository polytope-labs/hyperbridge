import {
	decodeAcceptedSourceChains,
	decodePhantomBidDeclaration,
	encodeAcceptedSourceChains,
	encodePhantomBidDeclaration,
} from "@/protocols/intents/phantom-aggregation"

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

	it("returns null for a blob with an unknown version byte", () => {
		const encoded = encodeAcceptedSourceChains(["EVM-1"])
		expect(decodeAcceptedSourceChains(`0x03${encoded.slice(4)}`)).toBeNull()
		// v2's own version byte over a v1 body is still a malformed v2 — the positions section
		// it promises is simply absent — so it must not half-read as "sources, no positions".
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

describe("phantom bid declaration v2 — Uniswap V4 positions", () => {
	it("round-trips sources and positions together", () => {
		const declaration = { acceptedSourceChains: ["EVM-1", "EVM-8453"], uniswapV4Positions: [2905215n, 2906058n] }
		expect(decodePhantomBidDeclaration(encodePhantomBidDeclaration(declaration))).toEqual({
			acceptedSources: declaration.acceptedSourceChains,
			uniswapV4Positions: declaration.uniswapV4Positions,
		})
	})

	// The whole point of the version split: a solver that declares no positions must keep emitting
	// exactly the bytes it emitted before positions existed, so nothing downstream sees a change.
	it("emits the v1 layout byte-for-byte when no positions are declared", () => {
		const chains = ["EVM-1", "EVM-8453"]
		expect(encodePhantomBidDeclaration({ acceptedSourceChains: chains })).toBe(encodeAcceptedSourceChains(chains))
		expect(encodePhantomBidDeclaration({ acceptedSourceChains: chains }).slice(0, 4)).toBe("0x01")
		expect(
			encodePhantomBidDeclaration({ acceptedSourceChains: chains, uniswapV4Positions: [1n] }).slice(0, 4),
		).toBe("0x02")
	})

	it("reads v1 bids as declaring no positions rather than failing", () => {
		const v1 = encodeAcceptedSourceChains(["EVM-56"])
		expect(decodePhantomBidDeclaration(v1)).toEqual({ acceptedSources: ["EVM-56"], uniswapV4Positions: [] })
	})

	it("carries positions with no accepted-source declaration", () => {
		const encoded = encodePhantomBidDeclaration({ uniswapV4Positions: [7n] })
		expect(decodePhantomBidDeclaration(encoded)).toEqual({ acceptedSources: [], uniswapV4Positions: [7n] })
	})

	it("round-trips tokenIds across the whole uint256 range", () => {
		const ids = [0n, 1n, 255n, 256n, 2n ** 64n, 2n ** 256n - 1n]
		expect(
			decodePhantomBidDeclaration(encodePhantomBidDeclaration({ uniswapV4Positions: ids })).uniswapV4Positions,
		).toEqual(ids)
	})

	it("refuses a truncated or over-long positions section", () => {
		const encoded = encodePhantomBidDeclaration({ acceptedSourceChains: ["EVM-1"], uniswapV4Positions: [2905215n] })
		expect(decodePhantomBidDeclaration(encoded.slice(0, encoded.length - 2)).acceptedSources).toBeNull()
		expect(decodePhantomBidDeclaration(`${encoded}ff`).acceptedSources).toBeNull()
	})

	it("rejects encoding an unrepresentable position list", () => {
		expect(() => encodePhantomBidDeclaration({ uniswapV4Positions: [-1n] })).toThrow()
		expect(() =>
			encodePhantomBidDeclaration({ uniswapV4Positions: Array.from({ length: 256 }, (_, i) => BigInt(i)) }),
		).toThrow()
	})
})
