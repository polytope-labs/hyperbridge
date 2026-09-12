# 2026-08-27 — Phantom orders are validated against the pallet's shape before they are quoted

`IntentFiller.preparePhantomBid` now refuses any phantom order that does not look like one
`phantom_order_commitment` built: a non-zero `session`, a non-zero output amount, or a
`source`/`destination` that is not the announced chain. Those checks are pure field comparisons —
no RPC. On top of them, the destination head is read and an order that is still fillable is
refused; that read is best-effort, and a failure warns and continues rather than stopping the
filler bidding.

The reason any of it is needed: `quotePhantomFill` deliberately runs with no budget, no
wallet-balance read and neither profit gate, so the amounts signed are bounded only by the order
body. The body is read from a single Hyperbridge node's offchain storage, and `fetchPhantomOrder`
assigns `id` from the event rather than re-deriving the commitment from the bytes — so it is
unauthenticated. A forged body would have turned that unbounded quote into a signed, executable
authorization to fill an order of someone else's choosing, paying out to a beneficiary named in
the same body.

Each invariant independently breaks that. `session` is the direct one: `_select` recovers a key
with `ECDSA.recover`, which can never return the zero address, so a genuine phantom order can
never have a solver selected at all.

Found by the scheduled IntentGateway/Simplex security audit.

The unit tests now also rebuild an order the way `phantom_order_commitment` does, encode it with
the real `placeOrder` ABI and decode it the way `fetchPhantomOrder` does, before running the guard
over it. The hand-built fixtures otherwise bake in the assumption the guard depends on — that a
pallet-generated order presents `source`/`destination` as the same string the event carries in
`chain` — and getting that wrong would refuse every genuine phantom order and silently stop the
filler bidding, which only the simnode E2E would have caught.

The simnode E2E was seeding `LatestStateMachineHeight` with `createType("u64", h).toHex()`.
polkadot-js renders integers big-endian in hex while SCALE stores them little-endian, so the
runtime read `0x00000000000f4240` back as 4.6e18: every phantom order in that suite carried a
far-future deadline and was, contrary to the entire point of a phantom order, genuinely fillable.
Nothing noticed until this guard refused to quote them. Fixed to seed `toU8a()`.

Files: `src/core/filler.ts`, `src/tests/core/phantom-order-validation.test.ts`,
`src/tests/phantom-filler.e2e.simnode.test.ts`.
