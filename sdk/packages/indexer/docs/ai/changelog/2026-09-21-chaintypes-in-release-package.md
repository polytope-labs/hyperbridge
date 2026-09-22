# 2026-09-21 — Chain types in the release package

The release tarball ships `dist/`, `src/configs` and `scripts`, but not `src/substrate-chaintypes`.
Operators run `npm run codegen` from that package. Since #1266, `generate-chain-yamls.ts` decides whether
to write the `chaintypes` line by looking for `src/substrate-chaintypes/<chain>.ts`, so manifests
generated from a release (from `indexer-testnet-v3.8.0` on) had no `chaintypes`, and the Hyperbridge
node could not decode its blocks.

The generator now writes `chaintypes: ./dist/substrate-chaintypes/<chain>.js` when either the source
or that compiled file exists. A checkout has the source before `subql build` writes dist, and the
release package has dist without the source.
