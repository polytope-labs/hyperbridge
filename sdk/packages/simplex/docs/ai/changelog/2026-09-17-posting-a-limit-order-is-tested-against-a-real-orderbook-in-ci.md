# 2026-09-17 — Posting a limit order is tested against a real orderbook in CI

`orderbook.live.test.ts` could only run where someone had built the orderbook themselves, so nothing
enforced it. It runs in CI now, against the published image, on any change to this package's
orderbook code.

`.github/workflows/test-simplex-orderbook.yml` starts `polytopelabs/hyperfx-orderbook` with the
config in `src/tests/fixtures/orderbook-ci.toml`, waits for it to answer a real query rather than
merely open a port, and runs the test against it. The container joins the runner's network, so both
sides reach each other on `127.0.0.1`, and the orderbook's logs are dumped whatever the outcome.
Unlike the other workflows here it has no `branches` filter: these PRs are a stack whose bases are
each other, and filtering on `main` would mean none of them ever ran it.

## No chain, but not `dev_mode` either

`scripts/fake-indexer.mjs` answers the two things the orderbook reaches out for: the Hyperbridge
indexer's `Inventory` query at `/graphql`, and each gateway's `params()` at `/rpc/<chain>`. It is
modelled on the orderbook's own harness, which mounts exactly those two endpoints, and it answers
generously: every solver holds plenty of every token, every solver is delegated, and the protocol fee
is whatever `--fee-bps` says.

The test used to run the server in `dev_mode`, where the validation cycle reads nothing and every
order surfaces unconditionally. With the stub answering, the cycle runs for real, which is both
closer to production and the only way `validatedAt` and `backed` ever become true here. The local
path, `HYPERFX_ORDERBOOK_BIN`, spawns the stub alongside the binary and needs no more setup than
before.
