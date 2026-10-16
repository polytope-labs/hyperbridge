# 2026-10-16 — Decimals are cross-checked, and a bid is capped above the ask

Two bounds around the same failure: a bid sized from a wrong number. One hardens the
`decimals()` read; the other caps the bid whatever that read says.

## `getTokenDecimals` cross-checks the registry

`ContractInteractionService.getTokenDecimals` reads `decimals()` through the plain
fallback client, not the event monitor's quorum, so the first endpoint to answer wins.
Cross-checking matters because one endpoint reporting 18 for a 6-decimal token inflates
every amount scaled by it by 10^12.

A successful read is cross-checked against the asset registry
(`configService.getAssetDecimalsByAddress`), which is verified on-chain when a token is
curated. Where the registry knows the token and the values disagree, the read is a hard
error — `Token … on … reports … decimals but the asset registry says …; refusing to price
it` — and nothing is cached, so the next call re-reads. Callers on the fill path treat a
throw as "skip this order" (`calculateProfitability` runs inside the caller's try/catch),
which is the right outcome for a value that cannot be trusted. A token the registry does
not know keeps the on-chain value and is cached; the overfill cap below bounds it.

The read-failure fallback to the registry is unchanged and still not cached. The native
token still short-circuits to 18 without a read. A cached value was cross-checked when it
was cached, and `setTokenDecimals` has no other caller.

## `maxOverfillBps` caps the bid

A matched limit order priced well above the ask would otherwise bid its whole offer —
drawing on the whole wallet, with the gateway handing the surplus to the swapper and the
protocol. `maxOverfillBps` only logged a warning.

The bid is `min(offer, ask × (1 + maxOverfillBps / 10000))`. The ceiling is computed
from the order's own requested output — never from our decimals read — so it bounds the bid
even when the read is inflated. A clamped bid is logged with the offer, the ceiling and the
configured bps. The capped payout is still at or above the ask, so it is a full fill and the
released escrow is the whole input. Everything downstream (`plannedOn`, the partial-fill
floor's cheap pass, `finalOutputAmount`, `releasedInput`, the filler outputs) uses the capped
value.

`maxOverfillBps = 0` bids exactly the ask. `validateConfig` now rejects a
`simplex.overfillProtection.maxOverfillBps` that is not an integer `>= 0`, rather than
letting `BigInt(...)` throw at construction. The cap does not feed `maxConsecutiveClamps`:
capping to a margin over the ask is ordinary for a limit order priced above the ask, and
`recordOrderOutcome(false)` is still recorded unconditionally.

## Ranking

Bids still carry the limit order's own rate up to `ask × (1 + maxOverfillBps / 10000)`, so operators
rank by price below it; above it they tie at the cap. The default is 500 bps (5%).

## Settlement releases its holds on failure

A settlement that fails part-way — an RPC error, the decimals cross-check, or the draw-down's
`LimitOrderWriteError` — returns the bid's holds to their limit orders before the error, so the
order's `reserved` is never left inflated.
