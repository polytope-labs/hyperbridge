import { describe, expect, it } from "vitest"
import { acceptedSourceChainsFor } from "@/core/filler"

// The declaration a phantom bid carries is derived, not configured: every chain the filler fills
// on, which is every configured chain that is not watch-only.
describe("acceptedSourceChainsFor", () => {
	it("declares every configured chain when none is watch-only", () => {
		expect(acceptedSourceChainsFor([8453, 56, 137], undefined)).toEqual(["EVM-56", "EVM-137", "EVM-8453"])
		expect(acceptedSourceChainsFor([8453, 56], {})).toEqual(["EVM-56", "EVM-8453"])
	})

	it("leaves out a chain marked watch-only, and keeps one marked false", () => {
		expect(acceptedSourceChainsFor([8453, 56, 137], { 56: true, 137: false })).toEqual(["EVM-137", "EVM-8453"])
	})

	it("declares the empty set when every chain is watch-only", () => {
		expect(acceptedSourceChainsFor([8453, 56], { 8453: true, 56: true })).toEqual([])
	})

	it("ignores a watch-only flag for a chain that is not configured", () => {
		expect(acceptedSourceChainsFor([8453], { 1: true })).toEqual(["EVM-8453"])
	})

	it("is deterministic in the face of input order and duplicates", () => {
		expect(acceptedSourceChainsFor([137, 8453, 137, 56], undefined)).toEqual(
			acceptedSourceChainsFor([56, 137, 8453], undefined),
		)
	})
})
