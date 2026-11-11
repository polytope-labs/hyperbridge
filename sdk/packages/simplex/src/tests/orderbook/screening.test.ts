import { afterEach, describe, expect, it, vi } from "vitest"
import { concat, encodeAbiParameters, keccak256, recoverAddress, toHex } from "viem"
import { generatePrivateKey } from "viem/accounts"
import { type HexString, type Order, orderCommitment } from "@hyperbridge/sdk"
import { encodeOrder, OrderScreener, requiresScreening } from "@/orderbook/screening"
import type { ScreenOrderResult } from "@/orderbook/types"
import { OrderbookClient, OrderbookRequestError } from "@/orderbook/client"
import { privateKeySigner } from "@/services/wallet"

const DOMAIN = { name: "HyperFX Orderbook", version: "1" }

function order(overrides: Partial<Order> = {}): Order {
	const placed = {
		user: `0x${"00".repeat(12)}${"11".repeat(20)}`,
		source: "EVM-8453",
		destination: "EVM-56",
		deadline: 123_456_789n,
		nonce: 7n,
		fees: 42n,
		session: "0x0000000000000000000000000000000000000000",
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: `0x${"00".repeat(12)}${"22".repeat(20)}`, amount: 5_000_000n }],
		output: {
			beneficiary: `0x${"00".repeat(12)}${"33".repeat(20)}`,
			assets: [{ token: `0x${"00".repeat(12)}${"44".repeat(20)}`, amount: 99n * 10n ** 18n }],
			call: "0x",
		},
		...overrides,
	} as unknown as Order
	return { ...placed, id: orderCommitment(placed) }
}

/** A screener on a stubbed `screenOrder` that answers from `answers` in turn, with no backoff. */
function screener(answers: Array<ScreenOrderResult | Error>) {
	const signer = privateKeySigner(generatePrivateKey())
	const screenOrder = vi.fn(async () => {
		const next = answers.shift()
		if (!next) throw new Error("no answer queued")
		if (next instanceof Error) throw next
		return next
	})
	const screening = new OrderScreener({ screenOrder }, signer, async () => DOMAIN, undefined, [0, 0, 0])
	return { screening, screenOrder, signer }
}

const screened = (commitment: string, flagged: boolean): ScreenOrderResult => ({
	kind: "screened",
	commitment: commitment as HexString,
	flagged,
})

describe("which orders are screened", () => {
	it("screens a mainnet order and leaves one wholly on testnets alone", () => {
		expect(requiresScreening({ source: "EVM-8453", destination: "EVM-56" })).toBe(true)
		expect(requiresScreening({ source: "EVM-97", destination: "EVM-97" })).toBe(false)
		expect(requiresScreening({ source: "EVM-97", destination: "EVM-80002" })).toBe(false)
	})

	it("screens an order with a leg on a chain not listed as a testnet", () => {
		expect(requiresScreening({ source: "EVM-97", destination: "EVM-56" })).toBe(true)
		expect(requiresScreening({ source: "EVM-999999", destination: "EVM-97" })).toBe(true)
	})
})

describe("the order sent with a screening request", () => {
	it("is abi.encode(order), the bytes its commitment hashes", () => {
		const placed = order()
		const encoded = encodeOrder(placed)

		expect(encoded).toBeDefined()
		expect(keccak256(encoded!)).toBe(placed.id)
		// One dynamic tuple: its offset first, as Solidity's abi.encode(struct) and alloy's abi_encode write it.
		expect(encoded!.slice(0, 66)).toBe(`0x${"0".repeat(62)}20`)
	})

	it("is left off when it does not hash to the order's id", () => {
		expect(encodeOrder({ ...order(), id: `0x${"ab".repeat(32)}` })).toBeUndefined()
	})
})

describe("OrderbookClient.screenOrder", () => {
	afterEach(() => vi.restoreAllMocks())

	const params = {
		solver: `0x${"aa".repeat(20)}` as HexString,
		commitment: `0x${"ab".repeat(32)}` as HexString,
		timestamp: 1_800_000_000,
		signature: `0x${"cd".repeat(65)}` as HexString,
	}

	function answering(screenOrder: Record<string, unknown>) {
		return vi
			.spyOn(globalThis, "fetch")
			.mockResolvedValue(
				new Response(JSON.stringify({ data: { screenOrder } }), {
					headers: { "content-type": "application/json" },
				}),
			)
	}

	it("reads a verdict, and sends a null order when none is given", async () => {
		const fetchMock = answering({ __typename: "OrderScreened", commitment: params.commitment, verdict: "FLAGGED" })

		expect(await new OrderbookClient("http://ob", 5_000).screenOrder(params)).toEqual({
			kind: "screened",
			commitment: params.commitment,
			flagged: true,
		})
		const body = JSON.parse(String(fetchMock.mock.calls[0][1]?.body))
		expect(body.variables).toEqual({ ...params, order: null })
	})

	it("reads a rejected message and a failed screening", async () => {
		answering({ __typename: "MessageRejected", code: "NO_ACTIVE_ORDER", message: "none" })
		expect(await new OrderbookClient("http://ob", 5_000).screenOrder(params)).toEqual({
			kind: "rejected",
			code: "NO_ACTIVE_ORDER",
			message: "none",
		})

		vi.restoreAllMocks()
		answering({
			__typename: "ScreeningFailed",
			failureCode: "ORDER_NOT_ON_CHAIN",
			message: "not yet",
			retryable: true,
		})
		expect(await new OrderbookClient("http://ob", 5_000).screenOrder(params)).toEqual({
			kind: "failed",
			code: "ORDER_NOT_ON_CHAIN",
			message: "not yet",
			retryable: true,
		})
	})

	it("refuses a verdict it does not know rather than reading it as unflagged", async () => {
		answering({ __typename: "OrderScreened", commitment: params.commitment, verdict: "MAYBE" })

		await expect(new OrderbookClient("http://ob", 5_000).screenOrder(params)).rejects.toThrow(OrderbookRequestError)
	})
})

describe("OrderScreener", () => {
	it("signs ScreenOrder(solver, commitment, timestamp) as the orderbook verifies it", async () => {
		const placed = order()
		const { screening, screenOrder, signer } = screener([screened(placed.id!, false)])

		expect(await screening.screen(placed)).toEqual({ kind: "clear" })

		const [sent] = screenOrder.mock.calls[0] as unknown as [
			{ solver: HexString; commitment: HexString; timestamp: number; signature: HexString; order?: HexString },
		]
		expect(sent.solver).toBe(signer.address)
		expect(sent.commitment).toBe(placed.id)
		expect(sent.order).toBe(encodeOrder(placed))

		// The digest built by hand from the orderbook's definition: a domain of name and version
		// alone, and `ScreenOrder(address solver,bytes32 commitment,uint64 timestamp)`.
		const domainSeparator = keccak256(
			encodeAbiParameters(
				[{ type: "bytes32" }, { type: "bytes32" }, { type: "bytes32" }],
				[
					keccak256(toHex("EIP712Domain(string name,string version)")),
					keccak256(toHex(DOMAIN.name)),
					keccak256(toHex(DOMAIN.version)),
				],
			),
		)
		const structHash = keccak256(
			encodeAbiParameters(
				[{ type: "bytes32" }, { type: "address" }, { type: "bytes32" }, { type: "uint64" }],
				[
					keccak256(toHex("ScreenOrder(address solver,bytes32 commitment,uint64 timestamp)")),
					sent.solver,
					sent.commitment,
					BigInt(sent.timestamp),
				],
			),
		)
		const digest = keccak256(concat(["0x1901", domainSeparator, structHash]))
		expect(await recoverAddress({ hash: digest, signature: sent.signature })).toBe(signer.address)
	})

	it("answers flagged when the orderbook's verdict is FLAGGED", async () => {
		const placed = order()
		const { screening } = screener([screened(placed.id!, true)])

		expect(await screening.screen(placed)).toEqual({ kind: "flagged" })
	})

	it("signs each retry afresh, at a later timestamp, while the failure is retryable", async () => {
		const placed = order()
		const { screening, screenOrder } = screener([
			{ kind: "failed", code: "ORDER_NOT_ON_CHAIN", message: "not yet", retryable: true },
			{ kind: "rejected", code: "SIGNATURE_REUSED", message: "spent" },
			new OrderbookRequestError("timed out"),
			screened(placed.id!, false),
		])

		expect(await screening.screen(placed)).toEqual({ kind: "clear" })

		const sent = screenOrder.mock.calls.map(
			(call) => (call as unknown as [{ timestamp: number; signature: string }])[0],
		)
		expect(sent).toHaveLength(4)
		for (let i = 1; i < sent.length; i++) {
			expect(sent[i].timestamp).toBeGreaterThan(sent[i - 1].timestamp)
			expect(sent[i].signature).not.toBe(sent[i - 1].signature)
		}
	})

	it("gives up after its retries with the last reason", async () => {
		const placed = order()
		const unavailable = {
			kind: "failed",
			code: "PROVIDER_UNAVAILABLE",
			message: "no answer",
			retryable: true,
		} as const
		const { screening, screenOrder } = screener([unavailable, unavailable, unavailable, unavailable])

		expect(await screening.screen(placed)).toEqual({
			kind: "unavailable",
			code: "PROVIDER_UNAVAILABLE",
			message: "no answer",
		})
		expect(screenOrder).toHaveBeenCalledTimes(4)
	})

	it("gives up after its retries when the orderbook never answers", async () => {
		const placed = order()
		const down = new OrderbookRequestError("Could not reach the orderbook")
		const { screening, screenOrder } = screener([down, down, down, down])

		expect(await screening.screen(placed)).toMatchObject({ kind: "unavailable", code: "REQUEST_FAILED" })
		expect(screenOrder).toHaveBeenCalledTimes(4)
	})

	it.each([
		[
			"a server without screening",
			{ kind: "failed", code: "SCREENING_DISABLED", message: "off", retryable: false },
		],
		[
			"a party Chainalysis cannot screen",
			{ kind: "failed", code: "UNSCREENABLE_PARTY", message: "x", retryable: false },
		],
		["a solver with nothing on the book", { kind: "rejected", code: "NO_ACTIVE_ORDER", message: "none" }],
		["an unknown solver", { kind: "rejected", code: "UNKNOWN_SOLVER", message: "who" }],
	] as const)("does not retry %s", async (_case, answer) => {
		const placed = order()
		const { screening, screenOrder } = screener([answer as ScreenOrderResult])

		expect(await screening.screen(placed)).toEqual({
			kind: "unavailable",
			code: answer.code,
			message: answer.message,
		})
		expect(screenOrder).toHaveBeenCalledTimes(1)
	})

	it("does not take a verdict given for another order", async () => {
		const placed = order()
		const { screening } = screener([screened(`0x${"cd".repeat(32)}`, false)])

		expect(await screening.screen(placed)).toMatchObject({ kind: "unavailable", code: "COMMITMENT_MISMATCH" })
	})
})
