# 2026-09-03 — Governance deliveries to `HostManager` gated on the same relayer

Closes the route around the app-level gates: `HostManager.onAccept` accepted `SetHostParam` from
any relayer, and that request can replace the host's handler, the contract every app trusts to
report the relayer address. `HostManager` now holds `_relayer`, set by the host admin through
`setRelayer`, and `onAccept` reverts with `UnauthorizedRelayer` for any other relayer, zero
included. Only governance traffic reaches this contract, so ordinary relaying is unaffected.
No file in this package changed; the entry is here because the delivery flow documented in
`Flow.md` is what it corrects.

Files: `evm/src/core/HostManager.sol`, `evm/script/DeployIsmp.s.sol`,
`evm/script/DeployHostManager.s.sol`, `evm/tests/foundry/HostManagerTest.sol`,
`evm/tests/foundry/BaseTest.sol`, `evm/tests/rust/src/tests/utils.rs`, `docs/ai/Decisions.md`,
`docs/ai/Flow.md`.
