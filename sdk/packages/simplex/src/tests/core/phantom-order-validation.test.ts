import { stubOrderScanner } from "../helpers/stub-scanner"
import { describe, expect, it, vi } from "vitest"
import type { HexString, Order, PhantomOrderEvent } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import { encodeAbiParameters, decodeAbiParameters, hexToBytes, toHex } from "viem"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"

/**
 * A phantom order is only safe to quote because it can never be filled.
 *
 * `quotePhantomFill` deliberately runs with no budget, no wallet-balance read and neither
 * profit gate, so the amounts the filler signs are bounded only by the order body. That is
 * fine for an order the chain will refuse — and not fine for anything else.
 *
 * Nothing upstream establishes which it is: the body comes from a single Hyperbridge node's
 * offchain storage, and `fetchPhantomOrder` assigns the commitment from the event rather than
 * re-deriving it from the bytes. These tests pin the checks that close that gap.
 *
 * `phantom_order_commitment` fixes every field asserted here, and each one independently
 * makes the bid unexecutable — `session` most directly, since `_select` recovers a key with
 * `ECDSA.recover`, which can never return the zero address.
 */

const COMMITMENT = "0x1111111111111111111111111111111111111111111111111111111111111111" as HexString
const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const CHAIN = "EVM-8453"
const CHAIN_ID = 8453
const TOKEN = "0x0000000000000000000000000000000000000000000000000000000000000001" as HexString
const ZERO_ADDRESS = "0x0000000000000000000000000000000000000000" as HexString
const ZERO_BYTES32 = "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString

const CURRENT_BLOCK = 1_000n

function event(): PhantomOrderEvent {
	return { commitment: COMMITMENT, chain: CHAIN, createdAt: 1, legs: [] }
}

/** A well-formed phantom order, exactly as `phantom_order_commitment` builds one. */
function order(overrides: Partial<Order> = {}): Order {
	return {
		id: COMMITMENT,
		user: ZERO_BYTES32,
		source: CHAIN,
		destination: CHAIN,
		deadline: CURRENT_BLOCK - 1n,
		nonce: 1n,
		fees: 0n,
		session: ZERO_ADDRESS,
		predispatch: { assets: [], call: "0x" as HexString },
		inputs: [{ token: TOKEN, amount: 1_000_000n }],
		output: {
			beneficiary: ZERO_BYTES32,
			assets: [{ token: TOKEN, amount: 0n }],
			call: "0x" as HexString,
		},
		...overrides,
	} as unknown as Order
}

function build(phantomOrder: Order, opts: { blockNumberThrows?: boolean } = {}) {
	const configService = {
		getConfiguredChainIds: () => [CHAIN_ID],
		getEntryPointAddress: () => ENTRY_POINT,
		getHyperbridgeWsUrl: () => undefined,
		getSubstratePrivateKey: () => undefined,
	} as any

	const getBlockNumber = opts.blockNumberThrows
		? vi.fn().mockRejectedValue(new Error("RPC down"))
		: vi.fn().mockResolvedValue(CURRENT_BLOCK)

	const chainClientManager = { getPublicClient: vi.fn().mockReturnValue({ getBlockNumber }) } as any

	const filler = new IntentFiller(
		[],
		[],
		{ maxConcurrentOrders: 1, watchOnly: {} } as any,
		configService,
		chainClientManager,
		{} as any, // ContractInteractionService — never reached in these tests
		{ address: "0xAAAA00000000000000000000000000000000AAAA" as HexString } as any,
		{ orders: stubOrderScanner() },
	)

	// Standing in for the quote path: being called at all is the signal that the checks let
	// the order through.
	const quotePhantomLeg = vi.fn(async () => null)
	;(filler as any).quotePhantomLeg = quotePhantomLeg

	const coprocessor = { fetchPhantomOrder: vi.fn(async () => phantomOrder) } as any

	return { run: () => (filler as any).preparePhantomBid(event(), coprocessor), quotePhantomLeg, getBlockNumber }
}

describe("phantom order validation", () => {
	it("quotes a well-formed, already-expired order", async () => {
		const { run, quotePhantomLeg } = build(order())

		await run()

		expect(quotePhantomLeg).toHaveBeenCalled()
	})

	describe("structural invariants", () => {
		it("refuses an order carrying a real session key", async () => {
			// The decisive one. A non-zero session is what makes `_select` able to stage a
			// selection, which is what makes the signed bid executable.
			const { run, quotePhantomLeg } = build(
				order({ session: "0x00000000000000000000000000000000000000aa" as HexString }),
			)

			await expect(run()).resolves.toBeNull()
			expect(quotePhantomLeg).not.toHaveBeenCalled()
		})

		it("refuses an order requesting a non-zero output amount", async () => {
			// Genuine phantom legs request zero, which is why `_fillSameChain` transfers nothing.
			const { run, quotePhantomLeg } = build(
				order({ output: { beneficiary: ZERO_BYTES32, assets: [{ token: TOKEN, amount: 1n }], call: "0x" } as any }),
			)

			await expect(run()).resolves.toBeNull()
			expect(quotePhantomLeg).not.toHaveBeenCalled()
		})

		it("refuses an order whose chains do not match the announced one", async () => {
			const { run, quotePhantomLeg } = build(order({ destination: "EVM-10" } as any))

			await expect(run()).resolves.toBeNull()
			expect(quotePhantomLeg).not.toHaveBeenCalled()
		})

		it("checks the structure without touching an RPC", async () => {
			// The checks run inside the on-chain bid window, so they must not depend on a
			// round trip that could be slow or unavailable.
			const { run, getBlockNumber } = build(
				order({ session: "0x00000000000000000000000000000000000000aa" as HexString }),
			)

			await run()

			expect(getBlockNumber).not.toHaveBeenCalled()
		})
	})

	describe("deadline cross-check", () => {
		it("refuses an order that is still fillable", async () => {
			// deadline >= head means fillOrder would NOT revert Expired(), so the bid we sign
			// would be executable — which a genuine phantom order never is.
			const { run, quotePhantomLeg } = build(order({ deadline: CURRENT_BLOCK + 1n }))

			await expect(run()).resolves.toBeNull()
			expect(quotePhantomLeg).not.toHaveBeenCalled()
		})

		it("refuses an order whose deadline is exactly the current block", async () => {
			// fillOrder's check is `deadline < block.number`, so deadline == head is fillable.
			const { run, quotePhantomLeg } = build(order({ deadline: CURRENT_BLOCK }))

			await expect(run()).resolves.toBeNull()
			expect(quotePhantomLeg).not.toHaveBeenCalled()
		})

		it("still quotes when the head cannot be read, since the structural checks stand alone", async () => {
			// Hard-failing here would let one flaky endpoint stop this filler bidding at all,
			// and it buys nothing: the invariants above already make a forged body unusable.
			const { run, quotePhantomLeg } = build(order(), { blockNumberThrows: true })

			await run()

			expect(quotePhantomLeg).toHaveBeenCalled()
		})
	})
})

/**
 * The checks above run against a hand-built fixture, which bakes in an assumption: that a real
 * pallet-generated order, ABI-encoded on the Rust side and decoded by `fetchPhantomOrder`,
 * actually presents `source`/`destination` as the same string the event carries in `chain`.
 *
 * If that assumption is wrong the guard refuses every genuine phantom order and the filler
 * silently stops bidding — which is exactly what the simnode E2E would catch, and what nothing
 * here would. So this rebuilds the order the way `phantom_order_commitment` does
 * (`intents-coprocessor/src/types.rs`), encodes it with the real `placeOrder` ABI, and decodes it
 * the way `fetchPhantomOrder` does, before running it through the guard.
 */
describe("phantom order validation — against a pallet-shaped order", () => {
	const CHAIN_BYTES = toHex(new TextEncoder().encode(CHAIN)) as HexString

	/** Mirrors `sol_types::Order` as the pallet fills it in for a phantom order. */
	function palletEncodedOrder(deadline: bigint): HexString {
		const orderType = (INTENT_GATEWAY_V2_ABI as readonly any[]).find(
			(e) => e.type === "function" && e.name === "placeOrder",
		)!.inputs[0]

		return encodeAbiParameters(
			[orderType],
			[
				{
					user: ZERO_BYTES32,
					source: CHAIN_BYTES,
					destination: CHAIN_BYTES,
					deadline,
					nonce: 7n,
					fees: 0n,
					session: ZERO_ADDRESS,
					predispatch: { assets: [], call: "0x" },
					// token_a on the input side, token_b on the output side, output amount zero.
					inputs: [{ token: TOKEN, amount: 1_000_000n }],
					output: { beneficiary: ZERO_BYTES32, assets: [{ token: TOKEN, amount: 0n }], call: "0x" },
				},
			] as any,
		) as HexString
	}

	/** Mirrors `IntentsCoprocessor.fetchPhantomOrder`'s decode. */
	function decodeLikeCoprocessor(encoded: HexString): Order {
		const orderType = (INTENT_GATEWAY_V2_ABI as readonly any[]).find(
			(e) => e.type === "function" && e.name === "placeOrder",
		)!.inputs[0]
		const [d] = decodeAbiParameters([orderType], encoded) as unknown as any[]
		const textDecoder = new TextDecoder()
		return {
			...d,
			id: COMMITMENT,
			source: textDecoder.decode(hexToBytes(d.source)),
			destination: textDecoder.decode(hexToBytes(d.destination)),
		} as unknown as Order
	}

	it("round-trips the pallet's chain bytes into the string the event carries", () => {
		const decoded = decodeLikeCoprocessor(palletEncodedOrder(CURRENT_BLOCK - 1n))

		// The pallet writes the same `chain_bytes` into the event and into source/destination,
		// so the guard's equality check is comparing like with like.
		expect(decoded.source).toBe(CHAIN)
		expect(decoded.destination).toBe(CHAIN)
		expect(event().chain).toBe(CHAIN)
	})

	it("quotes a genuine pallet-shaped order", async () => {
		const { run, quotePhantomLeg } = build(decodeLikeCoprocessor(palletEncodedOrder(CURRENT_BLOCK - 1n)))

		await run()

		expect(quotePhantomLeg).toHaveBeenCalled()
	})

	it("still refuses one whose deadline has not passed", async () => {
		const { run, quotePhantomLeg } = build(decodeLikeCoprocessor(palletEncodedOrder(CURRENT_BLOCK + 1n)))

		await expect(run()).resolves.toBeNull()
		expect(quotePhantomLeg).not.toHaveBeenCalled()
	})
})
