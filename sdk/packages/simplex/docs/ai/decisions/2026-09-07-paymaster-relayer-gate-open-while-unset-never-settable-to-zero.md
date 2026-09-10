# 2026-09-07 — Paymaster relayer gate: open while unset, never settable to zero

Chosen: `SimplexPaymaster.onAccept` accepts every relayer while `_relayer` is zero and only the
authorised one afterwards; `migrate` and the `SetRelayer` request refuse zero (`initialize` still
takes zero so tests can deploy an open paymaster; the deploy script refuses it).

Open-while-unset mirrors the gateway rather than the fail-closed BRIDGE token because the paymaster
has no owner key: a proxy upgraded without `migrate` under a closed gate could never be governed
again, and it holds the EntryPoint deposit, the stake and the fee surplus. Never-zero afterwards
follows the HostManager: a `SetRelayer(0)` can only be delivered by the current relayer, who could
deliver `SetRelayer(newKey)` instead, so zero has no recovery value and would only reopen the
contract to forged deliveries.

Accepted and not mitigated: once armed, every recovery path sits behind the gate, so a lost or
withholding relayer key strands the deposit, stake and surplus for good. A second key was rejected
because it reintroduces the privileged role the contract was designed without. Operational bounds:
the relayer is a plain EOA (the check is the handler's raw `msg.sender`, so an account executing
third-party calldata would let anyone through); sweep surplus and keep the deposit small with
`WithdrawAssets`; `swapAndDeposit` stays treasury-gated and is the one lockout-proof use of surplus;
never dispatch a second `SetRelayer` or `UpgradeContract` while one is undelivered, since requests
never expire and the relayer picks delivery order, so a superseded `SetRelayer(A)` delivered last
hands A the sole key. Confirm delivery with `requestReceipts(commitment)` first.
