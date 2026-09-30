# 2026-09-07 — APPROVE mode removed, on chain and in the client

Chosen: mode `0x01` is refused by the contract and the client never reads or creates an allowance
to the paymaster. Supersedes the 2026-08-18 entries "Permit2 before a legacy paymaster allowance,
bootstrap approves Permit2" (APPROVE was kept while a legacy allowance drained), "`forceApproveMode`
renamed to `skipPermit`" (APPROVE stayed available) and "Known: PERMIT2 mode is not
ERC-7562-clean" (APPROVE was the fallback for a spec-enforcing bundler).

Why: a standing allowance to the paymaster is exactly the exposure the security model bounds by
keeping amounts small; Permit2 gives a per-op, single-use authorisation with nothing at rest, and
a permit token leaves at most the permit residue. Consequences accepted: a permit token now always
signs a permit, so a solver who kept a manual $5 allowance loses the one case where concurrent bids
on a chain did not share the sequential EIP-2612 nonce (every other solver already did); a
front-run permit still loses the bid, and no allowance fallback was added to `_executePermit`
because that would be APPROVE by the back door; the ERC-7562 fallback no longer exists (mode 0 was
never clean either: it reads `block.timestamp` and executes an external permit), accepted as
availability risk. When a no-permit token meets an unusable Permit2 (unconfigured, or a paymaster
that does not expose `PERMIT2()`), the builder throws instead of sending a native-funded approve;
`buildPaymasterAndData` already demotes a throw to a skip reason and Circle or the deposit follow.
