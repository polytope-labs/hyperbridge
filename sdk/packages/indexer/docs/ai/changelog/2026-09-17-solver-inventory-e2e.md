# 2026-09-17 — Solver inventory E2E

`.github/workflows/test-solver-inventory.yml` runs the path the HyperFX orderbook depends on against
real components, and replaces the phantom-filler E2E, whose indexing half this package no longer has.

What it runs, on every indexer change:

1. **A stand-in orderbook** (`scripts/tests/fake-orderbook.cjs`) serves `GET /solvers` in the shape the
   real server publishes — `{ chains: [{ chain, solvers: [{ address }] }] }`, an ETag over the body,
   `304` for a matching `If-None-Match`, `400` for an unconfigured `?chain=`.
2. **An anvil fork of Base** (`scripts/tests/seed-solvers.cjs`) carries three solvers: a USDC balance
   written straight into the token's balance slot, and an EIP-7702 designator as the account's code —
   one pointing at a known SolverAccount, one at a stranger, one absent. A storage write emits no
   `Transfer`, so those balances can only reach the indexer through its genesis read.
3. **The indexer**, in the `solver-ci` environment (`src/configs/config-solver-ci.json`,
   `docker/docker-compose.solver-ci.yml`), with the live Gargantua testnet as the Hyperbridge node that
   polls the watchlist.
4. **The assertions** (`scripts/tests/verify-solver-inventory.cjs`) require every row to be
   `discoveredBy: WATCHLIST`, the genesis read to match what was seeded, `delegated` true only for the
   SolverAccount, and then one real USDC `Transfer` to move both of its tracked sides while the third
   solver's row stays where it was.

Five environment constraints hold this together. Each fails the same way when broken — no rows, no
error — so they are stated here rather than rediscovered:

- **Chaintypes come from the source file.** `subql build` writes the compiled one at the end of the
  same build that generates manifests, so testing for the artifact omits the line on any clean
  checkout. A Hyperbridge node without it cannot decode its own blocks: its hasher is keccak.
- **The Hyperbridge node runs with `--unfinalized-blocks`.** It starts at the chain head, where the
  best header moves under it; otherwise it dies on an assertion in `UnfinalizedBlocksService` and,
  with `restart: always`, polls only between restarts.
- **Every block must carry a transaction** (`scripts/tests/anvil-heartbeat.cjs`). SubQuery treats a
  block with no transactions as light and runs no block handlers on it, and an idle fork mines nothing
  else. Live chains have this property already.
- **The query service starts after the substrate node**, which owns the schema's DDL. It reads the
  database schema once at startup and never again, so starting earlier leaves the entities off its
  `Query` type for the whole run.
- **The SDK is built first.** The indexer imports `@hyperbridge/sdk/intents-helpers`, which only
  exists once the SDK's node bundle is built.
- **The fork finalizes locally** (`--slots-in-an-epoch 1`). The indexer looks up `finalized` and
  `safe` every second; on a fork those reach upstream unless anvil can answer them from local state,
  and an anvil driven upstream every second stops answering anyone.

The fork needs an archive-capable endpoint (`BASE_MAINNET`). Public Base RPCs either refuse historical
state or rate-limit a forked anvil into unresponsiveness. anvil's own output is captured to `anvil.log`
and printed with the other logs, because an unresponsive fork is the failure mode this test has, and
its side of the story is otherwise missing.

Files: `.github/workflows/test-solver-inventory.yml`, `.github/workflows/test-sdk.yml`,
`scripts/tests/solver-fixtures.cjs`, `scripts/tests/fake-orderbook.cjs`, `scripts/tests/seed-solvers.cjs`,
`scripts/tests/anvil-heartbeat.cjs`, `scripts/tests/verify-solver-inventory.cjs`,
`src/configs/config-solver-ci.json`, `src/configs/index.ts`, `scripts/generate-chain-yamls.ts`,
`docker/docker-compose.solver-ci.yml`, `src/services/solverWatchlist.service.ts`, `package.json`,
`docs/ai/decisions/2026-09-17-the-solver-inventory-e2e-fakes-the-orderbook-and-forks-base.md`
