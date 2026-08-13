import { describe, expect, it } from "vitest"
import { ConfirmationPolicy } from "@/config/interpolated-curve"

/**
 * Runtime chain edits touch two things that are easy to get silently wrong: the
 * confirmation curve the engine prices reorg risk with, and whether a filler the
 * operator put in watch-only stays that way.
 */

const CURVE = {
	points: [
		{ amount: "1000", value: 2 },
		{ amount: "100000", value: 9 },
	],
}

describe("ConfirmationPolicy runtime installation", () => {
	it("adopts a built-in default across from another policy set", () => {
		// A chain added at runtime that a built-in default covers: the defaults are
		// only materialised inside a freshly constructed policy, so the live one has
		// to take the entry rather than re-derive it. Without this the engine has no
		// curve and every cross-chain order sourced there throws per order.
		const live = new ConfirmationPolicy({ "1": CURVE })
		const resolved = new ConfirmationPolicy({ "1": CURVE, "8453": CURVE })

		expect(live.has(8453)).toBe(false)
		live.adopt(8453, resolved)
		expect(live.has(8453)).toBe(true)
		expect(() => live.assertCovers([1, 8453])).not.toThrow()
	})

	it("ignores an adopt for a chain the source does not cover", () => {
		const live = new ConfirmationPolicy({ "1": CURVE })
		live.adopt(999, new ConfirmationPolicy({ "1": CURVE }))
		expect(live.has(999)).toBe(false)
	})

	it("prices with the adopted curve, not a placeholder", () => {
		const live = new ConfirmationPolicy({ "1": CURVE })
		live.adopt(8453, new ConfirmationPolicy({ "8453": CURVE }))
		// biome-ignore lint/suspicious/noExplicitAny: Decimal-like is all getValue needs
		expect(live.getConfirmationBlocks(8453, { toNumber: () => 100000 } as any)).toBe(9)
	})
})
