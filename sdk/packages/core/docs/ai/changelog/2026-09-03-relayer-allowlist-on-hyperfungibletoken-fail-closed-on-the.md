# 2026-09-03 — Relayer allowlist on `HyperFungibleToken`, fail-closed on the BRIDGE token

`HyperFungibleToken` gains `_relayer`, `relayer()`, `setRelayer(address)` (owner only), the
`RelayerUpdated(address previous, address current)` event, the `UnauthorizedRelayer` error, and a
virtual `_checkRelayer` that `onAccept` and `onPostRequestTimeout` call before anything else. Both
callbacks mint, so both are gated. In the base the check is opt-in: zero leaves deliveries open, so
tokens already deployed from this package behave as before until their owner sets a relayer.
`evm/src/apps/BridgeToken.sol` overrides `_checkRelayer` to fail closed, and its deploy script sets
the relayer from `GOVERNANCE_RELAYER` before `configure`, so the token is never live without one.
`IHyperFungibleToken` is unchanged: `supportsInterface` keys on its `interfaceId`, so adding the
new functions there would change what every existing deployment reports.

Files: `contracts/apps/HyperFungibleToken.sol`, `docs/ai/ChangeLog.md`, `docs/ai/Decisions.md`,
`docs/ai/Flow.md`. Outside the package: `evm/src/apps/BridgeToken.sol`,
`evm/script/DeployBridgeToken.s.sol`, `evm/tests/foundry/BridgeTokenTest.t.sol`,
`evm/tests/foundry/HyperFungibleTokenTest.sol`.
