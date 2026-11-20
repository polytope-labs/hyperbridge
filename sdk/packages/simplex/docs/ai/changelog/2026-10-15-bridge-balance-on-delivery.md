# 2026-10-15 — BRIDGE balance re-read the moment a top-up is delivered

A delivered BRIDGE top-up now re-reads the filler's Hyperbridge balance before the sync that
confirmed the delivery returns, so the dashboard's reload shows the new balance at once.

The Hyperbridge balance is not part of `BalanceProvider.refresh()`: `trackHyperbridgeBalance`
reads it once at boot and then on the same once-a-minute interval, because the substrate read is
cheap to cache and rarely urgent. That left the dashboard showing the pre-top-up balance for up to
a minute after delivery: the UI reloads balances the moment the top-up stage flips to `delivered`,
but the snapshot's `hyperbridge` field was whatever the last interval tick had read.

`BalanceProvider.refreshHyperbridgeBalance()` is the interval's read, now public and safe to call
any time (a no-op until tracking is initialized; a failed read keeps the last snapshot).
`BridgeTopUp` takes an optional `refreshBridgeBalance` and awaits it — best-effort — where
`readDelivery` transitions the record to `delivered`, wired in `boot.ts` to the balance provider.
Awaiting it inside the sync is what makes the dashboard's follow-up `/api/balances` read fresh
rather than racing a fire-and-forget refresh.
