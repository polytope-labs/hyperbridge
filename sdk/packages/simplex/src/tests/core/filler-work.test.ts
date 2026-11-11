import { describe, expect, it, vi } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import { stubOrderScanner } from "../helpers/stub-scanner"

/** The work a running filler reports, which the desktop waits on before it replaces the solver. */

const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString

describe("IntentFiller work", () => {
	function build(rebalancingService?: { rebalancePortfolio(): Promise<unknown> }) {
		const filler = new IntentFiller(
			[],
			[],
			{},
			{} as any,
			{} as any, // ChainClientManager — unused with no chains configured
			{} as any, // ContractInteractionService — unused here
			{ address: OUR_ADDRESS } as any,
			{ orders: stubOrderScanner() },
			rebalancingService as any,
			new MemoryDataStore().bids,
		)
		return { filler }
	}

	it("reports the work stop drains", () => {
		const { filler } = build()
		;(filler as any).globalQueue = { size: 2, pending: 1 }
		;(filler as any).chainQueues = new Map([
			[1, { size: 3, pending: 1 }],
			[2, { size: 4, pending: 2 }],
		])

		expect(filler.getWorkSnapshot()).toEqual({
			queuedEvaluations: 2,
			evaluating: 1,
			queuedFills: 7,
			activeFills: 3,
			rebalancing: 0,
		})
	})

	it("reports and drains a rebalance already in flight before stopping", async () => {
		vi.useFakeTimers()
		let finishRebalance!: (value: { success: boolean; transfers: never[]; executedTransfers: never[] }) => void
		const rebalancePortfolio = vi.fn(
			() =>
				new Promise<{ success: boolean; transfers: never[]; executedTransfers: never[] }>((resolve) => {
					finishRebalance = resolve
				}),
		)
		const { filler } = build({ rebalancePortfolio })
		try {
			filler.start()
			await vi.advanceTimersByTimeAsync(30_000)
			expect(rebalancePortfolio).toHaveBeenCalledOnce()
			expect(filler.getWorkSnapshot().rebalancing).toBe(1)

			let stopped = false
			const stop = filler.stop().then(() => {
				stopped = true
			})
			await Promise.resolve()
			expect(stopped).toBe(false)

			finishRebalance({ success: true, transfers: [], executedTransfers: [] })
			await stop
			expect(filler.getWorkSnapshot().rebalancing).toBe(0)
		} finally {
			vi.useRealTimers()
		}
	})

	it("does not start a second rebalance while one is still running", async () => {
		vi.useFakeTimers()
		let finishRebalance!: (value: { success: boolean; transfers: never[]; executedTransfers: never[] }) => void
		const rebalancePortfolio = vi.fn(
			() =>
				new Promise<{ success: boolean; transfers: never[]; executedTransfers: never[] }>((resolve) => {
					finishRebalance = resolve
				}),
		)
		const { filler } = build({ rebalancePortfolio })
		try {
			filler.start()
			await vi.advanceTimersByTimeAsync(30_000)
			expect(rebalancePortfolio).toHaveBeenCalledTimes(1)

			// A CEX leg can run for half an hour; the 5-minute ticks must not stack
			// behind it and plan the same deficit twice.
			await vi.advanceTimersByTimeAsync(5 * 60 * 1000)
			await vi.advanceTimersByTimeAsync(5 * 60 * 1000)
			expect(rebalancePortfolio).toHaveBeenCalledTimes(1)

			finishRebalance({ success: true, transfers: [], executedTransfers: [] })
			await vi.advanceTimersByTimeAsync(5 * 60 * 1000)
			expect(rebalancePortfolio).toHaveBeenCalledTimes(2)
		} finally {
			vi.useRealTimers()
		}
	})
})
