# 2026-09-21 — Solver inventory on testnet

Testnet builds now index solvers the way mainnet does. The generated manifests no longer gate on the
environment:

- The Hyperbridge node (`hyperbridge-gargantua`) runs `handleSolverWatchlistPoll`, which polls
  `${HYPERFX_ORDERBOOK_URL}/solvers`.
- Every EVM chain with `yieldVaults` gets one `handleSolverTokenTransferEvent` datasource for each
  supported token, plus the `handleSolverInventoryBlock` block handler. On testnet that means
  BSC Chapel (USDC, USDT) and Polygon Amoy (USDC).

The `enableSolverDiscovery` and `enableSolverInventory` template flags are removed. Discovery follows
`isHyperbridgeChain`, and inventory follows `supportedTokens`.

`config-testnet.json` lists the current testnet `SolverAccount`
(`0x153DB990FE3b761B54ad71D0f1A0987dF11740DB`) under `solverAccount` for EVM-97 and EVM-80002. Solver
EOAs delegating to it now count for `parseDelegation` there.

The substrate node's compose service passes `SUBQL_ALLOW_DESTRUCTIVE_MIGRATION` through from the
host environment, and it is empty unless set. This applies to the generated services (the ones with
`--allow-schema-migration`) and to `docker-compose.{local,nexus-ci,solver-ci}.yml`. A destructive
schema change can then be allowed for one restart with `SUBQL_ALLOW_DESTRUCTIVE_MIGRATION=true`,
without editing the compose file.
