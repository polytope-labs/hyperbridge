# 2026-09-17 — Replace the phantom-filler CI with a solver-inventory E2E

The phantom-filler E2E was the CI job for the order flow this package no longer indexes. It is gone, and a new
workflow exercises what replaced it: the path the HyperFX orderbook actually depends on.

`.github/workflows/test-solver-inventory.yml` runs the whole chain of custody on every indexer change:

1. **A stand-in orderbook** (`scripts/tests/fake-orderbook.cjs`) serves `GET /solvers` in the shape the real server
   publishes — `{ chains: [{ chain, solvers: [{ address }] }] }`, with an ETag and `304` for a matching
   `If-None-Match`, verified against `crates/server/src/app.rs` in hyperfx-orderbook.
2. **An anvil fork of Base** gets three solvers (`scripts/tests/seed-solvers.cjs`): a USDC balance written straight
   into the token's balance slot, and an EIP-7702 designator as the account's code — one pointing at a known
   SolverAccount, one at a stranger, one absent.
3. **The indexer** runs against that fork with the live Gargantua testnet as its Hyperbridge node, which is what
   polls the watchlist.
4. **The assertions** (`scripts/tests/verify-solver-inventory.cjs`) require every row to be `discoveredBy:
   WATCHLIST`, the genesis read to match what was seeded, delegation to be true only for the SolverAccount, and
   then a real USDC Transfer to move both of its tracked sides while the third solver stays put.

Supporting wiring: a `solver-ci` environment (`src/configs/config-solver-ci.json`, `Environment` in
`src/configs/index.ts`, `start:solver-ci`), its compose file, and `generate-chain-yamls.ts` taking the live head for
it, since an anvil fork starts wherever it forked.

Verified locally against a Base fork: the seeding reads back through `balanceOf` and `eth_getCode`, and the Transfer
mines and lands the three wallets on exactly the amounts the assertions expect. The indexer half runs in CI, which
has the Base and Gargantua endpoints.

Files: `.github/workflows/test-solver-inventory.yml`, `.github/workflows/test-sdk.yml`,
`scripts/tests/solver-fixtures.cjs`, `scripts/tests/fake-orderbook.cjs`, `scripts/tests/seed-solvers.cjs`,
`scripts/tests/verify-solver-inventory.cjs`, `src/configs/config-solver-ci.json`, `src/configs/index.ts`,
`scripts/generate-chain-yamls.ts`, `docker/docker-compose.solver-ci.yml`, `package.json`
