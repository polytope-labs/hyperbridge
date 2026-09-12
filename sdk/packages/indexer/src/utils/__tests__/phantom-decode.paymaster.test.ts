import { ethers } from "ethers"
import {
	decodePhantomBidDeclaration,
	decodePhantomBidPaymasterAndData,
	encodePhantomBidDeclaration,
	encodePhantomBidPaymasterAndData,
	PERMIT2_SPONSORSHIP_BYTES,
	type HexString,
} from "@hyperbridge/sdk/intents-helpers"

// A phantom bid's paymasterAndData arrives in one of two shapes: the bare declaration every bid
// carried until now, or — for a bid built on simplex's real-bid path — the EntryPoint v0.8
// payload for the Simplex paymaster's PERMIT2 mode with the declaration appended after the
// permit. The aggregation reads the declaration off either through the SDK's decoder, so what
// is checked here is that the decoder the indexer ships (the `intents-helpers` sub-path, the
// VM2-safe entry) recognises a payload packed the way simplex packs it — reproduced with
// ethers' `solidityPack`, the indexer's own ABI coder, rather than viem's `encodePacked`.
//
// This file deliberately imports only the `intents-helpers` sub-path, like the sibling
// `phantom-decode.fill.test.ts`; the reasoning is in that file's header.
describe("phantom bid paymasterAndData shapes", () => {
	const PAYMASTER = "0x0f9c4b1a2d3e4f5061728394a5b6c7d8e9f01234" as HexString
	const FEE_TOKEN = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913" as HexString
	const PERMIT_AMOUNT = 5_000_000n
	const NONCE = 2n ** 200n + 12345n
	const DEADLINE = 1_800_000_000n

	/** `packPaymasterAndData(buildPermit2Mode(...))` as simplex packs it, via ethers. */
	function permit2Sponsorship(): HexString {
		const paymasterData = ethers.utils.solidityPack(
			["uint8", "address", "uint256", "uint256", "uint256", "uint8", "bytes32", "bytes32"],
			[2, FEE_TOKEN, PERMIT_AMOUNT, NONCE, DEADLINE, 27, `0x${"aa".repeat(32)}`, `0x${"bb".repeat(32)}`],
		)
		return ethers.utils.solidityPack(
			["address", "uint128", "uint128", "bytes"],
			[PAYMASTER, 200_000n, 40_000n, paymasterData],
		) as HexString
	}

	it("packs the sponsorship at the length the decoder expects", () => {
		expect(ethers.utils.arrayify(permit2Sponsorship())).toHaveLength(PERMIT2_SPONSORSHIP_BYTES)
	})

	it("reads the declaration appended to a Permit2-sponsored bid, and the sponsorship behind it", () => {
		const encoded = encodePhantomBidPaymasterAndData({
			sponsorship: permit2Sponsorship(),
			acceptedSourceChains: ["EVM-1", "EVM-8453"],
			uniswapV4Positions: [2905215n],
		})

		const decoded = decodePhantomBidPaymasterAndData(encoded)

		expect(decoded.mode).toBe("permit2")
		expect(decoded.declaration).toEqual({ acceptedSources: ["EVM-1", "EVM-8453"], uniswapV4Positions: [2905215n] })
		expect(decoded.sponsorship).toEqual({
			paymaster: PAYMASTER,
			token: FEE_TOKEN,
			permitAmount: PERMIT_AMOUNT,
			nonce: NONCE,
			deadline: DEADLINE,
		})
	})

	it("reads a sponsored bid with nothing appended as declaring nothing", () => {
		const decoded = decodePhantomBidPaymasterAndData(permit2Sponsorship())

		expect(decoded.mode).toBe("permit2")
		expect(decoded.declaration).toEqual({ acceptedSources: null, uniswapV4Positions: [] })
	})

	it("still reads a bare declaration exactly as before", () => {
		const bare = encodePhantomBidDeclaration({ acceptedSourceChains: ["EVM-56"], uniswapV4Positions: [7n] })

		expect(decodePhantomBidPaymasterAndData(bare)).toEqual({
			mode: "declaration",
			declaration: { acceptedSources: ["EVM-56"], uniswapV4Positions: [7n] },
			sponsorship: null,
		})
		expect(decodePhantomBidDeclaration(bare)).toEqual(
			decodePhantomBidDeclaration(`${permit2Sponsorship()}${bare.slice(2)}`),
		)
	})
})
