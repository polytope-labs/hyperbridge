import { encodeFunctionData } from "viem"
import {
	encodeERC7821ExecuteBatch,
	FILL_ORDER_ABI,
	FILL_ORDER_V1_ABI,
	type HexString,
} from "@hyperbridge/sdk/intents-helpers"
import { extractFillDataVm2 } from "@/utils/phantom-decode"

// A solver encodes its fill with viem (simplex), against whichever gateway it targets, and the
// indexer decodes it with ethers. `FillOptions.validUntil` gave `fillOrder` two shapes with
// different selectors (0x5cfb1ea5 -> 0xa5470064), so a decoder that knows only one silently
// rejects every bid of the other — and `aggregatePhantomBids` drops an undecodable bid with no
// log line. Both shapes are live: gateways predating `validUntil` take v1, upgraded ones v2.
//
// This file deliberately imports only the `intents-helpers` sub-path, the same VM2-safe entry the
// indexer uses at runtime. The sibling `phantom-decode.test.ts` pulls the full `@hyperbridge/sdk`
// bundle, which jest cannot transform once the SDK's dist is freshly built (its CJS output
// requires ESM-only `lodash-es`, and there is no `transformIgnorePatterns` override).
describe("extractFillDataVm2", () => {
	const GATEWAY = "0x1111111111111111111111111111111111111111" as HexString
	const OUTPUT_TOKEN = `0x${"55".repeat(32)}` as HexString
	const SOLVER_AMOUNT = 12_345n

	const order = {
		user: `0x${"11".repeat(32)}`,
		source: "0x",
		destination: "0x",
		deadline: 0n,
		nonce: 0n,
		fees: 0n,
		session: `0x${"22".repeat(20)}`,
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: `0x${"33".repeat(32)}`, amount: 1n }],
		output: {
			beneficiary: `0x${"44".repeat(32)}`,
			assets: [{ token: OUTPUT_TOKEN, amount: 2n }],
			call: "0x",
		},
	}
	const outputs = [{ token: OUTPUT_TOKEN, amount: SOLVER_AMOUNT }]

	/** A bid as it arrives: the fillOrder call wrapped in the solver account's ERC-7821 batch. */
	function bid(abi: unknown, args: unknown[]): HexString {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const fillCalldata = (encodeFunctionData as any)({ abi, functionName: "fillOrder", args }) as HexString
		return encodeERC7821ExecuteBatch([{ target: GATEWAY, value: 0n, data: fillCalldata }])
	}

	it("decodes a bid encoded for an upgraded gateway (v2, with validUntil)", () => {
		const calldata = bid(FILL_ORDER_ABI, [order, { relayerFee: 0n, nativeDispatchFee: 0n, validUntil: 99n, outputs }])

		const result = extractFillDataVm2(calldata, GATEWAY)

		expect(result).not.toBeNull()
		expect(result!.legs).toHaveLength(1)
		expect(result!.legs[0].outputToken.toLowerCase()).toBe(OUTPUT_TOKEN.toLowerCase())
		expect(result!.legs[0].solverAmount).toBe(SOLVER_AMOUNT)
	})

	// The regression: every mainnet gateway still predates `validUntil`, so this is the shape
	// actually on the wire today. Decoding only v2 dropped all of it.
	it("decodes a bid encoded for a gateway that predates validUntil (v1)", () => {
		const calldata = bid(FILL_ORDER_V1_ABI, [order, { relayerFee: 0n, nativeDispatchFee: 0n, outputs }])

		const result = extractFillDataVm2(calldata, GATEWAY)

		expect(result).not.toBeNull()
		expect(result!.legs).toHaveLength(1)
		expect(result!.legs[0].outputToken.toLowerCase()).toBe(OUTPUT_TOKEN.toLowerCase())
		expect(result!.legs[0].solverAmount).toBe(SOLVER_AMOUNT)
	})

	it("returns null when the batch targets a different contract", () => {
		const calldata = bid(FILL_ORDER_ABI, [order, { relayerFee: 0n, nativeDispatchFee: 0n, validUntil: 0n, outputs }])

		expect(extractFillDataVm2(calldata, "0x2222222222222222222222222222222222222222")).toBeNull()
	})

	it("returns null for calldata that is neither fillOrder shape", () => {
		const calldata = encodeERC7821ExecuteBatch([{ target: GATEWAY, value: 0n, data: "0xdeadbeef" }])

		expect(extractFillDataVm2(calldata, GATEWAY)).toBeNull()
	})
})
