/**
 * Fields in a `BidPlaced` event since bids gained an identifier: `(filler, commitment, bid, deposit)`.
 *
 * Events from before that runtime upgrade carry `(filler, commitment, deposit)` and decode against
 * their own block's metadata, so their third field is the deposit, not an identifier. The bids they
 * describe live in the retired `Bids` map and cannot be retracted or served through the current
 * calls, so they are not indexed.
 */
export const BID_PLACED_FIELDS = 4

/** Whether a `BidPlaced` event carries a bid identifier, i.e. was emitted after the upgrade. */
export function bidPlacedCarriesBidId(fields: { length: number }): boolean {
	return fields.length >= BID_PLACED_FIELDS
}
