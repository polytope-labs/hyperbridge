# 2026-09-03 — A stale phantom order is dropped, not bid on, and the drop is loud

Chosen: two independent guards, both sized off the pallet's own bid window read from chain — the poll skips a
backlog it cannot bid on, and the filler refuses events whose window has closed, warning each time.

Alternative rejected — only fix the poll. It stops the backlog forming but not every source of delay: this
handler runs on the global queue behind up to `maxConcurrentOrders` other jobs, and `preparePhantomBid` quotes
every leg before signing. The age check sits at the last point before submission, where all of that has already
happened.

Alternative rejected — only fix the filler. It stops the wasted deposits but leaves the poll walking a backlog
forever, so the filler keeps skipping every order and never returns to live.

Alternative rejected — bid anyway and let the pallet reject it. It does not reject: the extrinsic is accepted,
the deposit is reserved, and the aggregation simply never reads that bid because it read the order's bids when
the window closed. That is what made this invisible — the filler's own logs said "Phantom bids submitted".

Chosen: a failed head read bids on everything rather than nothing. Refusing to bid without a head would let one
flaky endpoint stop the filler entirely, which is the worse failure of the two — the age gate exists to catch a
systematic lag, not to be the last word on a single tick.

Neither limit is a constant, because the window is not: it is governance-set, and Nexus already runs 15 against a
runtime constant of 25 (Gargantua's is 5). A number compiled in here is wrong in one direction or the other —
at 40 it waved through 25 blocks of bids the pallet was already bouncing, and had governance raised the window
past 40 it would have started dropping live orders. `phantomTimings()` reads `PhantomBidWindow` (falling back to
the `PhantomOrderBidWindowBlocks` constant when zero, exactly as the pallet does) and `PhantomOrderInterval`,
once per instance.

The only fixed number left is `PHANTOM_BID_AGE_MARGIN_BLOCKS` (2), and it errs permissive on purpose: the pallet
accepts while `block_number <= created_at + window`, and a bid lands a block or two past the head this gate
reads, so shaving the edge would throw away bids that would have landed. The pallet is the authority there; this
gate exists for the systematic lag, where the order is thousands of blocks old and no margin matters.
