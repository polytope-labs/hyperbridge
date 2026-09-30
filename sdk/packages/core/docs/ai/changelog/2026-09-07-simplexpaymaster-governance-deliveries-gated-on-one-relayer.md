# 2026-09-07 — SimplexPaymaster governance deliveries gated on one relayer; APPROVE mode removed

`SimplexPaymaster.onAccept` checks `incoming.relayer` against a new `_relayer` (slot 8, gap 48)
after `onlyHost` and before the source check. Armed by a fifth `initialize` argument, by the
host-only `migrate(relayer)` as upgrade init data (version 1 to 2, `onlyFresh` on `initialize`), or
rotated by `RequestKind.SetRelayer`; unset means open, zero can never be set by governance. Mode
byte `0x01` is refused. Deploy script reads `GOVERNANCE_RELAYER`; new impl-only deploy script for the
governance upgrade of the live proxies. Pallet extrinsic `set_paymaster_relayer` added.

Files: `evm/src/utils/SimplexPaymaster.sol`, `evm/script/DeploySimplexPaymaster.s.sol`,
`evm/script/DeploySimplexPaymasterImpl.s.sol`, `evm/script/SimplexPaymasterPermit2Probe.s.sol`,
`evm/tests/foundry/SimplexPaymaster*.t.sol`, `modules/pallets/intents-coprocessor/src/*.rs`,
`docs/ai/Flow.md`.
