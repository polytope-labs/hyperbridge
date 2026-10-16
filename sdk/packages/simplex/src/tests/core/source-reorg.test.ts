import { describe, expect, it, vi } from "vitest"
import { Decimal } from "decimal.js"
import type { HexString, Order } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import type { FillerStrategy } from "@/strategies/base"
import { MemoryDataStore } from "@/data/memory"
import { stubOrderScanner } from "../helpers/stub-scanner"

/**
 * A source reorg can re-mine an `OrderPlaced` transaction in a different order,
 * under a different nonce, and so under a different commitment — while keeping
 * the same transaction hash. The confirmation wait counts depth against the
 * block the placement was scanned from, so the filler must only bid while a
 * quorum still places the transaction in that block, and must re-read the source
 * escrow under a quorum before bidding.
 */

const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString
const SCANNED_BLOCK = "0x" + "ab".repeat(32)
const OTHER_BLOCK = "0x" + "cd".repeat(32)
const GATEWAY = "0x" + "99".repeat(20)
const TOKEN_A = "0x" + "00".repeat(12) + "11".repeat(20)
const TOKEN_B = "0x" + "00".repeat(12) + "22".repeat(20)

/** One leg: an input paired with an output of the same token, as the gateway expects. */
function order(overrides: Partial<Order> = {}): Order {
	return {
		id: "0x" + "ab".repeat(32),
		user: "0xUSER",
		source: "EVM-137",
		destination: "EVM-8453",
		inputs: [{ token: TOKEN_A, amount: 1n }],
		output: { assets: [{ token: TOKEN_A, amount: 1n }] },
		...overrides,
	} as unknown as Order
}

function inclusion(overrides: Partial<{ blockHash: string; blockNumber: bigint; confirmations: bigint }> = {}) {
	return { blockHash: SCANNED_BLOCK, blockNumber: 100n, confirmations: 2n, ...overrides }
}

/** A `_orders` read result keyed by leg index. */
type ReadContract = (call: { args: [HexString, bigint] }) => Promise<bigint>

function build(options: { confirmationBlocks?: number; readContract?: ReadContract } = {}) {
	const strategy = {
		name: "stub",
		canFill: async () => true,
		confirmationPolicy: { getConfirmationBlocks: () => options.confirmationBlocks ?? 2 },
	} as unknown as FillerStrategy

	// The quorum's own endpoint client, so the escrow read really runs.
	const readContract = vi.fn(options.readContract ?? (async () => 5n))
	const endpointClient = { readContract }

	const getTransactionInclusion = vi.fn(async () => inclusion())
	const readWithQuorum = vi.fn(async (_label: string, read: (client: unknown) => Promise<unknown>) =>
		read(endpointClient),
	)
	const quorumStub = { size: 1, getTransactionInclusion, readWithQuorum }

	const filler = new IntentFiller(
		[{ chainId: 137 } as never, { chainId: 8453 } as never],
		[strategy],
		{ maxConcurrentOrders: 1 } as never,
		{
			loggers: undefined,
			getHyperbridgeWsUrl: () => undefined,
			getSubstratePrivateKey: () => undefined,
			getConfiguredChainIds: () => [137, 8453],
			isUserAllowed: () => true,
			getIntentGatewayAddress: () => GATEWAY,
		} as never,
		{
			getPublicClient: () => ({ readContract: async () => 5n }),
			getQuorumClient: () => quorumStub,
		} as never,
		{
			getCache: () => ({ getSolverSelection: () => false }),
			getInputUsdValue: async () => new Decimal(1000),
		} as never,
		{ address: OUR_ADDRESS } as never,
		{ orders: stubOrderScanner([137, 8453]) },
		undefined,
		new MemoryDataStore().bids,
	)

	// The evaluation is not what this file is about, but its result selects the
	// path: a null passes on the order before any skip reason here is emitted.
	const evaluateOrder = vi.spyOn(filler as any, "evaluateOrder").mockResolvedValue({ strategy, profitability: 1 })
	const executeOrder = vi.spyOn(filler as any, "executeOrder").mockImplementation(() => {})

	const skipped: Array<{ orderId?: string; reason?: string }> = []
	filler.monitor.on("orderSkipped", (event) => skipped.push(event))

	return { filler, skipped, executeOrder, evaluateOrder, getTransactionInclusion, readWithQuorum, readContract }
}

async function fill(filler: IntentFiller, placedOrder: Order, blockHash?: string) {
	;(filler as any).handleNewOrder(placedOrder, "0xtx", blockHash)
	await (filler as any).globalQueue.onIdle()
}

describe("source placement reorg during the confirmation wait", () => {
	it("fills when the quorum keeps the placement in the scanned block and the escrow holds", async () => {
		const { filler, skipped, executeOrder, getTransactionInclusion, readWithQuorum, readContract } = build()

		await fill(filler, order(), SCANNED_BLOCK)

		expect(skipped).toEqual([])
		expect(executeOrder).toHaveBeenCalledTimes(1)
		expect(readWithQuorum).toHaveBeenCalledTimes(1)
		// The re-read goes through the quorum's endpoint and asks for leg 0's escrow.
		expect(readContract).toHaveBeenCalledTimes(1)
		expect(readContract).toHaveBeenCalledWith(
			expect.objectContaining({ address: GATEWAY, functionName: "_orders", args: [order().id, 0n] }),
		)
		// The escrow is re-read on a quorum only after the last inclusion poll.
		const calls = getTransactionInclusion.mock.invocationCallOrder
		expect(readWithQuorum.mock.invocationCallOrder[0]).toBeGreaterThan(calls[calls.length - 1])
	})

	it("skips when the placement moves to another block mid-wait", async () => {
		const { filler, skipped, executeOrder, getTransactionInclusion, readWithQuorum } = build()
		getTransactionInclusion
			.mockResolvedValueOnce(inclusion({ blockHash: SCANNED_BLOCK, confirmations: 1n }))
			.mockResolvedValueOnce(inclusion({ blockHash: OTHER_BLOCK, blockNumber: 101n, confirmations: 3n }))

		await fill(filler, order(), SCANNED_BLOCK)

		expect(executeOrder).not.toHaveBeenCalled()
		expect(readWithQuorum).not.toHaveBeenCalled()
		expect(skipped).toEqual([{ orderId: order().id, reason: "Source placement reorged" }])
	})

	it("skips when the very first inclusion read is already in another block", async () => {
		const { filler, skipped, executeOrder, getTransactionInclusion, readWithQuorum } = build()
		getTransactionInclusion.mockResolvedValueOnce(
			inclusion({ blockHash: OTHER_BLOCK, blockNumber: 101n, confirmations: 3n }),
		)

		await fill(filler, order(), SCANNED_BLOCK)

		expect(getTransactionInclusion).toHaveBeenCalledTimes(1)
		expect(readWithQuorum).not.toHaveBeenCalled()
		expect(executeOrder).not.toHaveBeenCalled()
		expect(skipped).toEqual([{ orderId: order().id, reason: "Source placement reorged" }])
	})

	it("compares the block hash case-insensitively", async () => {
		const { filler, skipped, executeOrder } = build()

		await fill(filler, order(), "0x" + "Ab".repeat(32))

		expect(skipped).toEqual([])
		expect(executeOrder).toHaveBeenCalledTimes(1)
	})

	it("skips when the quorum's escrow read comes back empty after the wait", async () => {
		const { filler, skipped, executeOrder } = build({ readContract: async () => 0n })

		await fill(filler, order(), SCANNED_BLOCK)

		expect(executeOrder).not.toHaveBeenCalled()
		expect(skipped).toEqual([{ orderId: order().id, reason: "Source escrow missing after confirmations" }])
	})

	it("skips a multi-leg order when one leg's escrow is gone, after reading every leg", async () => {
		const twoLeg = order({
			inputs: [
				{ token: TOKEN_A, amount: 1n },
				{ token: TOKEN_B, amount: 1n },
			],
			output: {
				assets: [
					{ token: TOKEN_A, amount: 1n },
					{ token: TOKEN_B, amount: 1n },
				],
			},
		} as never)
		const { filler, skipped, executeOrder, readContract } = build({
			readContract: async ({ args }) => (args[1] === 1n ? 0n : 5n),
		})

		await fill(filler, twoLeg, SCANNED_BLOCK)

		expect(executeOrder).not.toHaveBeenCalled()
		expect(skipped).toEqual([{ orderId: twoLeg.id, reason: "Source escrow missing after confirmations" }])
		// Every leg is read, so the quorum's vote covers the whole escrow vector.
		expect(readContract.mock.calls.map(([call]) => call.args[1])).toEqual([0n, 1n])
	})

	it("fails closed when the quorum's escrow read rejects", async () => {
		const { filler, skipped, executeOrder, readWithQuorum } = build()
		readWithQuorum.mockRejectedValue(new Error("quorum down"))

		await fill(filler, order(), SCANNED_BLOCK)

		expect(executeOrder).not.toHaveBeenCalled()
		expect(skipped).toEqual([{ orderId: order().id, reason: "Source escrow missing after confirmations" }])
	})

	it("fails closed when the scanner reported no placement block", async () => {
		const { filler, skipped, executeOrder, getTransactionInclusion } = build()

		await fill(filler, order(), undefined)

		expect(getTransactionInclusion).not.toHaveBeenCalled()
		expect(executeOrder).not.toHaveBeenCalled()
		expect(skipped).toEqual([{ orderId: order().id, reason: "Placement block unknown" }])
	})

	it("does not gate a same-chain order on the placement block or a quorum read", async () => {
		// source === destination, so requiredConfirmations is 0: the fill reverts
		// on-chain if the order is gone, and no quorum read should be made.
		const { filler, skipped, executeOrder, getTransactionInclusion, readWithQuorum } = build()

		await fill(filler, order({ destination: "EVM-137" }), undefined)

		expect(skipped).toEqual([])
		expect(executeOrder).toHaveBeenCalledTimes(1)
		expect(getTransactionInclusion).not.toHaveBeenCalled()
		expect(readWithQuorum).not.toHaveBeenCalled()
	})

	it("does not gate a cross-chain order whose policy requires zero confirmations", async () => {
		const { filler, skipped, executeOrder, getTransactionInclusion, readWithQuorum } = build({
			confirmationBlocks: 0,
		})

		await fill(filler, order(), undefined)

		expect(skipped).toEqual([])
		expect(executeOrder).toHaveBeenCalledTimes(1)
		expect(getTransactionInclusion).not.toHaveBeenCalled()
		expect(readWithQuorum).not.toHaveBeenCalled()
	})

	it("reports no reorg skip when evaluation already passed on the order", async () => {
		const { filler, skipped, executeOrder, evaluateOrder, getTransactionInclusion } = build()
		evaluateOrder.mockResolvedValue(null)
		getTransactionInclusion.mockResolvedValueOnce(inclusion({ blockHash: OTHER_BLOCK, blockNumber: 101n }))

		await fill(filler, order(), SCANNED_BLOCK)

		expect(executeOrder).not.toHaveBeenCalled()
		// Evaluation reports its own skip; the reorg outcome is not surfaced again.
		expect(skipped).toEqual([])
	})
})
