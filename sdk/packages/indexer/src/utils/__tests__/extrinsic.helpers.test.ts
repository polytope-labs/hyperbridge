import { SubstrateExtrinsic } from "@subql/types"
import { extractUserOpFromExtrinsic } from "@/utils/extrinsic.helpers"

// The user operation is read out of the place_bid extrinsic rather than intents_getBidsForOrder,
// because that RPC pins the chain head internally and the pallet's offchain copy expires — so on any
// backfill the extrinsic is the only source that still has the bid. place_bid can arrive wrapped in
// batch/proxy/sudo, so the walk has to find it at depth, and has to match on commitment: one batch
// may carry bids for several orders, and picking the wrong one would attribute another order's quote
// to this event.

const COMMITMENT = `0x${"11".repeat(32)}`
const OTHER_COMMITMENT = `0x${"22".repeat(32)}`
const USER_OP = "0xdeadbeef"
const OTHER_USER_OP = "0xfeedface"

const hex = (value: string) => ({ toHex: () => value })

const BID = `0x${"b1".repeat(32)}`
const OTHER_BID = `0x${"b2".repeat(32)}`

const placeBid = (commitment: string, userOp: string, bid = BID) => ({
	section: "intentsCoprocessor",
	method: "placeBid",
	args: [hex(commitment), hex(bid), hex(userOp)],
})

const batch = (calls: unknown[]) => ({
	section: "utility",
	method: "batchAll",
	args: [calls],
})

const proxy = (call: unknown) => ({
	section: "proxy",
	method: "proxy",
	args: [hex("0xaaaa"), hex("0x00"), call],
})

const asExtrinsic = (method: unknown) => ({ extrinsic: { method } }) as unknown as SubstrateExtrinsic


describe("extractUserOpFromExtrinsic", () => {
	it("reads the user op from a direct place_bid call", () => {
		expect(extractUserOpFromExtrinsic(asExtrinsic(placeBid(COMMITMENT, USER_OP)), COMMITMENT, BID)).toBe(USER_OP)
	})

	it("finds a place_bid nested in a batch", () => {
		const call = batch([placeBid(OTHER_COMMITMENT, OTHER_USER_OP), placeBid(COMMITMENT, USER_OP)])
		expect(extractUserOpFromExtrinsic(asExtrinsic(call), COMMITMENT, BID)).toBe(USER_OP)
	})

	it("finds a place_bid nested in a proxied batch", () => {
		const call = proxy(batch([placeBid(COMMITMENT, USER_OP)]))
		expect(extractUserOpFromExtrinsic(asExtrinsic(call), COMMITMENT, BID)).toBe(USER_OP)
	})

	it("returns the user op of the matching commitment, not the first bid in the batch", () => {
		const call = batch([placeBid(OTHER_COMMITMENT, OTHER_USER_OP), placeBid(COMMITMENT, USER_OP)])
		expect(extractUserOpFromExtrinsic(asExtrinsic(call), OTHER_COMMITMENT, BID)).toBe(OTHER_USER_OP)
	})

	it("tells a filler's bids on one order apart by their identifier", () => {
		// Several prices on one order go out as several bids, possibly in one batch.
		const call = batch([placeBid(COMMITMENT, OTHER_USER_OP, OTHER_BID), placeBid(COMMITMENT, USER_OP, BID)])
		expect(extractUserOpFromExtrinsic(asExtrinsic(call), COMMITMENT, BID)).toBe(USER_OP)
		expect(extractUserOpFromExtrinsic(asExtrinsic(call), COMMITMENT, OTHER_BID)).toBe(OTHER_USER_OP)
		expect(extractUserOpFromExtrinsic(asExtrinsic(call), COMMITMENT, `0x${"b3".repeat(32)}`)).toBeUndefined()
	})

	it("returns undefined when no place_bid matches the commitment", () => {
		const call = batch([placeBid(OTHER_COMMITMENT, OTHER_USER_OP)])
		expect(extractUserOpFromExtrinsic(asExtrinsic(call), COMMITMENT, BID)).toBeUndefined()
	})

	it("returns undefined for an unrelated extrinsic", () => {
		const call = { section: "balances", method: "transfer", args: [hex("0xaaaa"), hex("0x01")] }
		expect(extractUserOpFromExtrinsic(asExtrinsic(call), COMMITMENT, BID)).toBeUndefined()
	})

	it("returns undefined when the event has no extrinsic", () => {
		expect(extractUserOpFromExtrinsic(undefined, COMMITMENT, BID)).toBeUndefined()
	})

	it("does not loop on a self-referential call tree", () => {
		const cyclic: any = { section: "utility", method: "batchAll", args: [] }
		cyclic.args = [[cyclic]]
		expect(extractUserOpFromExtrinsic(asExtrinsic(cyclic), COMMITMENT, BID)).toBeUndefined()
	})
})
