# Order execution follows destination progress

`OrderExecutor` reads an order's state from its destination gateway before it starts and before
every round of bids: the output credited per leg from `_partialFills`, and `_filled`. Fills made by
other solvers count towards `totalFilledAssets` and `remainingAssets`, and an order resumed after a
restart continues from what is already credited.

Once `_filled` is set the stream ends. It yields `FILLED` when every leg is complete, with
`selectedSolver` set to the finalizer and no `userOpHash` if the completing fill was not executed in
this session. It yields the new `CANCELLED` status, with the progress reached, when the order was
finalized short of its output, which is what a destination-side cancellation does.

`Bid.execute` reads fill events only from its own operation's logs in the bundle transaction, the
range that ends at the EntryPoint's `UserOperationEvent` for its hash, and only from the gateway. A
bundle can carry another solver's fill of the same order; that fill is no longer reported as this
bid's. An operation whose `UserOperationEvent` reports failure throws, and `filledAssets` sums every
`PartialFill` the operation emitted, by leg.
