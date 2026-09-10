# 2026-09-03 — The phantom-order lag limit is read from the chain, and always applies

Chosen: the poll derives its threshold from `phantomTimings()` — the pallet's `PhantomBidWindow` (or the
`PhantomOrderBidWindowBlocks` constant behind it) and `PhantomOrderInterval` — and applies it unconditionally.

Alternative rejected — a `maxLagBlocks` option, off by default. It was written that way first, to preserve the
cursor's existing property: it advances only past blocks whose events were really read, so an outage delays
orders instead of losing them, which is what made the cursor a fix for the dropped-subscription bug and what a
consumer reading the feed as history would want. But no such consumer exists, and the feed is not history: a
phantom order is a standing invitation to bid that expires with its window. Past that window there is no caller
for whom walking the backlog is right, so the option only offered every caller a way to get it wrong — and the
one that got it wrong on mainnet would have had to opt in to be fixed.

The property is bounded rather than abandoned: inside the limit an outage still delays orders instead of dropping
them, and the tests pin both halves.

Alternative rejected — expose the lag and let the caller reset the poll. It moves the same decision one layer out
while making every caller reimplement the jump, and the poll would still need the head it already reads.

Alternative rejected — a fixed number of blocks. It was 60 for one commit and it was already wrong: Nexus's
window is 15 inside a 55-block interval, so the real threshold is 70, and on Gargantua (window 5) it is 10. Both
values are governance-set and neither is derivable from the other, so any constant is wrong on some chain or
after some referendum.

The timings are read once per instance rather than per tick: a governance change to either is rare, a request per
tick forever is not free, and the cost of caching is that a change lands on the next restart. A failed read is
not cached, so it retries, and it fails the tick rather than guessing — leaving the cursor exactly where it was.

Removed in the same breath: `lookbackBlocks`, which started a cold cursor some blocks behind the head so a
restarting process could still bid on a window already open. That is the same late bid the age gate downstream
now refuses — a process that has just come up is, by definition, near the end of any window it reaches back for —
so the option was buying exactly the behaviour this change exists to stop. No caller set it.
