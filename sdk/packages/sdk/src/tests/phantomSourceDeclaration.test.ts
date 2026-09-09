import { encodePacked } from "viem"
import {
	decodeAcceptedSourceChains,
	decodePhantomBidDeclaration,
	decodePhantomBidPaymasterAndData,
	encodeAcceptedSourceChains,
	encodePhantomBidDeclaration,
	encodePhantomBidPaymasterAndData,
	PERMIT2_SPONSORSHIP_BYTES,
	type HexString,
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

// A bid built on the real-bid path carries the EntryPoint v0.8 paymasterAndData simplex packs for
// the Simplex paymaster's PERMIT2 mode, with the declaration appended after the permit. These
// fixtures pack it the way simplex's `packPaymasterAndData` and `buildPermit2Mode` do, field for
// field, so a layout drift between the two packages fails here rather than in production.
describe("phantom bid paymasterAndData — Permit2-sponsored bids", () => {
	const PAYMASTER = "0x0f9c4b1a2d3e4f5061728394a5b6c7d8e9f01234" as HexString
	const FEE_TOKEN = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913" as HexString
	const PERMIT_AMOUNT = 5_000_000n
	const NONCE = 2n ** 200n + 12345n
	const DEADLINE = 1_800_000_000n
	const R = `0x${"aa".repeat(32)}` as HexString
	const S = `0x${"bb".repeat(32)}` as HexString

	function permit2Sponsorship(mode = 2): HexString {
		const paymasterData = encodePacked(
			["uint8", "address", "uint256", "uint256", "uint256", "uint8", "bytes32", "bytes32"],
			[mode, FEE_TOKEN, PERMIT_AMOUNT, NONCE, DEADLINE, 27, R, S],
		)
		return encodePacked(["address", "uint128", "uint128", "bytes"], [PAYMASTER, 200_000n, 40_000n, paymasterData])
	}

	/** The 2612 permit mode (0x00) payload: mode(1)+token(20)+amount(32)+deadline(32)+v(1)+r(32)+s(32). */
	function permitSponsorship(): HexString {
		const paymasterData = encodePacked(
			["uint8", "address", "uint256", "uint256", "uint8", "bytes32", "bytes32"],
			[0, FEE_TOKEN, PERMIT_AMOUNT, DEADLINE, 27, R, S],
		)
		return encodePacked(["address", "uint128", "uint128", "bytes"], [PAYMASTER, 250_000n, 40_000n, paymasterData])
	}

	it("packs a complete Permit2 sponsorship at exactly the length the decoder expects", () => {
		expect((permit2Sponsorship().length - 2) / 2).toBe(PERMIT2_SPONSORSHIP_BYTES)
	})

	it("round-trips a declaration appended to a Permit2 sponsorship, reporting both", () => {
		const encoded = encodePhantomBidPaymasterAndData({
			sponsorship: permit2Sponsorship(),
			acceptedSourceChains: ["EVM-1", "EVM-8453"],
			uniswapV4Positions: [2905215n],
		})

		expect(encoded.startsWith(permit2Sponsorship())).toBe(true)
		expect(decodePhantomBidPaymasterAndData(encoded)).toEqual({
			mode: "permit2",
			declaration: { acceptedSources: ["EVM-1", "EVM-8453"], uniswapV4Positions: [2905215n] },
			sponsorship: {
				paymaster: PAYMASTER,
				token: FEE_TOKEN,
				permitAmount: PERMIT_AMOUNT,
				nonce: NONCE,
				deadline: DEADLINE,
			},
		})
	})

	// The declaration-only readers see exactly what they see on a bare bid, so nothing downstream
	// needs to know which shape a bid arrived in.
	it("hands the declaration readers the same answer for both shapes", () => {
		const bid = { acceptedSourceChains: ["EVM-56"], uniswapV4Positions: [7n] }
		const bare = encodePhantomBidPaymasterAndData(bid)
		const sponsored = encodePhantomBidPaymasterAndData({ ...bid, sponsorship: permit2Sponsorship() })

		expect(bare).toBe(encodePhantomBidDeclaration(bid))
		expect(decodePhantomBidDeclaration(sponsored)).toEqual(decodePhantomBidDeclaration(bare))
		expect(decodeAcceptedSourceChains(sponsored)).toEqual(["EVM-56"])
	})

	it("reads a sponsorship with nothing appended as a bid that declared nothing", () => {
		const decoded = decodePhantomBidPaymasterAndData(permit2Sponsorship())

		expect(decoded.mode).toBe("permit2")
		expect(decoded.declaration).toEqual({ acceptedSources: null, uniswapV4Positions: [] })
		expect(decoded.sponsorship?.paymaster).toBe(PAYMASTER)
	})

	// The tail is a whole declaration or nothing: a torn one must not half-read, but tearing the
	// declaration does not un-sponsor the bid.
	it("keeps the sponsorship and drops the declaration when the tail is malformed", () => {
		const encoded = encodePhantomBidPaymasterAndData({
			sponsorship: permit2Sponsorship(),
			acceptedSourceChains: ["EVM-1"],
		})
		const truncated = encoded.slice(0, encoded.length - 2) as HexString
		const trailing = `${encoded}ff` as HexString

		for (const blob of [truncated, trailing]) {
			const decoded = decodePhantomBidPaymasterAndData(blob)
			expect(decoded.mode).toBe("permit2")
			expect(decoded.declaration).toEqual({ acceptedSources: null, uniswapV4Positions: [] })
			expect(decoded.sponsorship?.nonce).toBe(NONCE)
		}
	})

	// Only PERMIT2 mode is a bid: simplex confines the 2612 permit (mode 0x00) to a first-time
	// delegation, so a bid carrying one is not something a solver produces, and the retired
	// allowance mode (0x01) is refused by the paymaster itself.
	it("does not read other paymaster modes as a sponsorship", () => {
		expect(decodePhantomBidPaymasterAndData(permitSponsorship()).mode).toBe("none")
		expect(decodePhantomBidPaymasterAndData(permit2Sponsorship(1)).mode).toBe("none")
		expect(decodePhantomBidPaymasterAndData(permit2Sponsorship().slice(0, -2) as HexString).mode).toBe("none")
	})

	it("reads a bare declaration exactly as before, whatever its length", () => {
		// Long enough to reach the sponsorship's mode offset, so the bare parse has to win outright.
		const chains = Array.from({ length: 40 }, (_, i) => `EVM-${100000 + i}`)
		const decoded = decodePhantomBidPaymasterAndData(encodePhantomBidDeclaration({ acceptedSourceChains: chains }))

		expect(decoded).toEqual({
			mode: "declaration",
			declaration: { acceptedSources: chains, uniswapV4Positions: [] },
			sponsorship: null,
		})
		expect(decodePhantomBidPaymasterAndData("0x")).toEqual({
			mode: "none",
			declaration: { acceptedSources: null, uniswapV4Positions: [] },
			sponsorship: null,
		})
	})

	// A caller must not be able to sign bytes the aggregation would read as "declared nothing".
	it("refuses to append a declaration to anything but a Permit2-mode sponsorship", () => {
		const declaration = { acceptedSourceChains: ["EVM-1"] }
		expect(() => encodePhantomBidPaymasterAndData({ ...declaration, sponsorship: permitSponsorship() })).toThrow()
		expect(() => encodePhantomBidPaymasterAndData({ ...declaration, sponsorship: "0x1234" })).toThrow()
		expect(() =>
			encodePhantomBidPaymasterAndData({ ...declaration, sponsorship: `${permit2Sponsorship()}00` as HexString }),
		).toThrow()
		expect(() =>
			encodePhantomBidPaymasterAndData({ ...declaration, sponsorship: "nonsense" as HexString }),
		).toThrow()
		// No sponsorship at all is the bare shape, not an error.
		expect(encodePhantomBidPaymasterAndData({ ...declaration, sponsorship: "0x" })).toBe(
			encodePhantomBidDeclaration(declaration),
		)
		expect(encodePhantomBidPaymasterAndData({ ...declaration, sponsorship: null })).toBe(
			encodePhantomBidDeclaration(declaration),
		)
	})
})
