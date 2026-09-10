# 2026-09-03 — `pollPhantomOrders` can skip a backlog it will never use

A mainnet filler fell behind the head and stayed there: the cursor gains at most `maxBlocksPerPoll` per tick and
never skips, so a deficit accumulated while ticks were lost is only repaid if the sustained rate beats the
chain's. It kept delivering orders 3,448 blocks old, whose bid window had closed hours before, and the filler
dutifully bid on every one.

The cursor now abandons a backlog it cannot use: once it is more than a generation cycle behind the head
(`bidWindowBlocks + max(intervalBlocks, bidWindowBlocks)`) it jumps to one window behind the head — keeping every
order that can still be bid on — and reports the range it dropped through `onSkip`. Not configurable — this
feed exists to be bid on, and an order that far behind cannot be bid on by anyone, so there is no caller for whom
walking that backlog is the right answer. The check runs only once the cursor is established, so a cold start —
exactly one block behind — is never read as a backlog.

That bounds the property the cursor was built for. It still advances only past blocks whose events were really
read, so an outage delays orders rather than dropping them — up to the point where the delayed orders are dead
anyway.

`lookbackBlocks` is gone with it. It existed so a restarting process could reach back and still bid on a window
already open, which is the same late bid this change is removing: by the time the process is up, that window has
all but closed. Nothing passed it, and a cold start now begins at the head.

The threshold is the chain's, not a constant: `phantomTimings()` reads `PhantomBidWindow` — falling back to the
`PhantomOrderBidWindowBlocks` runtime constant when that storage value is zero, exactly as the pallet's own
`phantom_bid_window()` does — and `PhantomOrderInterval`, once per instance. Nexus runs a window of 15 against a
constant of 25 and an interval of 55, so anything hard-coded is wrong in one direction or the other.

Also adds `latestBlockNumber()`. A phantom order carries the block it was registered at and no clock, so a
consumer deciding whether one is still biddable has to read the head, and nothing public exposed it.

Files: `src/chains/intentsCoprocessor.ts`, `src/tests/pollPhantomOrders.test.ts`.
