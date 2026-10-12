# 2026-10-09 — Stop indexing Optimism, Gnosis, Optimism Sepolia, Pharos Atlantic and Tron Nile

`optimism-mainnet` and `gnosis-mainnet` are removed from `src/configs/config-mainnet.json`, and
`optimism-sepolia`, `pharos-atlantic` and `tron-nile` from `src/configs/config-testnet.json`.

The release no longer generates a chain manifest, a `subquery-multichain.yaml` project entry or a
`docker/<env>/<chain>.yml` service for these chains, even when an endpoint such as `OPTIMISM_MAINNET` is
still set in the environment, so a deploy cannot bring their indexer containers back. `ENV_CONFIG`
(`src/env-config.json`) no longer carries an RPC URL for `EVM-10`, `EVM-100`, `EVM-11155420`, `EVM-688689`
or `EVM-3448148188`, the same as for any configured chain whose endpoint is unset.

Rows already written for these chains stay in the database and remain queryable. Events emitted on
Hyperbridge itself, such as `StateMachineUpdated` for these state machines, are still indexed by the
Hyperbridge indexer.
