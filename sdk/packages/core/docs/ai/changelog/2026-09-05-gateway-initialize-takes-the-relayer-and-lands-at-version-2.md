# 2026-09-05 — Gateway `initialize` takes the relayer and lands at version 2; `migrate` for older proxies

`IntentGatewayV2.initialize(Params, bytes[] peerChains, address relayer)` now arms the relayer
gate from the init data and runs under `reinitializer(VERSION)` with `VERSION = 2`, so a fresh
proxy comes out armed and at the version of the code it runs. `migrate(address relayer)`, host-only
and under the same `reinitializer(VERSION)`, is for proxies deployed before this implementation:
it arms them and takes them from 1 to 2, and reverts on a proxy `initialize` already took there.
`setRelayer` stays a plain host-only rotation that leaves the version alone; all three write
through `_setRelayer` in `ExtrinsicIntents`. The next implementation that needs a migration bumps
`VERSION` once. The interface documents `migrate` and `version` accordingly.

`DeployIntentGateway.s.sol` always deploys the implementation and the solver account, deploys the
proxy only where `INTENT_GATEWAY_V2` is absent from the chain's config, reads the relayer from
`GATEWAY_RELAYER` for the init data, and records `INTENT_GATEWAY_V2_IMPL`. The relayer is now part
of what fixes a new proxy's address, as the implementation address already was.

The reinitializer cost more than the 71 bytes of EIP-170 headroom, and every gateway getter and
event has a consumer in `sdk`, `simplex` or the indexer, so the room came from deduplicating
internal code with no behaviour change: `_sendValue` in `IntentsBase` for the native
send-and-check (the same-chain fill loop keeps its inline copy, being at the via-ir stack limit),
`_splitSurplus` moved to `IntentsBase` and used by the cross-chain fill, `_withdrawalBody` and
`_postToSource` in `ExtrinsicIntents` for the escrow messages, and `placeOrder` reusing its
`feeToken` read and hashing the order once.

Tests: every `initialize` call gains the relayer argument, `address(0)` outside `setUp` so those
gateways stay open as before; `testInitializeArmsTheGate` pins the events and version; the
`migrate` tests run on a proxy written back to version 1 through the `Initializable` slot, since
this implementation cannot produce one; the live mainnet-fork upgrade migrates the real one.
`HostManager.onAccept` gained NatSpec.

Files: `contracts/apps/IntentGatewayV2.sol`, `docs/ai/ChangeLog.md`, `docs/ai/Decisions.md`,
`docs/ai/Flow.md`. Outside the package: `evm/src/apps/IntentGatewayV2.sol`,
`evm/src/apps/intentsv2/ExtrinsicIntents.sol`, `evm/src/apps/intentsv2/IntrinsicIntents.sol`,
`evm/src/apps/intentsv2/IntentsBase.sol`, `evm/src/core/HostManager.sol`,
`evm/script/DeployIntentGateway.s.sol`, `evm/tests/foundry/IntentGatewayV2Test.sol`,
`evm/tests/foundry/IntentGatewayV2SameChainTest.sol`,
`evm/tests/foundry/IntrinsicIntentsReentrancyTest.sol`,
`evm/tests/foundry/account/SolverAccountTest.sol`.
