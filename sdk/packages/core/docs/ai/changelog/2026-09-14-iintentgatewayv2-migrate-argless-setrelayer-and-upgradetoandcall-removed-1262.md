# 2026-09-14 — `IIntentGatewayV2`: `migrate()` takes no argument; `setRelayer` and `upgradeToAndCall` removed (#1262)

The gateway implementation is split into delegatecall modules. `migrate()` now only bumps the
`Initializable` version (2 to 3 for this release); every live proxy is already armed, so it no
longer takes a relayer. `setRelayer` and `upgradeToAndCall` moved off the implementation onto the
`ExtrinsicModule`, reached only through the `Execute` governance action, so a call to the proxy
with either selector finds no function and they leave the published interface. `relayer()` stays.
The implementation's `_owner` immutable, a placeholder that gated nothing, is gone too; the
constructor now takes only the two module addresses. The new `intrinsicModule()` and
`extrinsicModule()` getters are declared, and `setParams(Params)`, which no gateway has had, is
dropped.

Files: `contracts/apps/IntentGatewayV2.sol`, `package.json`,
`docs/ai/changelog/2026-09-14-iintentgatewayv2-migrate-argless-setrelayer-and-upgradetoandcall-removed-1262.md`,
`docs/ai/decisions/2026-09-14-host-only-governance-functions-left-the-interface-rather-than-stubs.md`,
`docs/ai/flows/how-a-cross-chain-delivery-reaches-the-gateway-and-where-the.md`.
