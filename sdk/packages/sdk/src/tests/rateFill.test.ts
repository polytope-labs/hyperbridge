import { previewRateFill } from "@/protocols/intents/rateFill"
import { describe, expect, it } from "vitest"

describe("previewRateFill", () => {
	it.each([
		[10n, 3n, 0n, 4n, 4n, { credit: 1n, release: 3n, delivered: 3n, surplus: 2n }],
		[3n, 10n, 0n, 2n, 7n, { credit: 6n, release: 1n, delivered: 6n, surplus: 0n }],
		[10n, 3n, 0n, 11n, 11n, { credit: 3n, release: 10n, delivered: 10n, surplus: 7n }],
		[1_000n, 1_000n, 0n, 400n, 440n, { credit: 400n, release: 400n, delivered: 440n, surplus: 40n }],
		[1_000n, 1_000n, 800n, 500n, 550n, { credit: 200n, release: 200n, delivered: 220n, surplus: 20n }],
		[10n, 3n, 0n, 4n, 2n, { credit: 1n, release: 3n, delivered: 2n, surplus: 1n }],
		[10n, 3n, 1n, 10n, 3n, { credit: 2n, release: 7n, delivered: 3n, surplus: 1n }],
	])("matches settlement arithmetic for %#", (escrow, required, filled, take, offered, expected) => {
		expect(previewRateFill(escrow, required, filled, take, offered)).toEqual(expected)
	})

	it("rejects a quote below the order rate", () => {
		expect(() => previewRateFill(1_000n, 1_000n, 0n, 400n, 399n)).toThrow("RateBelowOrder")
	})

	it("rejects positive quotes that round to zero progress", () => {
		expect(() => previewRateFill(10n, 3n, 0n, 1n, 1n)).toThrow("RateFillNoProgress")
	})

	it("rejects zero values and invalid existing progress", () => {
		const cases: [bigint, bigint, bigint, bigint, bigint][] = [
			[0n, 1n, 0n, 1n, 1n],
			[1n, 0n, 0n, 1n, 1n],
			[1n, 1n, 0n, 0n, 1n],
			[1n, 1n, 0n, 1n, 0n],
			[1n, 1n, 2n, 1n, 1n],
		]
		for (const args of cases) {
			expect(() => previewRateFill(...args)).toThrow()
		}
	})
})
