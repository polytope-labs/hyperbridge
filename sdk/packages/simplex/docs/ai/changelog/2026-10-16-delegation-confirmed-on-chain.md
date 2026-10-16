# 2026-10-16 — A delegation is confirmed on chain, and only a never-submitted op falls back

`DelegationService` used to read success from a transaction receipt alone and to fall back to a
native tx whenever the bundler path did not clearly succeed. EIP-7702 skips an authorization that
does not match the authority's nonce without reverting, so a receipt could read `success` while the
wallet stayed undelegated — and simplex would then bid from an EOA with no code, paying Hyperbridge
fees and locking deposits for fills that cannot execute. The changes below make delegation confirm
on chain and stop the paths that could double-delegate.

## Nonces

The tx nonce and the authorization tuple's nonce are two counters, and the tuple's is the one *after*
the tx's: viem fills a missing tx nonce from `pending`, and EIP-7702 compares the tuple against the
authority's nonce once the tx has bumped it. So a direct tx reads its nonce at `pending` and signs the
tuple at `nonce + 1`, and `sendDelegationTransaction` passes that nonce to `walletClient.sendTransaction`
explicitly in both the plain and the batched-approve branch. `revokeDelegation` does the same.

On the bundler path there is no tx from this wallet to claim, so the tuple is signed at the
authority's current nonce — but only once nothing else is in flight. `buildAuthorization` reads both
`latest` and `pending` and throws `The wallet has a transaction in flight on <chain>; the delegation
is signed once it has landed` while `pending > latest`; otherwise the tuple would go stale the moment
that tx mined. That throw happens inside `trySendSponsored`'s preparation try/catch, which reports the
op as never submitted (`null`), so the caller falls back to the native path, which handles an
in-flight tx correctly by reading `pending`.

While an EOA tx is in flight, then, the sponsored delegation waits and the native path queues behind
it at the same nonce. Receipt waits are bounded — the bundler confirmation waits up to 30 s for the
tx, and a native tx up to its own receipt timeout — so the per-wallet/chain lock below can hold for
up to that wait before the next attempt runs.

Every bundled signer honours the explicit nonce on a set-code tx: the viem-backed ones (private key,
secret phrase, Turnkey) serialize the nonce viem places on the prepared transaction, `digestSigner`
included; `mpcVaultSigner` serialises the whole transaction — nonce included — for its raw-signed
authorization-list branch, and passes it as `nonce` on the structured `evmSendCustom` branch.

## Confirmation

A success receipt no longer reports a delegation. `trySendDelegation` and `setupDelegationViaBundler`
read the code at the receipt's block (`getCode({ blockNumber })`) and compare it to the configured
SolverAccount; `revokeDelegation` confirms the code is empty there. A successful tx with no delegation
logs that the authorization was skipped and returns false. The bundler path waits for the tx's receipt
with `waitForTransactionReceipt` (bounded at 30 s) because the public RPC can lag the bundler's node;
a missing or unreadable receipt is not confirmation. A pinned block read failure is "not confirmed",
never a pass — the caller retries, and the next attempt's `isDelegated(latest)` sees a delegation that
did land.

## No native fallback after a submitted op

`setupDelegationViaBundler` returns `delegated | unavailable | pending | failed`. Only `unavailable` —
`null` from `trySendSponsored`, or missing config — proceeds to the direct tx, as before. A
`SubmittedUnconfirmedError` is `pending` (or `delegated` if the chain already shows the delegation),
and a `reverted on-chain` op or any other throw is `failed` (or `delegated` likewise); both stop
setup and return false. `setupDelegation` logs the outcome and lets the caller retry later. The
caller only ever sees the boolean, so the public API is unchanged.

## One attempt per wallet and chain

A module-level map keyed by wallet address and chain serialises the delegation operations: the filler
and the limit-order service build separate `DelegationService` instances for the same wallet, so a
second `setupDelegation` joins the attempt already in flight rather than sending a competing op on the
same EntryPoint nonce. `revokeDelegation` shares the lock — a revoke during a setup runs, but only
after it — so the two can never overlap. The entry clears when the attempt settles.

## Watch-only

A watch-only solver — `simplex run --watch-only` or a TOML `simplex.watchOnly = true` — refuses
`ChainController.enableFills` and `setWatchOnly(chainId, false)`, and refuses to add a chain with an
explicit `watchOnly: false`, each error naming both causes. For a TOML global `true`, `enableFills`
used to delegate and clear the in-memory flag until the next restart; now it refuses, because the run
is declared watch-only. `syncWatchOnlyToConfig` no longer writes the in-memory snapshot under the
override — that snapshot marks every chain and made one `--watch-only` run permanent. It applies only
the chain the operator added, removed or toggled, and only the value the operator gave it: its key is
deleted when it stops being watch-only, kept as an explicit `watchOnly: true` otherwise, and an add
with no flag writes nothing (the override's derived watch-only is not the operator's request), so a
later override-free run sees what it would have.

## Limit orders

`LimitOrderService.create` runs the 7702-delegation check for same-asset orders too. They are never
posted, but they still price bids, and a bid from an undelegated EOA cannot execute.
