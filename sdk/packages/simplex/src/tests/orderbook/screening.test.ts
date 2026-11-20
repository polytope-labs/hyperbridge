import { afterEach, describe, expect, it, vi } from "vitest"
import { concat, encodeAbiParameters, keccak256, recoverAddress, toHex } from "viem"
import { generatePrivateKey } from "viem/accounts"
import { type HexString, type Order, orderCommitment } from "@hyperbridge/sdk"
import {
	assertOrderEncoding,
	encodeOrder,
	ORDER_ENCODING_VECTOR,
	OrderScreener,
	reportScreeningStatus,
	requiresScreening,
	SCREENING_RETRY,
	type ScreenOptions,
} from "@/orderbook/screening"
import type { ScreenOrderResult } from "@/orderbook/types"
import {
	OrderbookClient,
	OrderbookGraphqlError,
	OrderbookRequestError,
	SCREEN_ORDER_TIMEOUT_MS,
} from "@/orderbook/client"
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

/**
 * A screener on a stubbed `screenOrder` that answers from `answers` in turn, repeating the last one
 * once they run out, on a clock that only moves when the screener waits. `waits` records each wait.
 */
function screener(answers: Array<ScreenOrderResult | Error>, onCall?: (call: number) => void) {
	const signer = privateKeySigner(generatePrivateKey())
	let call = 0
	const screenOrder = vi.fn(async () => {
		onCall?.(call)
		const next = answers[Math.min(call++, answers.length - 1)]
		if (next instanceof Error) throw next
		return next
	})
	const clock = { at: 1_800_000_000_000, waits: [] as number[] }
	const screening = new OrderScreener({ screenOrder }, signer, async () => DOMAIN, undefined, SCREENING_RETRY, {
		now: () => clock.at,
		sleep: async (ms) => {
			clock.waits.push(ms)
			clock.at += ms
		},
	})
	return { screening, screenOrder, signer, clock }
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

	it("reproduces the commitment Foundry's cast computes for an order with every field set", () => {
		const { order: reference, commitment } = ORDER_ENCODING_VECTOR

		expect(() => assertOrderEncoding()).not.toThrow()
		expect(keccak256(encodeOrder({ ...reference, id: commitment })!)).toBe(commitment)
		// The SDK's own commitment agrees, so the order's id and the bytes sent with it cannot drift apart.
		expect(orderCommitment(reference)).toBe(commitment)
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
		return vi.spyOn(globalThis, "fetch").mockResolvedValue(
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

	it("waits longer than the orderbook's own 30 s cut-off, whatever shorter timeout is configured", async () => {
		answering({ __typename: "OrderScreened", commitment: params.commitment, verdict: "UNFLAGGED" })
		const timers = vi.spyOn(globalThis, "setTimeout")

		await new OrderbookClient("http://ob", 5_000).screenOrder(params)

		expect(timers.mock.calls.map(([, ms]) => ms)).toContain(SCREEN_ORDER_TIMEOUT_MS)
		expect(SCREEN_ORDER_TIMEOUT_MS).toBeGreaterThan(30_000)
	})

	it("throws a GraphQL error as one, still an OrderbookRequestError to every other caller", async () => {
		vi.spyOn(globalThis, "fetch").mockResolvedValue(
			new Response(JSON.stringify({ errors: [{ message: 'Unknown field "screenOrder" on type "Mutation"' }] }), {
				headers: { "content-type": "application/json" },
			}),
		)

		const refused = new OrderbookClient("http://ob", 5_000).screenOrder(params)
		await expect(refused).rejects.toThrow(OrderbookGraphqlError)
		await expect(refused).rejects.toThrow(OrderbookRequestError)
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

	it("keeps asking through an outage for a minute, backing off to 8 s, then gives the last reason", async () => {
		const placed = order()
		const unavailable = {
			kind: "failed",
			code: "PROVIDER_UNAVAILABLE",
			message: "no answer",
			retryable: true,
		} as const
		const { screening, screenOrder, clock } = screener([unavailable])

		expect(await screening.screen(placed)).toEqual({
			kind: "unavailable",
			code: "PROVIDER_UNAVAILABLE",
			message: "no answer",
		})
		expect(clock.waits).toEqual([500, 1_000, 2_000, 4_000, 8_000, 8_000, 8_000, 8_000, 8_000, 8_000])
		expect(screenOrder).toHaveBeenCalledTimes(11)
		// The last attempt goes out inside the minute; the next would not have.
		expect(clock.waits.reduce((sum, ms) => sum + ms, 0)).toBeLessThanOrEqual(SCREENING_RETRY.minBudgetMs)
	})

	it("keeps asking for as long as the order's confirmations take, when that is longer", async () => {
		const placed = order()
		const down = new OrderbookRequestError("Could not reach the orderbook")
		const { screening, screenOrder, clock } = screener([down])

		expect(await screening.screen(placed, { budgetMs: 120_000 })).toMatchObject({ code: "REQUEST_FAILED" })
		expect(screenOrder).toHaveBeenCalledTimes(19)
		expect(clock.waits.reduce((sum, ms) => sum + ms, 0)).toBeLessThanOrEqual(120_000)
	})

	it("gets its verdict once the orderbook answers again", async () => {
		const placed = order()
		const down = new OrderbookRequestError("HTTP 503")
		const { screening, screenOrder } = screener([down, down, down, screened(placed.id!, false)])

		expect(await screening.screen(placed)).toEqual({ kind: "clear" })
		expect(screenOrder).toHaveBeenCalledTimes(4)
	})

	it("asks again when the solver had no active order, as between withdrawing a limit order and reposting it", async () => {
		const placed = order()
		const { screening, screenOrder } = screener([
			{ kind: "rejected", code: "NO_ACTIVE_ORDER", message: "none" },
			screened(placed.id!, true),
		])

		expect(await screening.screen(placed)).toEqual({ kind: "flagged" })
		expect(screenOrder).toHaveBeenCalledTimes(2)
	})

	it("does not retry a request the orderbook refused as written", async () => {
		const placed = order()
		const { screening, screenOrder } = screener([new OrderbookGraphqlError('Unknown field "screenOrder"')])

		expect(await screening.screen(placed)).toEqual({
			kind: "unavailable",
			code: "GRAPHQL_ERROR",
			message: 'Unknown field "screenOrder"',
		})
		expect(screenOrder).toHaveBeenCalledTimes(1)
	})

	it("stops asking once the caller no longer needs a verdict", async () => {
		const placed = order()
		const abort = new AbortController()
		const options: ScreenOptions = { signal: abort.signal }
		const { screening, screenOrder } = screener([new OrderbookRequestError("down")], () => abort.abort())

		expect(await screening.screen(placed, options)).toMatchObject({ kind: "unavailable", code: "ABORTED" })
		expect(screenOrder).toHaveBeenCalledTimes(1)
	})

	it.each([
		[
			"a server without screening",
			{ kind: "failed", code: "SCREENING_DISABLED", message: "off", retryable: false },
		],
		[
			"a party Elliptic cannot screen",
			{ kind: "failed", code: "UNSCREENABLE_PARTY", message: "x", retryable: false },
		],
		["an unknown solver", { kind: "rejected", code: "UNKNOWN_SOLVER", message: "who" }],
		["a signature that does not recover", { kind: "rejected", code: "BAD_SIGNATURE", message: "bad" }],
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

describe("the screening status reported at boot", () => {
	const logger = () => ({ error: vi.fn(), warn: vi.fn(), info: vi.fn() })

	it("says nothing is wrong when the orderbook screens", async () => {
		const log = logger()
		await reportScreeningStatus({ screeningEnabled: async () => true }, ["EVM-8453"], log)
		expect(log.error).not.toHaveBeenCalled()
		expect(log.info).toHaveBeenCalledTimes(1)
	})

	it("is an error when the orderbook has screening off, or predates it", async () => {
		const off = logger()
		await reportScreeningStatus({ screeningEnabled: async () => false }, ["EVM-8453"], off)
		expect(off.error).toHaveBeenCalledTimes(1)

		const old = logger()
		await reportScreeningStatus(
			{
				screeningEnabled: async () => {
					throw new OrderbookGraphqlError('Unknown field "screeningEnabled"')
				},
			},
			["EVM-8453"],
			old,
		)
		expect(old.error).toHaveBeenCalledTimes(1)
	})

	it("only warns when the orderbook could not be asked", async () => {
		const log = logger()
		await reportScreeningStatus(
			{
				screeningEnabled: async () => {
					throw new OrderbookRequestError("down")
				},
			},
			["EVM-8453"],
			log,
		)
		expect(log.error).not.toHaveBeenCalled()
		expect(log.warn).toHaveBeenCalledTimes(1)
	})

	it("does not ask for a filler on testnets alone, which screens nothing", async () => {
		const screeningEnabled = vi.fn(async () => false)
		const log = logger()
		await reportScreeningStatus({ screeningEnabled }, ["EVM-97", "EVM-80002"], log)
		expect(screeningEnabled).not.toHaveBeenCalled()
		expect(log.error).not.toHaveBeenCalled()
	})
})
