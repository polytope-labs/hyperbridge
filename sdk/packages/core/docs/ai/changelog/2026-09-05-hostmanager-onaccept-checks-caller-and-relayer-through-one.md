# 2026-09-05 — `HostManager.onAccept` checks caller and relayer through one modifier

`restrict` takes the address to check and the address it must equal, and `onAccept` carries it
twice: `restrict(msg.sender, _params.host)` and `restrict(incoming.relayer, _params.admin)`.
Both failures revert with `UnauthorizedAction`; the separate `UnauthorizedRelayer` error is gone,
and the Foundry and Rust tests expect `UnauthorizedAction` for a wrong relayer. `init` uses the
same two-argument form. `HostManager.json` regenerated.

Files: `docs/ai/ChangeLog.md`. Outside the package: `evm/src/core/HostManager.sol`,
`evm/tests/foundry/HostManagerTest.sol`, `evm/tests/rust/src/tests/host_manager.rs`,
`evm/rust/abi/HostManager.json`.
