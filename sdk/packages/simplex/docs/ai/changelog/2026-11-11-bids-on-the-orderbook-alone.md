# 2026-11-11 — Simplex bids on the orderbook alone and needs no Hyperbridge account

Simplex 0.18.6 posts every bid to the HyperFX orderbook and never to Hyperbridge. The Substrate
account, its BRIDGE balance and the BRIDGE top-up are gone. This replaces the two-venue bidding of
[bids on the orderbook](2026-10-21-bids-on-the-orderbook.md).

## Where a bid goes

`FillerStrategy.executeOrder(order, bids?: BidPlacer)` takes an `OrderbookBidPlacer`
(`src/core/bid-placer.ts`, replacing `bid-venues.ts`). `BidPlacer.place(commitment, userOp)` posts
the SCALE-encoded op with `OrderbookClient.submitBid` and returns the orderbook's
`SubmitBidResult`. A request that got no answer comes back as `failed` with `REQUEST_FAILED` and
never rejects. `IntentFiller.setOrderbook(client)` builds the placer. Intake skips a
solver-selection order when no orderbook is set.

A bid is placed when the orderbook holds it (`accepted` or `unchanged`, `orderbookHoldsBid`):

- `FillResult.success` is that. `error` is the refusal as `CODE: message` (`bidRefusal`).
  `txHash` is unset, and `FillResult.pending` is gone.
- `orderExecuted.success` and the activity's `bid` row mean the same. `orderFilled` fires with the
  commitment only for a placed bid.

## What the filler no longer does

The filler opens no Hyperbridge connection, and `hyperbridgeConnection` is gone. `BridgeBalanceGate`
and the retraction machinery are removed: the retraction queue and sweep,
`retractStaleBids`, `retractBidsForLimitOrder`, and the retractions on a fill, a rival's
completion, a cancel or an expiry. The orderbook cannot withdraw a bid. A bid whose limit order
closes after it went out stays executable until its `validUntil`, and the limit order's on-chain
budget caps what it can fill. A complete fill, ours or a rival's, still stops the bid loop.
`SolverWork.retractions` is gone from `/api/status`. The desktop's work parser no longer requires
it.

Fill attribution (`recordExecutedFill`) now windows every candidate bid: one stored longer before
the fill than `bidValiditySeconds` + discovery pad + permit skew cannot have executed. Before, only
retracted and orderbook-only rows were windowed. The fill's block is therefore read for every fill
with a candidate.

## Bid store

New rows write `success` and `onOrderbook` as the orderbook's answer, and nothing else about
Hyperbridge. `StoredBid` keeps `pending`, `extrinsicHash`, `blockHash`, `retracted`,
`retractedAt`, `retractExtrinsicHash` and `dead` for rows written before 0.18.6. Nothing writes
them now, and the SQLite schema is unchanged. `BidStore` drops `unretractedReclaimable`,
`expiredUnretracted`, `markRetracted` and `markDead`. `BidInsert` drops `extrinsicHash`,
`blockHash` and `pending`. `BidStats` and `BidStatsDto` drop `retracted` and `pendingRetraction`.
The in-memory store prunes the oldest rows past its cap. The dashboard shows **Orderbook** or
**Failed** for a bid, and keeps the Hyperbridge extrinsic links for older rows.

## Config and setup

`simplex.substratePrivateKey` and `simplex.hyperbridgeWsUrl` are accepted and never read. Boot logs
a warning when either is set, and validation no longer requires the URL with the key. `simplex
init` and the UI wizard no longer ask for an account. A rewrite of the file (`emitFillerToml`, the
init update run) keeps both keys, with a comment saying they are unused. The key controls an
account that may still hold BRIDGE, and a rewrite must not delete the operator's only copy of it.
The setup API drops `generate-substrate-key`, `check-substrate-balance` and
`SetupDefaults.hyperbridgeWs`. It still masks the key in a preview.

Removed with the account: `BridgeTopUp` and its `/api/bridge-top-up` routes and dialogs,
`BalanceProvider`'s BRIDGE balance (`BalanceSnapshot.hyperbridge`, `HyperbridgeBalance`), the
dashboard's **BRIDGE available** tile, `status.addresses.substrate`, and `services/substrate-key`.
`RuntimeState.bridgeTopUp` is no longer typed. A stored record stays in the state table, unread.

## What an upgrading operator recovers by hand

Nothing in 0.18.6 retracts a bid placed on Hyperbridge, and nothing resumes or reclaims an open
top-up. The [configuration docs](../../../../../../docs/content/developers/evm/simplex/configuration.mdx)
("Upgrading from a Hyperbridge account") say to let 0.18.5 finish first. Pause, and wait until no
retractions are pending and the top-up has delivered or been reclaimed. Otherwise retract by hand
with `intentsCoprocessor.retractBid(commitment, bid)` for each `bids.db` row with an
`extrinsic_hash` and `retracted = 0`. The bid schema did not change, so 0.18.5 can run on a 0.18.6
data directory.

## E2E

The testnet swap run configures solvers without Hyperbridge keys and no longer funds them with
BRIDGE. `E2E_SOLVER{1,2,3}_SUBSTRATE_KEY` and `SECRET_PHRASE` are dropped from the workflow.
`orderbook-only-bid` no longer restarts a solver without a key. It checks that Hyperbridge served
no solver's bid and that a bid the orderbook served filled the order. `E2E_HYPERBRIDGE_WS_URL`
stays: the swapper's SDK still reads bids from the coprocessor beside the orderbook.
