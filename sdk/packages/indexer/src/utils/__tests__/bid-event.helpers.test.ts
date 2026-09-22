import { bidPlacedCarriesBidId } from "@/utils/bid-event.helpers"

// Re-indexing starts before the upgrade that gave bids an identifier. Those older events decode
// against their own metadata as (filler, commitment, deposit), and reading their third field as the
// identifier would file every one of them under its deposit.
describe("bidPlacedCarriesBidId", () => {
	it("accepts an event carrying (filler, commitment, bid, deposit)", () => {
		expect(bidPlacedCarriesBidId(["filler", "commitment", "bid", "deposit"])).toBe(true)
	})

	it("rejects a pre-upgrade event carrying (filler, commitment, deposit)", () => {
		expect(bidPlacedCarriesBidId(["filler", "commitment", "deposit"])).toBe(false)
	})
})
