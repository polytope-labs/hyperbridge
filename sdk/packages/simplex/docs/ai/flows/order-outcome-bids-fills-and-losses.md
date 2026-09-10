# Order outcome: bids, fills, and losses

With solver selection, `IntentFiller` "executing" an order submits a bid: `orderFilled` (now with
`commitment`) and `orderExecuted` (with `commitment`) fire on acceptance. `ActivityRecorder` skips
`orderFilled` when it carries a commitment and records `orderExecuted` with a commitment and
`success` as a `bid` row (txHash = extrinsic hash). `ChainScanner` passes each OrderFilled log's
`transactionHash` in `ScannedFill`; `EventMonitor.handleFill` emits `orderFillObserved` for every
fill with `ours` (filler address match), then the existing `orderFilledOnChain` for ours only. The
recorder's `settle` records `filled` (ours) or `lost` (reason = winner) for orders it knows
(summary cache or `ActivityStore.knowsOrder`). `Orders.tsx` ranks Filled > Outbid (neutral badge, winner address beneath) > Bid placed /
Bid retracted (latest bid) > Executed/Failed > Skipped > Detected, shows the latest bid's standing
in the Bids cell; the row's only external link is the HyperFX order page (explorer links for the
placement and fill transactions were removed). `fills()` (wallet ledger) now lists real fills only.
The latest bid renders in one Bids cell as two icon links to Statescan for the running network
(`OrderHistoryDto.network`): up for the bid extrinsic, down for the retraction; either is a dimmed
arrow when absent (a bid closed out by `BidNotFound` has no retraction extrinsic), and a failed bid
shows "Failed" with its error on hover. At boot the
backfill's second pass lists `unsettledOrders` (a `bid` row or a legacy bid-time `filled` row — those
carry `volumeUsd` — with no `lost` or observed `filled` row), fetches each from the indexer with its
`statusMetadata`, retypes legacy rows to `bid`, and records `filled`/`lost` from the FILLED entry's
filler and transaction hash.
