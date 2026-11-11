# Order outcome: bids, fills, and losses

With solver selection, `IntentFiller` "executing" an order posts a bid to the HyperFX orderbook
(`OrderbookBidPlacer`): `orderFilled` (with `commitment`) fires when the orderbook holds the bid, and
`orderExecuted` (with `commitment`) fires for every attempt, `success` meaning the orderbook holds it.
`ActivityRecorder` skips `orderFilled` when it carries a commitment and records a successful
`orderExecuted` with a commitment as a `bid` row (no txHash: a bid is no transaction). `ChainScanner`
passes each OrderFilled log's `transactionHash` in `ScannedFill`, with `complete` set for
`OrderFilled` and cleared for `PartialFill`; `EventMonitor.handleFill` emits `orderFillObserved` for
every fill with `ours` (filler address match) and `complete`, then `orderFilledOnChain` for ours
only. A complete fill, ours or a rival's, stops `executeOrder`'s bid loop from placing further bids
on the order. Nothing withdraws a bid from the orderbook: it is served until its TTL and can execute
until its own `validUntil`, so a bid whose limit order was cancelled or closed after it went out is
settled by provenance like any other. Our own fill, partial or complete, first goes through `settleFilledLimitOrder`, which
records it against the limit order that priced the executed bid (`StoredBid.pricedBy`) and brings
every limit order that priced one of our bids on the swap down to its on-chain tally
(`LimitOrderService.syncToChain`). The
recorder's `settle` records `filled` (ours) or `lost` (reason = winner) for orders it knows
(summary cache or `ActivityStore.knowsOrder`). `Orders.tsx` ranks Filled > Outbid (neutral badge, winner address beneath) > Bid placed /
Bid retracted (latest bid; only a bid from before 0.18.6 can be retracted) > Executed/Failed >
Skipped > Detected, shows the latest bid's standing in the Bids cell; the row's only external link
is the HyperFX order page. `fills()` (wallet ledger) lists real fills only. The Bids cell shows
**Orderbook** for a bid the orderbook holds and **Failed**, with the orderbook's refusal on hover,
for one it refused. A bid from before 0.18.6 that Hyperbridge took renders as two icon links to
Statescan for the running network (`OrderHistoryDto.network`): up for the bid extrinsic, down for
the retraction. At boot the
backfill's second pass lists `unsettledOrders` (a `bid` row or a legacy bid-time `filled` row — those
carry `volumeUsd` — with no `lost` or observed `filled` row), fetches each from the indexer with its
`statusMetadata`, retypes legacy rows to `bid`, and records `filled`/`lost` from the FILLED entry's
filler and transaction hash.
