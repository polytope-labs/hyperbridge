# 2026-09-07 — 0.14.0: relayer-gated paymaster governance; APPROVE mode removed

`SimplexPaymaster.onAccept` now refuses any delivery whose `incoming.relayer` is not the one
authorised relayer, checked right after `onlyHost` and before the Hyperbridge source check, so a
forged consensus proof alone can no longer reach governance (upgrades, params, withdrawals). The
relayer lives in a new storage slot 8 (`_relayer`, the gap shrinks to 48 words; slots 0 to 7 are
unchanged for the live proxies). It is armed by a fifth `initialize` argument on a bare proxy, by
the host-only `migrate(relayer)` delivered as the init data of an `UpgradeContract` request on a
proxy from before the gate (Initializable version 1 to 2, `onlyFresh` keeps `initialize` off such
a proxy), and rotated by the new `RequestKind.SetRelayer = 7`. An unset relayer leaves the gate
open; `migrate` and `SetRelayer` refuse zero. `version()` and `relayer()` views added.

Mode byte `0x01` (a standing allowance to the paymaster) is refused with `InvalidMode(1)`; only
PERMIT (`0x00`) and PERMIT2 (`0x02`) remain. The client drops the APPROVE branch, the permit-mode
allowance short-circuit and the `approve(paymaster, $5)` bootstrap: a permit token always signs a
permit, a no-permit token needs Permit2 (bootstrapped once with `approve(Permit2, max)`), and when
Permit2 is unusable the builder throws an actionable error that `buildPaymasterAndData` demotes to
a skip reason. `VERIFICATION_GAS_LIMIT_APPROVE` is gone.

Runtime: `pallet-intents-coprocessor` gains `RequestKind::PaymasterSetRelayer` and the
`set_paymaster_relayer` extrinsic (call index 20, refuses zero, weighed as `upgrade_paymaster`).
Deploy: `DeploySimplexPaymaster.s.sol` reads `GOVERNANCE_RELAYER` and asserts the arm; new
`DeploySimplexPaymasterImpl.s.sol` deploys an implementation only, for the governance upgrade of
the live proxies. Release ordering: publish this version only after the live proxies are upgraded
(see Decisions).

Files: `evm/src/utils/SimplexPaymaster.sol`, `evm/script/DeploySimplexPaymaster.s.sol`,
`evm/script/DeploySimplexPaymasterImpl.s.sol`, `evm/script/SimplexPaymasterPermit2Probe.s.sol`,
`evm/tests/foundry/SimplexPaymasterTest.t.sol`, `evm/tests/foundry/SimplexPaymasterGasGriefTest.t.sol`,
`evm/tests/foundry/SimplexPaymasterPermit2ForkTest.t.sol`,
`modules/pallets/intents-coprocessor/src/{lib,types,tests}.rs`,
`src/services/paymaster/types.ts`, `src/services/paymaster/provider/simplex.ts`,
`src/services/UserOpSender.ts`, `src/tests/services/SimplexPaymaster.test.ts`,
`src/tests/services/UserOpSender.test.ts`, `package.json`, `CHANGELOG.md`, `docs/ai/*.md`.
