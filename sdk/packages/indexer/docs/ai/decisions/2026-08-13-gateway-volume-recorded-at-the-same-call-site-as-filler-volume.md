# 2026-08-13 — Gateway volume recorded at the same call site as filler volume, not the unconditional handler site (#1085)

Chosen: the new `IntentGatewayV3.FILLED` update sits directly next to the `IntentGatewayV3.FILLER.<address>` update inside `updateOrderStatus`, sharing its `getOutputValuesUSD` pricing.

Alternative considered: `orderFilledV3.event.handler.ts` already calls `recordOrderVolume("FILLED", ...)` unconditionally, even when the fill is indexed before its `OrderPlaced`. Recording there would not miss fill-before-place races.

Why the shared site won: it guarantees the invariant that the gateway entry equals the sum of the per-filler entries for a given chain and day, because both come from the same pricing call on the same code path. `recordOrderVolume` prices tokens itself and skips tokens with no known price, so its totals can disagree with the filler totals. The cost is inheriting a known gap: when a fill arrives before its `OrderPlaced`, `updateOrderStatus` stores `PendingStatusMetadata` and returns early, and the later replay (`flushPendingStatuses`) restores status only, never volume. The per-filler records already under-count that case; the gateway record under-counts it identically, which keeps the two consistent.

Known pre-existing quirk (not changed): `updateCumulativeVolume` skips the addition when `lastUpdatedAt` equals the incoming timestamp, so two fills in the same block increment daily volume twice but cumulative volume once. Left as is; changing it would affect every existing volume series.
