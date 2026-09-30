# 2026-09-05 — Gateway `initialize` refused on any proxy already at a version

`initialize` carries an `onlyFresh` modifier that reverts with `InvalidInitialization` unless the
`Initializable` version is 0. Without it, an upgrade that installed this implementation on a
version-1 proxy without running `migrate` would leave `initialize`, which has no caller
restriction, open to anyone until governance caught up. Now the host-only `migrate` is the only
way up for such a proxy. `testInitializeRefusedOnLegacyProxy` plays it. The interface NatSpec for
`migrate` says so.

Files: `contracts/apps/IntentGatewayV2.sol`, `docs/ai/ChangeLog.md`, `docs/ai/Decisions.md`,
`docs/ai/Flow.md`. Outside the package: `evm/src/apps/IntentGatewayV2.sol`,
`evm/tests/foundry/IntentGatewayV2Test.sol`.
