# 2026-09-07 — The paymaster relayer is `GOVERNANCE_RELAYER`, and the release order behind it

Chosen: `DeploySimplexPaymaster.s.sol` reads `GOVERNANCE_RELAYER`, the same key the HostManager and
BridgeToken deploy scripts arm, refused when zero. A dedicated `PAYMASTER_RELAYER` was considered
and dropped in review: paymaster governance is delivered by the same relayer as the rest of
Hyperbridge governance, so a separate env only added a way to misconfigure it.

Release order: live Ethereum, Base and Polygon proxies predate `PERMIT2()`, and the configured BSC
and Arbitrum addresses have no code. The filler never funds an EntryPoint deposit on a chain with a
Simplex address configured and `prepareBidUserOp` still submits a bid when selection ends with no
paymaster, so a USDT-only solver on those chains is unsponsored in either ordering: this client
before the proxy upgrade throws in the builder, the previous client after the upgrade sends mode 1
and fails validation. USDC has permit on all three, so it is USDT-only solvers either way. Rule:
deploy the implementation with `DeploySimplexPaymasterImpl.s.sol`, `upgrade_paymaster` with
`migrate(relayer)` init data on the three proxies, fresh deploys on BSC and Arbitrum, and only then
tag `simplex-v0.14.0`. A guard that skips the bid when no paymaster is usable was left out as a
separate behaviour change.
