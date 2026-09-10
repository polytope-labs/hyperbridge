# 2026-08-27 — Phantom orders are gated on the pallet's structural invariants, not on a re-derived commitment

Chosen: `preparePhantomBid` checks `session == 0`, all output amounts zero, and
`source == destination == event.chain` — all RPC-free — and then cross-checks the deadline against
the destination head as defence in depth.

The obvious alternative is to re-derive the commitment from the fetched bytes and compare it to the
event's. Rejected as the gate: a node that serves a forged body also serves the
`PhantomOrderRegistered` event, so it can publish a matching commitment. And `filler.ts` already
documents why the commitment comes from the event — re-deriving risks parity divergence if the
encode round-trip does not reproduce the pallet's bytes exactly, which would break every honest bid
to defend against a dishonest node the check does not stop.

The structural checks have neither problem. They are comparisons against constants the pallet fixes
at construction, and each independently makes a forged bid unexecutable — `session == 0` most
directly, since `ECDSA.recover` can never return the zero address, so `_select` can never stage a
selection.

**Why the deadline check is not the primary gate.** It was, initially. Two things argued against
leaning on it alone. It needs a live `eth_blockNumber`, and this code runs inside the on-chain bid
window (as few as 5 blocks), so it is the wrong place to add a mandatory round trip. And it
compares a Hyperbridge-confirmed height against a destination-chain head — two clocks whose
relationship holds in production but is an environment property, not an invariant, which makes a
hard failure on it brittle in exactly the environments (dev, simnode, forks) where the heights are
seeded by hand. Keeping it as a cross-check preserves the signal without either cost.

**Why its RPC failure is non-fatal.** Failing closed reopens nothing, because the structural checks
do not depend on the EVM RPC — and the threat here is a dishonest Hyperbridge node, not a dishonest
EVM endpoint. Hard-failing would let one flaky endpoint stop this filler bidding altogether, which
trades a risk that is already covered for a certain outage.

Not adopted: applying the real bid's ceilings to phantom quotes. Wrong layer — the quote is meant to
be unbounded, because it is a price advertisement rather than a fill. Constraining it would distort
what the filler publishes to fix a problem that is really about trusting the order body.
