import { stubOrderScanner } from "../helpers/stub-scanner"
import { describe, expect, it, vi } from "vitest"
import type { HexString, Order, PhantomOrderEvent } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"

/**
 * A phantom order is only safe to quote because it is already expired.
 *
 * The pallet builds one with the chain's latest *confirmed* height as its deadline so it "can
 * never be executed for real", and `fillOrder` reverts `Expired()` once `deadline < block.number`.
 * That is what lets `quotePhantomFill` skip every ceiling a real bid gets — no budget, no
 * wallet-balance read, neither profit gate — and still be harmless.
 *
 * Nothing upstream proves the order body actually came from the pallet: it is read from a single
 * Hyperbridge node's offchain storage, and `fetchPhantomOrder` assigns the commitment from the
 * event rather than re-deriving it from the bytes. So a body with a live deadline would turn an
 * unbounded quote into a signed, executable authorization to fill an order of someone else's
 * choosing. These tests pin the guard that refuses it.
 */

const COMMITMENT = "0x1111111111111111111111111111111111111111111111111111111111111111" as HexString
const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const CHAIN = "EVM-8453"
const CHAIN_ID = 8453
const TOKEN = "0x0000000000000000000000000000000000000000000000000000000000000001" as HexString

const CURRENT_BLOCK = 1_000n

function event(): PhantomOrderEvent {
	return { commitment: COMMITMENT, chain: CHAIN, createdAt: 1, legs: [] }
}

function order(deadline: bigint): Order {
	return {
		id: COMMITMENT,
		user: "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString,
		source: CHAIN,
		destination: CHAIN,
		deadline,
		nonce: 1n,
		fees: 0n,
		session: "0x0000000000000000000000000000000000000000" as HexString,
		predispatch: { assets: [], call: "0x" as HexString },
		inputs: [{ token: TOKEN, amount: 1_000_000n }],
		output: {
			beneficiary: "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString,
			assets: [{ token: TOKEN, amount: 0n }],
			call: "0x" as HexString,
		},
	} as unknown as Order
}

function build(deadline: bigint, opts: { blockNumberThrows?: boolean } = {}) {
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

	// Standing in for the quote path: being called at all is the signal that the expiry guard
	// let the order through.
	const quotePhantomLeg = vi.fn(async () => null)
	;(filler as any).quotePhantomLeg = quotePhantomLeg

	const coprocessor = { fetchPhantomOrder: vi.fn(async () => order(deadline)) } as any

	return {
		run: () => (filler as any).preparePhantomBid(event(), coprocessor),
		quotePhantomLeg,
		getBlockNumber,
	}
}

describe("phantom order expiry guard", () => {
	it("quotes an order whose deadline has already passed", async () => {
		const { run, quotePhantomLeg } = build(CURRENT_BLOCK - 1n)

		await run()

		expect(quotePhantomLeg).toHaveBeenCalled()
	})

	it("refuses an order that is still fillable", async () => {
		// deadline >= current block means fillOrder would NOT revert Expired() — the bid we sign
		// would be executable, which a genuine phantom order never is.
		const { run, quotePhantomLeg } = build(CURRENT_BLOCK + 1n)

		await expect(run()).resolves.toBeNull()
		expect(quotePhantomLeg).not.toHaveBeenCalled()
	})

	it("refuses an order whose deadline is exactly the current block", async () => {
		// fillOrder's check is `deadline < block.number`, so deadline == head is still fillable.
		const { run, quotePhantomLeg } = build(CURRENT_BLOCK)

		await expect(run()).resolves.toBeNull()
		expect(quotePhantomLeg).not.toHaveBeenCalled()
	})

	it("refuses to quote when the current block cannot be read", async () => {
		// Failing closed matters more than availability here: without a height there is no way to
		// tell an expired order from a live one, and quoting is the unsafe branch.
		const { run, quotePhantomLeg } = build(CURRENT_BLOCK - 1n, { blockNumberThrows: true })

		await expect(run()).resolves.toBeNull()
		expect(quotePhantomLeg).not.toHaveBeenCalled()
	})
})
