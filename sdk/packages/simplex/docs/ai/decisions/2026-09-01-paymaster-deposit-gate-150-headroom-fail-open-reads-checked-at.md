# 2026-09-01 — Paymaster deposit gate: 150% headroom, fail-open reads, checked at selection time

Chosen: a candidate paymaster is skipped unless `EntryPoint.balanceOf(paymaster)` covers the op's
max prefund times `DEPOSIT_HEADROOM_PERCENT` (150%), computed from the caller-supplied base gas
plus the candidate's worst-case verification and postOp constants.

Exact-prefund (100%) was rejected because the bundler applies its exact check at execution — for a
bid, minutes after selection — and concurrent in-flight ops draw on the same deposit; passing the
exact check at selection and failing it at execution reproduces the original failure. A larger
multiplier (2x+) was rejected as arbitrary: healthy keeper-maintained deposits exceed one prefund
by orders of magnitude, so any near-1 multiplier only bites when the deposit is effectively empty,
and 150% buys roughly one concurrent op's share without rejecting a merely modest deposit.

The deposit read fails open (candidate kept, warn logged). Fail-closed would turn every transient
RPC error into a lost bid or a native-gas fallback; failing open degrades to exactly the status quo
— the bundler's own precheck — and never worse. The check is skipped entirely when the caller
passes no `prefund` or the chain has no EntryPoint, preserving balance-only selection for callers
that cannot price the op.

Known gap, deliberately left: `filler.ts` skips solver EntryPoint deposit top-ups whenever
`hasPaymaster(chain)` is true, so a chain whose paymasters are all deposit-drained now produces
`type: "none"` bids that draw on a solver deposit nothing maintains. The durable fix is the
paymaster keeper maintaining the deposits, not the filler funding a fallback it was configured to
avoid.
