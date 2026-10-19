# 2026-10-16 — A partial fill's floor, and three guards around posting, rebalancing and delegation

Four fixes from the filler audit. Each closes a path that spent money, or moved it, on
a bid that could not pay for itself.

## `minPartialFillUsd` — a partial fill has a size floor

A partial fill collects none of the order's fees and is deliberately exempt from the
fee check (GATE 1) and from the caller's profit floor: what pays its gas is the margin
in the limit order that prices it. That left it with no size floor at all, so a
near-exhausted limit order, or a wallet just above its gas reserve (`usableWallet`),
bid dust on every matching order — each bid paying fill gas, the cross-chain relayer
fee and the Hyperbridge bid fees.

`[simplex] minPartialFillUsd` sets the smallest payout a partial fill may bid. It
defaults to 20, must be a number `>= 0`, and is read through
`FillerConfigService.getMinPartialFillUsd()`. `0` disables the floor.

- The bid is valued by its payout, through `usdValueOf` on the limit orders' own rates,
  once `finalOutputAmount` is final and `releasedInput` is known.
- When no limit order prices the output token, the fallback is the input the bid
  releases, at the same rates, and it is computed only then. `getOrderUsdValue` totals
  every leg of a multi-leg order, so it is not scaled by one leg's released amount; this
  leg's own input is priced instead. On a single-leg order the two agree.
- A partial worth less than the floor is skipped with an `info` line naming the order,
  limit order, token, payout and value. One that cannot be priced at all is skipped too,
  the way an unpriceable order sits below `minOrderSizeUsd`.
- **Behaviour change: a partial nothing can price is skipped by default.** The operator's
  own book must give the payout, or the input it releases, a dollar value, and
  `limitOrderUsdEdges` drops same-asset orders. A book of same-asset CNGN/CNGN or
  EURC/EURC orders, or a ZARP/CNGN book on its own, has no path to a dollar stable, so
  every partial on it is skipped until `minPartialFillUsd = 0` is set.
  `docs/content/developers/evm/simplex/configuration.mdx` documents the field beside
  `minOrderSizeUsd`, and `troubleshooting.mdx` lists both skip reasons.
- The check runs before `estimateGasFillPost`, so a skipped bid costs no gas estimate.
  It also runs before the wallet pool is drawn down and before `recordOrderOutcome`, so a
  refused bid leaves the pool for later candidates. When the leg is already known to be a
  partial and even its maximum possible value is below the floor (`targetOutput`, or the
  whole input when no limit order prices the output), or nothing prices it at all, a
  cheap pass skips it before any balance or funding-venue planning, so a dust or
  unpriceable bid holds no wallet room and reserves no vault liquidity either. A bid the
  final check refuses, after the wallet cut its payout, has already planned its vault
  withdrawal, and that reservation lapses on its own.
- Full fills, GATE 1, the profit-floor exemption and `feeCheckWaived` are unchanged.

## Reconciliation cannot undo a cancel

`reconcile()` builds its owner map from a snapshot (`live()`), then awaits network
paging (`postedOrders()`) and per-orphan withdrawals. A `cancel()` landing in that
window was overwritten: the under-funded branch wrote the snapshot's `open` back over
the cancelled row, and the order started matching and posting again. That write now
takes `setStatus(owner.id, owner.status, reason, [owner.status])`, the same status
guard every other posting write uses, and the under-funded tally is counted only when
it applied. The guard also stops a snapshot write of `open` over a row that moved to
`resizing`.

## Overlapping rebalance ticks

`startRebalancing` fires at 30 seconds and then every 5 minutes. A CEX leg can take
half an hour and a USDT0 leg fifteen minutes, so two ticks would overlap and plan the
same deficit twice. `runTrackedRebalance` now returns without starting a second pass
while `activeRebalances` is non-empty, logging `Previous rebalance still running;
skipping this tick`. `stop()`'s drain and `getWorkSnapshot` are unchanged.

Out of scope: re-planning money still in transit after a timeout, an error or a
restart. That needs persisted in-flight transfer accounting.

## Orders are not bid on for an undelegated chain

At boot `setupSolverSelection(chains, { requireAll: false })` tolerated per-chain
EIP-7702 failures with a warning and recorded nothing. The bundler's estimate injects
the SolverAccount code by state override, so an undelegated EOA's bid passed estimation
and paid the Hyperbridge bid fees and deposit for a fill that could never execute.

`IntentFiller` now tracks the chains that failed in `undelegatedChains` (set or left
set on failure, cleared on success, dropped by `removeChain`, and cleared by `addChain`'s
rollback and by `enableFills` when solver selection is inactive there). The mark is a
belief: a failed `getCode`, or a delegation that mines after its receipt timeout, leaves
a delegated wallet marked, and the dashboard cannot clear it because its switch reads
the chain and is disabled. So `handleNewOrder` re-reads `isDelegated` for a marked chain
at most once per `DELEGATION_RECHECK_MS` (60 s), and drops the mark, then evaluates the
order, when the wallet is delegated. `handleNewOrder`
skips an order whose destination is in the set, logging at `info` and emitting
`orderSkipped` with reason `undelegated`, right after the watch-only intake check. With
`requireAll: true` a failure still throws as before, but the chain is marked first.
This is not watch-only: that flag is persisted to the TOML by the watch-only sync.
A successful dashboard "Enable fills" (`ChainController.enableFills` →
`IntentFiller.enableFills` → `setupSolverSelection([chain], { requireAll: true })`)
clears the skip.

Turning watch-only off for a running chain (`ChainController.setWatchOnly(chainId, false)`)
calls `IntentFiller.guardDelegation`. A chain that was watch-only at boot was never
delegated or marked, so on a solver-selection chain it reads `isDelegated` and marks the
chain undelegated when that is false. The re-read at intake, or `enableFills`, clears it.
That solver selection read is uncached for such a chain, so a failure of it is logged and
solver selection is assumed active: the delegation check still runs, and a failed
delegation read marks the chain undelegated, so a transient RPC error does not fail
`setWatchOnly(false)`.

`reconcile()` can reach `retire()` through `repost()` from its snapshot, so the
`expired` write there takes the same `POSTABLE` status guard and cannot undo a cancel.

`minPartialFillUsd` carries the same default, validation and round-trip as
`minOrderSizeUsd`: `src/config/filler-toml.ts`, `src/services/FillerConfigService.ts`,
`src/core/boot.ts`, `src/cli/init/emit-toml.ts` and `filler-config-example.toml`.
