import { describe, expect, it, vi } from "vitest"
import { Decimal } from "decimal.js"
import type { HexString, Order } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import type { OrderScreening, ScreeningDecision, ScreenOptions } from "@/orderbook/screening"
import type { FillerStrategy } from "@/strategies/base"
import { MemoryDataStore } from "@/data/memory"
import { stubOrderScanner } from "../helpers/stub-scanner"

/**
 * Every mainnet order is screened through the orderbook before it is bid on. A flagged order, and
 * one that could not be screened, is dropped; an order wholly on testnets is never screened.
 */

const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString
const SCANNED_BLOCK = "0x" + "ab".repeat(32)
const GATEWAY = "0x" + "99".repeat(20)
const TOKEN = "0x" + "00".repeat(12) + "11".repeat(20)

function order(source: string, destination: string): Order {
	return {
		id: "0x" + "ab".repeat(32),
		user: "0xUSER",
		source,
		destination,
		inputs: [{ token: TOKEN, amount: 1n }],
		output: { beneficiary: "0xBENEFICIARY", assets: [{ token: TOKEN, amount: 1n }] },
	} as unknown as Order
}

interface Build {
	chains?: number[]
	confirmations?: { required: number; reached: bigint }
	/** The block the quorum places the placement in; another than the scanned one is a reorg. */
	inclusionBlock?: string
	canFill?: boolean
	screener?: OrderScreening | null
}

function screener(decision: ScreeningDecision | (() => Promise<ScreeningDecision>)) {
	return { screen: vi.fn(typeof decision === "function" ? decision : async () => decision) }
}

function build(options: Build = {}) {
	const chains = options.chains ?? [137, 8453]
	const strategy = {
		name: "stub",
		canFill: async () => options.canFill ?? true,
		confirmationPolicy: { getConfirmationBlocks: () => options.confirmations?.required ?? 0 },
	} as unknown as FillerStrategy

	const quorumStub = {
		size: 1,
		getTransactionInclusion: vi.fn(async () => ({
			blockHash: options.inclusionBlock ?? SCANNED_BLOCK,
			blockNumber: 100n,
			confirmations: options.confirmations?.reached ?? 0n,
		})),
		readWithQuorum: vi.fn(async (_label: string, read: (client: unknown) => Promise<unknown>) =>
			read({ readContract: async () => 5n }),
		),
	}

	const filler = new IntentFiller(
		chains.map((chainId) => ({ chainId }) as never),
		[strategy],
		{ maxConcurrentOrders: 1 } as never,
		{
			loggers: undefined,
			getHyperbridgeWsUrl: () => undefined,
			getSubstratePrivateKey: () => undefined,
			getConfiguredChainIds: () => chains,
			isUserAllowed: () => true,
			getIntentGatewayAddress: () => GATEWAY,
		} as never,
		{
			getPublicClient: () => ({ readContract: async () => 5n }),
			getQuorumClient: () => quorumStub,
		} as never,
		{
			cacheService: { extendForConfirmations: () => () => {} },
			blockTimeSeconds: async () => 2,
			getCache: () => ({ getSolverSelection: () => false }),
			getInputUsdValue: async () => new Decimal(1000),
		} as never,
		{ address: OUR_ADDRESS } as never,
		{ orders: stubOrderScanner(chains) },
		undefined,
		new MemoryDataStore().bids,
	)
	const screening = options.screener === undefined ? screener({ kind: "clear" }) : options.screener
	if (screening) filler.setScreener(screening)

	const evaluateOrder = vi.spyOn(filler as any, "evaluateOrder").mockResolvedValue({ strategy, profitability: 1 })
	const executeOrder = vi.spyOn(filler as any, "executeOrder").mockImplementation(() => {})

	const skipped: Array<{ orderId?: string; reason?: string }> = []
	filler.monitor.on("orderSkipped", (event) => skipped.push(event))

	return { filler, skipped, executeOrder, evaluateOrder }
}

async function handle(filler: IntentFiller, placed: Order) {
	;(filler as any).handleNewOrder(placed, "0xtx", SCANNED_BLOCK)
	await (filler as any).globalQueue.onIdle()
}

describe("screening a mainnet order", () => {
	it("bids on an order screening clears", async () => {
		const clear = screener({ kind: "clear" })
		const { filler, skipped, executeOrder } = build({ screener: clear })
		const placed = order("EVM-137", "EVM-8453")

		await handle(filler, placed)

		expect(clear.screen).toHaveBeenCalledWith(placed, expect.objectContaining({ budgetMs: 0 }))
		expect(skipped).toEqual([])
		expect(executeOrder).toHaveBeenCalledTimes(1)
	})

	it("drops an order screening flags", async () => {
		const { filler, skipped, executeOrder } = build({ screener: screener({ kind: "flagged" }) })

		await handle(filler, order("EVM-137", "EVM-8453"))

		expect(executeOrder).not.toHaveBeenCalled()
		expect(skipped).toEqual([{ orderId: order("", "").id, reason: "Screening flagged the order" }])
	})

	it("drops an order that could not be screened", async () => {
		const unavailable = screener({ kind: "unavailable", code: "PROVIDER_UNAVAILABLE", message: "no answer" })
		const { filler, skipped, executeOrder } = build({ screener: unavailable })

		await handle(filler, order("EVM-137", "EVM-8453"))

		expect(executeOrder).not.toHaveBeenCalled()
		expect(skipped).toEqual([{ orderId: order("", "").id, reason: "Screening unavailable: PROVIDER_UNAVAILABLE" }])
	})

	it("drops an order when the screener throws", async () => {
		const throwing = screener(async () => {
			throw new Error("signer offline")
		})
		const { filler, skipped, executeOrder } = build({ screener: throwing })

		await handle(filler, order("EVM-8453", "EVM-8453"))

		expect(executeOrder).not.toHaveBeenCalled()
		expect(skipped).toEqual([{ orderId: order("", "").id, reason: "Screening unavailable: SCREENER_ERROR" }])
	})

	it("drops every mainnet order when no screener was set", async () => {
		const { filler, skipped, executeOrder } = build({ screener: null })

		await handle(filler, order("EVM-137", "EVM-8453"))

		expect(executeOrder).not.toHaveBeenCalled()
		expect(skipped).toEqual([{ orderId: order("", "").id, reason: "Screening unavailable: no screener" }])
	})

	it("ends the confirmation wait as soon as screening drops the order", async () => {
		// Never confirmed: without the abort, the wait would poll until the test timed out.
		const { filler, skipped, executeOrder } = build({
			screener: screener({ kind: "flagged" }),
			confirmations: { required: 1_000, reached: 1n },
		})

		await handle(filler, order("EVM-137", "EVM-8453"))

		expect(executeOrder).not.toHaveBeenCalled()
		expect(skipped).toEqual([{ orderId: order("", "").id, reason: "Screening flagged the order" }])
	})

	it("waits for screening to finish before bidding", async () => {
		let release: (decision: ScreeningDecision) => void = () => {}
		const slow = screener(() => new Promise<ScreeningDecision>((resolve) => (release = resolve)))
		const { filler, executeOrder } = build({ screener: slow })

		const done = handle(filler, order("EVM-137", "EVM-8453"))
		await vi.waitFor(() => expect(slow.screen).toHaveBeenCalled())
		await new Promise((resolve) => setTimeout(resolve, 20))
		expect(executeOrder).not.toHaveBeenCalled()

		release({ kind: "clear" })
		await done
		expect(executeOrder).toHaveBeenCalledTimes(1)
	})

	it("does not screen an order no strategy can fill", async () => {
		const clear = screener({ kind: "clear" })
		const { filler, evaluateOrder } = build({ screener: clear, canFill: false })
		evaluateOrder.mockResolvedValue(null)

		await handle(filler, order("EVM-8453", "EVM-8453"))

		expect(clear.screen).not.toHaveBeenCalled()
	})

	it("does not screen an order the evaluation would not bid on", async () => {
		// Each screen is a signature, a request, and a billed analysis of every new address.
		const clear = screener({ kind: "clear" })
		const { filler, evaluateOrder, executeOrder } = build({ screener: clear })
		evaluateOrder.mockResolvedValue(null)

		await handle(filler, order("EVM-137", "EVM-8453"))

		expect(clear.screen).not.toHaveBeenCalled()
		expect(executeOrder).not.toHaveBeenCalled()
	})

	it("lets the screen keep asking for as long as the order's confirmations take", async () => {
		// 60 confirmations of 2 s blocks: two minutes the order waits whatever screening does.
		const clear = screener({ kind: "clear" })
		const { filler, executeOrder } = build({ screener: clear, confirmations: { required: 60, reached: 60n } })

		await handle(filler, order("EVM-137", "EVM-8453"))

		expect(clear.screen).toHaveBeenCalledWith(expect.anything(), expect.objectContaining({ budgetMs: 120_000 }))
		expect(executeOrder).toHaveBeenCalledTimes(1)
	})

	it("stops the screen's retries once the order is done with", async () => {
		let signal: AbortSignal | undefined
		const capturing = {
			screen: vi.fn(async (_order: Order, options?: ScreenOptions) => {
				signal = options?.signal
				return { kind: "clear" } as const
			}),
		}
		const { filler } = build({ screener: capturing })

		await handle(filler, order("EVM-137", "EVM-8453"))

		expect(signal?.aborted).toBe(true)
	})

	it("reports a reorged placement without waiting on a screen still in progress", async () => {
		const pending = screener(() => new Promise<ScreeningDecision>(() => {}))
		const { filler, skipped, executeOrder } = build({
			screener: pending,
			confirmations: { required: 1, reached: 5n },
			inclusionBlock: "0x" + "cd".repeat(32),
		})

		await handle(filler, order("EVM-137", "EVM-8453"))

		expect(executeOrder).not.toHaveBeenCalled()
		expect(skipped).toEqual([{ orderId: order("", "").id, reason: "Source placement reorged" }])
	})
})

describe("an order wholly on testnets", () => {
	it("is bid on without being screened", async () => {
		const clear = screener({ kind: "clear" })
		const { filler, skipped, executeOrder } = build({ chains: [97, 80002], screener: clear })

		await handle(filler, order("EVM-97", "EVM-80002"))

		expect(clear.screen).not.toHaveBeenCalled()
		expect(skipped).toEqual([])
		expect(executeOrder).toHaveBeenCalledTimes(1)
	})

	it("needs no screener at all", async () => {
		const { filler, executeOrder } = build({ chains: [97], screener: null })

		await handle(filler, order("EVM-97", "EVM-97"))

		expect(executeOrder).toHaveBeenCalledTimes(1)
	})
})
