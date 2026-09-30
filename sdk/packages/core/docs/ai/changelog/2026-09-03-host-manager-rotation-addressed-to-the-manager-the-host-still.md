# 2026-09-03 — Host manager rotation addressed to the manager the host still trusts

`pallet-ismp-host-executive::update_host_params` applied the update before reading the request
recipient, so a rotation from one HostManager to another was addressed to the new one. The host
accepts `updateHostParams` only from its current manager, so the delivery reverted, Hyperbridge
recorded the new manager anyway, and every later host update and withdrawal for that chain was
sent to a contract the host did not trust. The recipient is now captured before the update; the
payload still installs the new manager. This is the path the HostManager relayer gate is rolled
out through. No file in this package changed.

Files: `modules/pallets/host-executive/src/lib.rs`,
`modules/pallets/testsuite/src/tests/pallet_ismp_host_executive.rs`,
`evm/tests/foundry/HostManagerTest.sol`, `docs/ai/Flow.md`.
