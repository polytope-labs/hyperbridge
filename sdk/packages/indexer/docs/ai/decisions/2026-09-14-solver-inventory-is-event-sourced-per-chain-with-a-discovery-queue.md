# 2026-09-14 — Solver inventory is event-sourced per chain, and discovered by fills and a queued watchlist (#1264)

The setting: the HyperFX orderbook validates the orders it publishes against each solver's balance and
EIP-7702 delegation, and reads both from this indexer. Until now the indexer tracked them only as a by-product of
phantom-order indexing, which this same change removes.

**Chosen: discovery is a request queue.** The Hyperbridge node polls the orderbook's `GET /solvers` and writes
`SolverDiscoveryRequest` rows. Each EVM node turns its chain's rows into `TrackedSolver` rows, so everything
derived from them has one writer, as #1214 established for the pool family. Three alternatives lost:
- **Every EVM node polls for its own chain.** That multiplies the requests per block by the number of chains, for
  a list one request already returns whole.
- **The Hyperbridge node writes `TrackedSolver` directly.** That gives the rows two writers — the fill handler is
  the other — with the stale `get` cache #1214 describes on both sides.
- **Transfers of supported tokens discover solvers.** Every counterparty becomes a candidate, and each unknown one
  costs an `eth_getCode` to rule out, so the bill scales with token activity rather than with solvers.

**Chosen: requests come in versioned batches, with a one-row `SolverWatchlist` per chain.** The store's
`getByFields` has no range operators, so "requests newer than my cursor" cannot be asked directly. Asking "is the
chain's watchlist version ahead of my cursor?" costs two reads on a quiet block; when it is, one query per
missing version reads only the new rows. `SolverWatchlist` is read by field query even though it is keyed by
chain, because another node writes it.

**Chosen: the genesis read runs in the EVM block handler, never in the fill handler.** Fill handlers stay
store-only. Two facts combine:
- The block handler runs before the block's logs.
- A read pinned to block N includes every log of N.

So one rule positions everything: a row carries (`blockNumber`, `lastLogIndex`), and a read sets the index to the
largest Int. That single comparison covers the genesis boundary, a reconciliation, and a duplicate delivery.
Rejected: reading in the fill handler with "skip logs at or before N". It is correct too, but it puts per-token RPC
on the fill path, whose failure would either fail the fill or be swallowed.

**Chosen: `SolverVaultShares` of its own, rather than reusing `VaultLpPosition.shares`.** A `VaultLpPosition`
exists only after a vault event by an LP the yield ledger already knows, and its opening balance needs that
triggering log. A solver discovered while holding shares it has not moved since would have no row. The two also
gate differently: the ledger requires delegation or an existing position, while inventory tracks every
discovered solver. Share Transfers are applied for tracked solvers before the ledger's own ordinary-transfer
filter, so mints and burns count.

**Chosen: transfers are filtered by an in-memory set of tracked solvers.** A supported token's datasource
delivers every holder's transfers, so dropping the untracked ones must cost no store read and no RPC. The set is
complete, because a solver only becomes `TRACKED` in the same process. A rollback can leave a stale member, and
the store read behind the filter then finds its row missing.

**Chosen: a per-chain `SolverInventoryHead`, advanced at most every 30 s of block time.** Event-sourced rows only
change when something moves, while the orderbook withholds a balance measured more than 120 s ago. Without a
liveness signal, every quiet solver would be withheld. Rejected: bumping every row's `refreshedAt` on a timer.
Writes would scale with solvers × tokens, historical mode makes each one a new row version, and it would
misrepresent what `refreshedAt` means.

**Chosen: revalue hourly, reconcile daily, and re-anchor on reconciliation while recording the drift.** Vault
yield accrues without an event, and a running total never heals a missed event, so a daily pinned read bounds the
damage. `lastReconciledDrift` plus a warning make it visible. Rejected: recording the drift but keeping the wrong
total, which knowingly publishes a figure the chain contradicts. Delegation is re-checked at the same daily read,
since a 7702 authorization emits nothing.

**Chosen: the poll runs only near chain head, judged against wall clock.** Without that guard, a resync from
genesis would replay one fetch per historical block of a list that only describes now. Fetching an external list
is not replayable, and that is tolerable here because it only decides *when* tracking starts: the genesis read is
pinned to whichever block that turns out to be. The ETag is kept only once every chain has been queued, so a
store failure cannot let the next poll 304 past a list that was never applied. `safeFetch` also gains a timeout
(30 s by default, 5 s for this poll); without one, a peer that accepts and never answers hangs the handler.

Known limits, stated rather than solved:
- **Busy tokens.** A supported token with heavy traffic (USDC on Ethereum or Base) sends every Transfer log
  through the handler. It is dropped in memory, but fetching and decoding it still costs.
- **Every-block handler.** The EVM block handler has no `modulo`, so the node fetches every block. Most blocks on
  those chains carry a supported-token Transfer anyway.
- **Non-standard tokens.** A rebasing or fee-on-transfer token drifts until the next reconciliation.
- **Archive RPC.** Genesis and reconciliation reads during a resync need an archive-capable endpoint, as vault
  opening balances already do.
- **Stale delegation.** A revoked delegation is noticed within a day.
- **Additive schema.** Every new entity is additive.
