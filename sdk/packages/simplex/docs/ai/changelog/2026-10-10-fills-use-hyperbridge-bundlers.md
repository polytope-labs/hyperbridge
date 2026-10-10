# 2026-10-10 — 0.18.0: fills always use Hyperbridge's bundlers

Simplex no longer takes a bundler from its config. It submits every fill through the ERC-4337
bundler Hyperbridge runs for the chain. `HYPERBRIDGE_BUNDLER_URLS` in `src/config/bundlers.ts` lists
them: Ethereum, BSC, Polygon, Base and Arbitrum, plus BSC Chapel and Polygon Amoy on testnet.
`resolveChainConfigs` sets each resolved chain's `bundlerUrl` from that list.

A chain Hyperbridge runs no bundler for can only be watched. Unless it is watch-only, the bundler
preflight refuses it at boot, when it is added, and when watch-only is turned off for it.
`PUT /api/chains` rejects it the same way.

Older configs still load. A `bundlerUrl` in `[[chains]]` is ignored, and `emitFillerToml` no longer
writes one, so the next save drops it.

Removed:

- `ChainInput.bundlerUrl` and `Simplex.chains.setBundlerUrl`.
- `FillerConfigService.setBundlerUrl`.
- The `simplex init` bundler step and its Pimlico helpers.
- `POST /api/setup/validate-bundler`, and `bundlerUrl` from the `validate-alchemy-key` results.

The wizards and the Chains panel offer only chains with a Hyperbridge bundler (`chainsForNetwork`),
so the testnet catalog is BSC Chapel and Polygon Amoy. `GET /api/chains` and `chains.list()` still
report each chain's `bundlerUrl`, now Hyperbridge's, and empty where there is none.

Simplex-desktop moves to 0.18.0 with it.
