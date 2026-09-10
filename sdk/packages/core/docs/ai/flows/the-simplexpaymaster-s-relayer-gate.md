# The SimplexPaymaster's relayer gate

Verified against `evm/src/utils/SimplexPaymaster.sol` and exercised by the relayer-gate cases in
`evm/tests/foundry/SimplexPaymasterTest.t.sol`.

Steps 1 and 2 of the gateway route apply unchanged; the paymaster's only ISMP entry point is
`onAccept`, which carries governance (upgrades, params, token registry, withdrawals, stake). It runs
`onlyHost`, then `_checkRelayer(incoming.relayer)`, then the Hyperbridge source check, and reads the
kind byte only after all three. As on the gateway, an unset relayer gates nothing, so a proxy
upgraded without arming stays reachable; unlike the gateway, governance can never set zero. A bare
proxy is armed through the relayer argument of `initialize`; a proxy from before the gate through
`migrate(relayer)` as the init data of the upgrade request, host-only and one-shot; rotation through
the `SetRelayer` request kind. The client-side and rollout detail lives in
`sdk/packages/simplex/docs/ai/Flow.md`.
