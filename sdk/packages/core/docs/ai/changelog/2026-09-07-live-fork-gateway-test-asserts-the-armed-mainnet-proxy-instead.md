# 2026-09-07 — Live-fork gateway test asserts the armed mainnet proxy instead of replaying its migration

The mainnet IntentGatewayV2 proxy has been upgraded and armed (relayer set, version 2), so the
fork test that rehearsed that migration failed on its first precondition. It is replaced by
`testLiveProxyIsArmedAndGovernedOnlyByItsRelayer`: the live proxy reads armed and migrated with
the relayer packed behind an unset `_paused` in slot 13, refuses `initialize` and a second
`migrate`, refuses an `Execute` upgrade from anyone but its relayer, installs it with every
readable piece of state intact when the relayer delivers it, and rotates through `Execute`, after
which the previous relayer is out.

Files: `evm/tests/foundry/IntentGatewayV2Test.sol`, `docs/ai/Flow.md`.
