import { describe, expect, it } from "vitest"
import { acceptedSourceChainsFor } from "@/core/filler"

// The declaration a phantom bid carries is derived, not configured: every configured chain, in a
// fixed order. Watch-only is a destination-side flag and does not narrow it.
describe("acceptedSourceChainsFor", () => {
	it("declares every configured chain as a state machine id", () => {
		expect(acceptedSourceChainsFor([8453, 56, 137])).toEqual(["EVM-56", "EVM-137", "EVM-8453"])
	})

	it("declares the empty set when no chain is configured", () => {
		expect(acceptedSourceChainsFor([])).toEqual([])
	})

	it("is deterministic in the face of input order and duplicates", () => {
		expect(acceptedSourceChainsFor([137, 8453, 137, 56])).toEqual(acceptedSourceChainsFor([56, 137, 8453]))
	})
})
