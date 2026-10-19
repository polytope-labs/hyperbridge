# 2026-10-16 — A delegation is confirmed on chain, and only a never-submitted op falls back

`DelegationService` used to read success from a transaction receipt alone and to fall back to a
native tx whenever the bundler path did not clearly succeed. EIP-7702 skips an authorization that
does not match the authority's nonce without reverting, so a receipt could read `success` while the
wallet stayed undelegated — and simplex would then bid from an EOA with no code, paying Hyperbridge
fees and locking deposits for fills that cannot execute. Delegation now confirms on chain and stops
the paths that could double-delegate. The mechanics (nonces, confirmation, the lock, the pending-op
record) are in `docs/ai/flows/signing-from-construction-to-each-signature.md`.

## Nonces

A direct tx is sent at the `pending` nonce with its tuple signed at `nonce + 1`, passed explicitly
by `sendDelegationTransaction` in both the plain and the batched-approve branch; `revokeDelegation`
does the same. The bundler path signs the tuple at the authority's current nonce and refuses while
a tx is in flight, which `trySendSponsored` reports as never submitted. The native fallback then
queues behind that tx at the next (`pending`) nonce. Every bundled signer honours the explicit nonce
on a set-code tx.

## Confirmation

A success receipt no longer reports a delegation: the code at the receipt's block must be the
configured SolverAccount (empty, for a revoke). A failed read at that block, or a bundler-path
receipt wait that times out (30 s), falls back to the wallet's delegation at `latest`, so a
delegation that landed is not reported as failed. The final native fallback returns
`trySendDelegation(...) || isDelegated(chain)`.

## No native fallback after a submitted op

`setupDelegationViaBundler` returns `delegated | unavailable | pending | failed`, and only
`unavailable` proceeds to the direct tx.

- A `SubmittedUnconfirmedError` or a `PendingOpOnNonceError` is `pending`, and a reverted op or any
  other throw is `failed`; either is `delegated` if the chain already shows the delegation. Setup
  stops and returns false.
- An unconfirmed UserOp is remembered per wallet and chain. The next attempt resolves it before
  anything is sent: a receipt confirms it on chain, a moved EntryPoint nonce without a delegation
  drops it, and anything unresolved stays `pending`, so a retry whose bundler call is refused as
  already known cannot go native.

`setupDelegation` still answers a boolean. It can also reject, when the read that sizes the
sponsored op fails before anything is submitted, so the caller retries.

## One attempt per wallet and chain

The filler and the limit-order service build separate `DelegationService` instances for one wallet,
so a module-level lock makes a second `setupDelegation` join the attempt in flight. A
`revokeDelegation` queues behind a setup instead of overlapping it.

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
posted, but they still price bids, and a bid from an undelegated EOA cannot execute. On a watch-only
chain the service is given a live predicate (`boot.ts` reads the filler's watch-only flags) and only
checks `isDelegated`: an undelegated solver is refused with the watch-only wording, and
`setupDelegation` is never called, so creating an order cannot send a set-code tx or a sponsored op.
