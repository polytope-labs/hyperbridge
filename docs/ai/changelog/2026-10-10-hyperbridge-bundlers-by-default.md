# 2026-10-10 — SDK 3.0.1 and simplex 0.17.1: Hyperbridge's bundlers by default

Hyperbridge runs ERC-4337 bundlers on Ethereum, BSC, Polygon, Base and Arbitrum, and on BSC Chapel
and Polygon Amoy for testnet. They need no API key and serve EntryPoint v0.8 and v0.9. The SDK and
simplex now use them without being told to.

## SDK

`ChainConfigData.bundlerUrl` in `src/configs/chain.ts` holds each chain's Hyperbridge bundler.
`EvmChain.bundlerUrl` returns the bundler the chain was given, or else that one. It is undefined on
a chain Hyperbridge runs no bundler for.

`EvmChainParams.bundlerUrl` and the second argument to `EvmChain.create()` stay optional, and a URL
passed there still wins. `IntentGateway` reads its bundler from the destination chain, so an
`IntentGateway` on a supported chain now submits fills and estimates fill gas through Hyperbridge's
bundler with no configuration.

## Simplex

Simplex no longer takes a bundler from its config. It submits every fill through Hyperbridge's
bundler for the chain. `HYPERBRIDGE_BUNDLER_URLS` in `src/config/bundlers.ts` lists them, and
`resolveChainConfigs` sets each resolved chain's `bundlerUrl` from it. The list repeats the SDK's
because the dashboard bundles that module without the SDK. A test checks the two match.

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

The testnet swap E2E no longer takes `E2E_BSC_TESTNET_BUNDLER_URL` or `E2E_POLYGON_AMOY_BUNDLER_URL`.
Its SDK client uses the default bundlers.

Simplex-desktop moves to 0.17.1 with simplex.
