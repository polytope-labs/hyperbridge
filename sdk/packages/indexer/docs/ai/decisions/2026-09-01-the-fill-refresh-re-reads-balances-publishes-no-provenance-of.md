# 2026-09-01 — The fill refresh re-reads balances, publishes no provenance of its own, and skips replayed fills

Chosen: on `OrderFilled`, re-read every recorded bidder's balance for the pools the fill traded through and
rewrite the depths from them, leaving rates and every `lastUpdatedBlock`/`lastUpdatedAt` alone.

Alternative rejected — subtract the filled amount from the filler's row. No RPC, exact for the one solver that
filled, and wrong for everyone else: a fill is not the only thing that moves inventory between windows
(rebalances, other pools' fills, withdrawals), and the arithmetic would drift from the chain with nothing to
correct it until the next window. Re-reading measures the thing the depth is defined as.

Alternative rejected — refresh only the fill's own chain. Cheaper, and it is the only chain this fill changed.
But the pool's depth is a cross-chain sum, and the request is for the pool's liquidity, not one chain's slice;
the per-chain memo already collapses the extra reads, and the two guards below mean this only ever runs at the
tip.

Alternative rejected — stamp the refresh with the fill's block and timestamp. `lastUpdatedBlock` is a Hyperbridge
block number, and the staleness window (`MAX_SAMPLE_AGE_BLOCKS`) is measured in those; an EVM block number is
numerically unrelated and would make every row look either astronomically fresh or unusably stale. Writing only
`lastUpdatedAt` would leave the pair describing two different events. A dedicated `refreshedAt` field was the
honest version of this and was dropped deliberately: it buys provenance metadata at the price of a schema change
on live pool entities, and the depth being fresher than its timestamp claims is the safe direction to be wrong in.

Chosen: a pool whose `lastUpdatedAt` is newer than the fill is skipped without reading a balance. Balances are
read at the chain head — as the snapshot path also reads them — so refreshing against a fill the pool has already
been sampled after would replace fresher data with a partial view of it. It also makes a historical resync free:
every replayed fill is older than the pools' current samples, so no RPC is issued at all.

Chosen: a chain whose balances cannot be read in full is left exactly as indexed. A failed read and a zero
balance are indistinguishable downstream, so publishing the reads that succeeded would report the rest of that
chain's bidders as having withdrawn — worse than a stale number, because it looks like news.

Accepted blind spot, documented at `refreshPoolLiquidity`: the refresh reads wallet ERC-20 plus redeemable
ERC-4626 positions, but a snapshot's weight can also include Uniswap V4 positions the bid declared, and only a
bid names those. A V4-funded bidder therefore shrinks to its liquid inventory until the next window restores it.
Errs downward, which costs a quote rather than a failed fill. The fix, if V4-funded solvers become material, is to
split the position share out in the SDK aggregation and persist it on `PoolBidder` so it can be carried forward —
which is a schema change, hence not done pre-emptively.

Chosen: `PartialFill` refreshes on the same terms as `OrderFilled`, through the same service method. It spends
output-token inventory identically — the only difference is that it is emitted by the same-chain path
(`IntrinsicIntents`), where source and destination are one chain, so the pair resolves against a single registry.
Cross-chain fills are all-or-nothing (`ExtrinsicIntents` emits only `OrderFilled`), so between the two handlers
every fill that moves inventory now triggers a refresh. The per-block balance memo means a partial fill and the
full fill that follows it in the same block read balances once.

Chosen: a refresh extends `LiquidityProviderBalanceV2` too, stamping its rows with Hyperbridge's head block read
live from the configured node. The series is keyed by Hyperbridge block and a fill has none of its own, so
something has to supply one, and the head is the only number that keeps the series monotonic — which is the single
property consumers read it for ("greatest blockNumber is the current balance").

Alternative rejected — leave the series to snapshots. It is what the first cut did, and it leaves the pool rows
and the balance rows disagreeing between windows: the depth knows the inventory is spent while the newest balance
row still reports it, and those two are meant to be the same measurement.

Alternative rejected — overwrite the newest existing row in place. No RPC and no new key, but it rewrites history:
that row claims to be the balance at its Hyperbridge block, and after the overwrite it is not.

Alternative rejected — a fill-shaped key (`…-fill-{evmBlock}-…`). Append-only and honest about provenance, but it
puts two unrelated block sequences in one column, so the greatest-blockNumber rule stops meaning "latest".

The stamp is a borrowed clock, not a claim: the balance was read at the EVM chain's head, not reconstructed at
that Hyperbridge block, and the schema description now says so. Two readings landing on one key resolve to the
larger, the rule the sweep already follows — a refresh is always the V4-blind reading, so it must not replace a
complete one. The cost is that a second fill while Hyperbridge is still on the same block does not lower the row;
one block later it does.
