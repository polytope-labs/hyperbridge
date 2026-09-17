# 2026-09-17 — Keep the E2E fork's blocks non-empty

The solver-inventory E2E's block handler ran exactly once and never again, so nothing was ever
discovered, seeded or advanced.

SubQuery's Ethereum node only runs block handlers on blocks it treats as full
(`indexer.manager.ts`: `if (isFullBlock(block))`), and `isFullBlock` in `block.ethereum.js` returns
**false for a block with no transactions** — an empty block is indistinguishable from a light one, so
it is treated as light and only log handlers run. An idle anvil fork mines nothing but empty blocks.

That explained the evidence exactly: the handler ran on the first indexed block, which came from real
Base and carried transactions, and never on the locally-mined empty ones after it.

`scripts/tests/anvil-heartbeat.cjs` now sends one self-transfer per block from an anvil dev account,
and the workflow starts it after seeding. Every block then carries a transaction, as on any real
chain.

Nothing in the indexer changed: on Base, Ethereum or any live chain, blocks carry transactions and
the handler fires normally. This is a property of an idle fork, not of the handler.

Files: `scripts/tests/anvil-heartbeat.cjs`, `.github/workflows/test-solver-inventory.yml`
