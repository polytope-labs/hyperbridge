# 2026-10-16 — The chain is the budget: no limit-order holds, no vault reservations

Simplex no longer keeps local holds against limit orders or local reservations against vault
liquidity. A limit order's budget is enforced on chain by `debitOrder`, and a vault refuses a
`withdraw` past `maxWithdraw`, both in the fill's own batch. Bids are sized from those on-chain
figures, and `remaining` is brought down to the order's tally after every fill. See
[the decision](../decisions/2026-10-16-limit-order-budgets-and-vault-liquidity-come-from-the-chain.md).

## Interfaces

- `LimitOrder.reserved`, `LimitOrderStore.reserve`, `release`, `transaction` and `drawDown`,
  `BidInsert.reservations`, `StoredBid.reservations` and `BidStore.claimReservation` are gone, so a
  custom store no longer implements them. `LimitOrderStore.clampRemaining` is the one write to
  `remaining`: it lowers an `open` or `resizing` order to the on-chain room, and never raises it.
  A store must accept `resizing`: a repost holds that status and posts the clamped size, so a
  store that refuses it re-advertises the pre-fill size. The public store reference says so.
- `StoredBid.pricedBy` is new (below). A custom `BidStore` that returns `undefined` for it on old
  rows is read as `null`.
- `LimitOrderService.syncToChain(id, blockNumber?)` and
  `ContractInteractionService.limitOrderSpent(chain, budgetId, blockNumber?)` take the block to
  read at. `orderFilledOnChain` carries the fill's `blockNumber`.
- The operator UI no longer shows "Held by live bids" in a limit order's detail or the fainter
  "held" segment of its progress bar. `progressOf` returns `{ consumed }` only.

## Bid provenance

Each bid row records the limit order that priced it as `StoredBid.pricedBy: BidPricing | null`:

| Field | Value |
|-------|-------|
| `limitOrderId` | The order that priced the bid. |
| `payout` | What the bid offered, at 1e18. |
| `take` | The input the bid signed beside it, in the input token's raw units. Absent on old rows. |
| `takeDecimals` | The input token's decimals on the source chain. Absent on old rows. |

`executeOrder` writes it with the bid row and nothing changes it afterwards: not a loss, a
retraction or a fill. `BidStore.byLimitOrder` reads from it.

## Settlement

`IntentFiller.settleFilledLimitOrder` takes the same path for a partial and a completing fill:

1. Takes our bid rows on the commitment that carry `pricedBy` (`pricedBy != null`; a row without
   it is not settled).
2. Attributes the fill to one of them, among the bids that went out (`success` or `pending`), are
   not retracted, and have no fill already recorded on this commitment (by `bid`). What the fill
   delivered is read per limit order, and only bids whose order pays a token the fill delivered are
   considered, so a multi-leg swap priced by orders that pay different tokens attributes to the right
   leg. Among those, the match is the one used before: the bid whose take covers the escrow released,
   best rate first, then the payout closest to the delivery. A fill is named by its transaction hash
   and log index (`orderFilledOnChain` carries `logIndex`, stored as `limit_order_fills.log_index`), so
   a replayed scan does not double a fill while two of our fills in one transaction are both
   recorded. Without a `logIndex` the hash alone names it. A fill with no hash, or a bid row with no
   `bid` identifier, cannot be deduplicated; that only affects logs without a hash and rows from
   before bids carried an identifier.
3. Records it with `LimitOrderStore.recordFill`, at what the gateway charged
   (`payout × released / take`, rounded up in the token's units, never below the delivery, capped at
   the payout) and with `amountIn` from `takeDecimals`.
4. Calls `LimitOrderService.syncToChain(id, blockNumber)` for every limit order that priced one of our
   bids on the commitment. It reads the order's tally at the block the fill landed in, lowers
   `remaining` to `budgetRoom`, and resizes or closes the order, which is what `reconcileTallies`
   does for each open order. Reading at the fill's block keeps a lagging load-balanced endpoint from
   answering with the tally from before the fill; the tally only grows, so that block already
   includes it. An endpoint that has not reached the block gives no figure, and the periodic pass
   follows up. That pass reads the latest block, and now logs a lowering at `warn`, since
   settlement should already have made it. A scanner that cannot tell the block substitutes
   `0n`, and a read pinned to block 0 fails, so `EventMonitor` forwards `blockNumber` only when
   it is above zero; an unset one reads the latest block.

   `syncToChain` answers `false` when the order is not `open`, when the tally leaves it where it
   is, when the chain does not show the fill yet, or when the tally cannot be read (the read
   logs a warning). It throws when the chain cannot be read for the token's decimals, or the store
   or orderbook fail. Settlement catches that and logs a warning.

## A fill while the order is `resizing`

`syncToChain` leaves an order that is not `open`, so a fill that lands while a repost, a resize or
a re-sign holds the row in `resizing` is not applied by settlement. `LimitOrderService.repost`
therefore lowers the order to the chain's tally before it posts: it reads the tally, lowers
`remaining` with `clampRemaining`, and posts that size. If the room left is under the orderbook's
dust floor it closes the order as `filled` instead of posting. A tally that cannot be read posts the
size the store holds, and the periodic pass follows up.

## SQLite migration and rollback

- `bids.priced_by TEXT` (JSON `BidPricing`) and `limit_order_fills.log_index INTEGER` are added in
  place when missing.
- `bids.reservations` and `limit_orders.reserved` stay in the schema and are never written, so a
  rollback to the previous build opens the file. This build also never releases the holds of
  pre-upgrade bids it retracts, so `reserved` would stay inflated and the previous build's
  `clampRemaining`, which keeps `remaining` up to `room + reserved`, would size bids past the chain's
  room. So every start sets `reserved` to `'0'` wherever it is not (a read first, so a clean start
  takes no write lock). This build never reads the column, and after a rollback the previous build
  starts clean; its releases floor at zero. It sees no holds on bids placed after the upgrade.
- A bid row written before the upgrade has no `priced_by`. Its provenance is read from the first
  entry of `reservations`, whose `amount` is the payout, so a bid in flight across the upgrade still
  attributes its fill. A row whose hold was already settled has no reservation left and reads `null`.

## Vault liquidity

`VaultLiquidityState.withdrawable(asset)` is the vault's `maxWithdraw(solver)` from the last
refresh. `VaultFundingPlanner.planWithdrawalForToken` refreshes under the per-chain mutex before
every plan, as it did before, so each plan is sized from a fresh `maxWithdraw`.
`getBalanceSnapshot` reports `availableAssets` as that same figure.

One evaluation never plans the same liquidity twice. `FXFiller.calculateProfitability` keeps
`venuePlanned`, a running total keyed by venue, destination chain and token, beside `balanceCache`
(the wallet pool) and `plannedOn` (each limit order). It passes the total so far as
`planWithdrawalForToken(..., alreadyPlanned)`, and the planner releases at most
`withdrawable − alreadyPlanned`, floored at zero. A candidate's vault draw is added only once it
becomes a plan. A candidate refused afterwards by the partial floor, the dispatch-fee check, gate 1,
gate 2 or a zero release leaves it for later candidates. The total is dropped when the evaluation
returns. The next evaluation, including one of the same order, starts at zero.

## Accepted cost

Within one evaluation, the bids on a swap never plan more wallet, limit-order or vault liquidity
than exists. Across evaluations nothing is held: two bids from separate evaluations can each offer
the whole room a limit order or a vault has left. When both are selected, the one that executes
second exceeds the order's budget or the vault's `maxWithdraw` and reverts on chain. Its gas is
spent. No order pays out past its cap and no vault releases more than it holds.

## Reposting after a retryable post failure

`LimitOrderService.post` wrote a retryable failure back as `open` without the `POSTABLE` status guard,
so a cancel or an expiry that landed while the post was in flight was reverted. It is guarded now,
and a lost guard returns the row as the cancel left it.
