# 2026-09-03 — Stop bidding on phantom orders whose window has closed

A mainnet filler (`0xb98306ac…`, Hyperbridge `12KyapjPpm2fK62gepzZKEEk3xP9vzEJdWBspagx8DxZaj2k`) spent nine hours
bidding on phantom orders that had expired hours earlier. Its bids landed, nothing errored, and it backed no pool
the whole time: the indexer shows 0 `PoolBidder` rows against 692 balance rows, the newest of which stops at
03:03 while healthy fillers keep writing them.

The lag grew monotonically — 4 blocks behind the order it bid on, then 147, 1611, 2228, 2765, 3448 — which is the
signature of a cursor that cannot catch up rather than a one-off stall. `pollPhantomOrders` advances the cursor
by at most `maxBlocksPerPoll` (10) per tick and never skips ahead, so a deficit accumulated while ticks were lost
(a scan overrunning the 15s interval, or the rate-limit backoff sitting out up to 8 ticks) is only repaid if the
sustained rate beats the chain's. Below that, the filler stays behind forever, and every order it then sees is
already dead: the bid window is tens of blocks.

Two fixes, at the two places the delay can come from:

- The SDK's poll now abandons a backlog older than one generation cycle rather than walking it, and the scanner
  logs each skip: a filler that keeps skipping is one falling behind the chain.
- `handlePhantomOrders` drops events older than the pallet's own bid window (read from chain, plus a 2-block
  margin) against one head read per batch. That also covers the delay the scanner cannot see — the global queue this runs on, and the
  quoting inside `preparePhantomBid` — and turns a silent outage into a warning naming the lag. A failed head
  read keeps every event: one flaky endpoint must not stop the filler bidding.

Neither is a substitute for the other: the first stops the backlog forming, the second refuses to act on one that
does.

Files: `src/scanner/hyperbridge-scanner.ts`, `src/core/filler.ts`, `src/tests/core/phantom-bid-staleness.test.ts` (new).
