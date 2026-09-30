# 2026-09-03 — Relayer allowlist on the intent gateway

The gateway now accepts `onAccept` and `onGetResponse` deliveries only from a single authorised
relayer stored at `_relayer` (slot 13, packed behind `_paused`). The check runs before the message
body is decoded, so escrow redemptions, refunds and every governance action, upgrades included, are
covered. A refused delivery reverts, which the host records as undelivered, so the authorised
relayer can submit the same message later. `setRelayer(address)` is callable by the immutable
`_owner` and by the host; the host branch exists so a governance `UpgradeContract` can carry the
call as its migration calldata and arm the relayer in the upgrade transaction (`upgradeToAndCall`
delegatecalls that calldata with the host still as `msg.sender`).

The interface gains `RelayerUpdated(address previous, address current)` and `setRelayer`, keeping
its declarations identical to `IntentsBase`. The unused `_paused` getter was dropped from the gateway
to stay under the EIP-170 size limit; it was never declared here.

Files: `contracts/apps/IntentGatewayV2.sol`, `package.json`, `docs/ai/ChangeLog.md`,
`docs/ai/Decisions.md`, `docs/ai/Flow.md`. Gateway side: `evm/src/apps/IntentGatewayV2.sol`,
`evm/src/apps/intentsv2/IntentsBase.sol`, `evm/src/apps/intentsv2/ExtrinsicIntents.sol`,
`evm/tests/foundry/IntentGatewayV2Test.sol`.
