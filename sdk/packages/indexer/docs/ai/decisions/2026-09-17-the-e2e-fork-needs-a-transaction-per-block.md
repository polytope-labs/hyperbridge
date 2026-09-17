# 2026-09-17 — The E2E's fork needs a transaction in every block

SubQuery's Ethereum node skips block handlers on empty blocks: `isFullBlock` calls a block with no
transactions light, and `indexBlockData` then runs only log handlers. An idle anvil fork produces
nothing else, so the solver-inventory E2E's entire block-handler path — watchlist consumption,
genesis reads, the head — silently never ran.

**Chosen: a heartbeat transaction per block.** One self-transfer from an anvil dev account, every
1.5 s against a 2 s block time. It reproduces the property every real chain already has, and leaves
the indexer untouched.
- Rejected: a `modulo: 1` filter on the handler. Tested, and it changes nothing — the filter is
  evaluated only once the block is already being treated as full.
- Rejected: automine (no `--block-time`), where a block is produced per transaction. It would also
  guarantee full blocks, but it makes block production depend entirely on the heartbeat, so a stalled
  sender stops the chain rather than merely skipping a block.
- Rejected: changing the indexer to tolerate empty blocks. There is nothing to fix — the handler
  behaves correctly on any chain whose blocks carry transactions.

**Known limit: the fork needs an archive-capable endpoint.** CI forks with the `BASE_MAINNET` secret.
Public Base RPCs either refuse historical state outright (`archive requests require a personal
token`) or rate-limit a forked node into unresponsiveness, which is what blocked this from being
validated end to end locally.
