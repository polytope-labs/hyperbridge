# 2026-09-04 — Fresh gateway deployments arm the relayer in the deploy script

`DeployIntentGateway.s.sol` deployed the gated gateway with `_relayer` unset, so a proxy on a new
chain refused every delivery, including the `upgrade_gateway` message that could have armed it;
only the owner key could, and nothing called it. The script now reads `GATEWAY_RELAYER`, requires
the deploy key to be the admin (the only caller of `setRelayer`), calls `setRelayer` right after
the proxy is deployed, and asserts the relayer afterwards. `initialize` is unchanged so the
deterministic proxy address is unchanged. The gateway constructor now rejects a zero owner, since
a mis-set `ADMIN` would leave no key able to arm a fresh proxy; runtime size is unaffected. No
file in this package changed.

Files: `evm/script/DeployIntentGateway.s.sol`, `evm/src/apps/IntentGatewayV2.sol`,
`evm/tests/foundry/IntentGatewayV2Test.sol`, `docs/ai/Flow.md`.
