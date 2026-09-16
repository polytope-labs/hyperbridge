# 2026-09-15 — Filler credit follows each fill, not the order

Decided: credit filler volume, the gateway `FILLED` volume and filler points inside `recordFill` and
`recordPartialFill`, valued from that fill's event outputs.

With partial fills, several solvers can fill one order. Crediting at `FILLED` status gave the
completing solver every other solver's slice.

The gateway `FILLED` series moves with the per-filler series. `seedAggregateVolume` builds it from the
per-filler daily rows, so updating one without the other would make them disagree.

Volume is credited when the fill is indexed, even if its order is not indexed yet. Points need the
order row, so a fill that arrives first gets its points on placement, from `backfillEarlyFills`.

Rejected: crediting at `FILLED` status as before. It misattributes other solvers' slices.

## Release inventory is published for the event's solver

Decided: `handleEscrowReleasedEventV3` publishes pool inventory for the `solver` the event names.

A partial redeem never sets `_filled`, so resolving the provider from `_filled` would publish nothing
for most releases.

The old event shape names no solver. It only comes from gateways that finalize every redeem, so
`_filled` is always set there, and the legacy handler keeps reading it.
