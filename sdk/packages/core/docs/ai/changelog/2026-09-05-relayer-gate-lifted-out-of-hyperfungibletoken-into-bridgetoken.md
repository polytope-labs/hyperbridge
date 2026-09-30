# 2026-09-05 — Relayer gate lifted out of `HyperFungibleToken` into `BridgeToken`

`HyperFungibleToken` no longer carries `_relayer`, `relayer()`, `setRelayer`, `RelayerUpdated`,
`UnauthorizedRelayer` or the `_checkRelayer` hook; `onAccept` and `onPostRequestTimeout` are
`public virtual` so a token can gate deliveries before calling `super`. `BridgeToken` owns all of
that now: it overrides both callbacks with `onlyHost`, then `_checkRelayer`, then `super`, and
`_checkRelayer` fails closed as before. The base-token relayer tests in
`HyperFungibleTokenTest.sol` are gone with the feature; `BridgeTokenTest.t.sol` references the
error and event on `BridgeToken`. Storage of non-upgradeable `HyperFungibleToken` deployments
shifts by one slot, which only matters for a contract that reads it by slot.

Files: `contracts/apps/HyperFungibleToken.sol`, `docs/ai/ChangeLog.md`, `docs/ai/Decisions.md`,
`docs/ai/Flow.md`. Outside the package: `evm/src/apps/BridgeToken.sol`,
`evm/tests/foundry/HyperFungibleTokenTest.sol`, `evm/tests/foundry/BridgeTokenTest.t.sol`.
