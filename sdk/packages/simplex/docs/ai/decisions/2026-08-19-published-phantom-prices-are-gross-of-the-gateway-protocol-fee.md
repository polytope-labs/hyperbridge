# 2026-08-19 — Published phantom prices are gross of the gateway protocol fee

Noted, not changed. `placeOrder` deducts `protocolFeeBps` from each input, mutates `order.inputs`
to the reduced amounts, and takes the commitment over those — so on a real order the strategy
already receives a post-fee input and must not net it again. Phantom orders never pass through
`placeOrder`, so their standard amount is un-netted and the published price is the gross rate.

This does not break quoting: `PhantomSnapshotQuoter` calls `deductProtocolFee` before applying the
rate, mirroring the gateway's floored arithmetic exactly. Any _other_ consumer reading
`medianPrice / standardAmount` as an executable rate is ~0.3% optimistic at the deployed 30 bps.
Netting it inside `quotePhantomFill` would need the strategy to read a gateway parameter it does
not currently know about; left open pending a decision on whether the published surface should be
gross or net.
