# Watch-only orders in the activity feed

`IntentFiller` drops an order for a watch-only destination in its intake queue (before
`evaluateOrder`), and records it there as `orderSkipped` with reason "watch-only" so the history
row reads "Skipped — watch-only". `evaluateOrder` keeps its own identical check for callers that
reach it directly. Orders for chains that are not configured at all are still dropped silently at
intake: they are other lanes' traffic, and recording each would flood the feed.
