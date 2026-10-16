# 2026-10-16 — A BRIDGE top-up submitted but unconfirmed is resolved from on-chain evidence

A placement can be broadcast and then fail before a receipt arrives: a sponsored UserOp the bundler
accepted but never receipted, or a native tx whose receipt wait fails. Both senders throw
`SubmittedUnconfirmedError` (`src/services/submission.ts`), carrying the chain and the submission:
`{ kind: "userOp", userOpHash, nonce, nonceKey, expires, submittedAt }` or
`{ kind: "tx", txHash, submittedAt }`. The message text is unchanged, because callers that catch a
generic error (the UI's send endpoint, delegation setup, vault funding) rely on it.

A batch never broadcast at all (a rejected bundler submission, a refused undelegated batch, a
reverted `Send tx`) stays a plain error. In `TokenSender.submitCalls`, any failure to read the
receipt after the tx was sent — not only a timeout — is wrapped as a `tx`-kind error, since the
outcome is then unknown.

## The record keeps the evidence

`BridgeTopUp.place` records `placeSession` — the order's session-key address, derived from the
SDK's `sessionPrivateKey`, which is what names this top-up's `OrderPlaced` log — and
`placeFromBlock` (decimal string), the paying chain's height before it submits, which starts the
recovery scan. Only after an unconfirmed failure does it record what was sent:

- `placeTxHash`, or `placeUserOpHash` with `placeUserOpNonce`/`placeUserOpNonceKey` (decimal
  strings), plus `placeSubmittedAt` (ms);
- `placeUserOpExpires`: whether the paymaster attached a validity deadline. Permit2-mode ops do; a
  PERMIT bootstrap op's EIP-2612 permit never expires, so it can land at any time.

Recovery adds `placeScannedTo` (decimal string): the height a scan has read up to, trailing the tip
by a 64-block reorg margin and never below `placeFromBlock`. The next scan starts at
`max(placeFromBlock, placeScannedTo)`. The cursor never decides anything on its own — the scan
records the height it actually reached (`scannedTo`), and every later read of the chain is pinned
to it, so evidence is only ever drawn from a consistent height.

A `placing` record with any of that stays `placing` — pending, so `start()` refuses another top-up —
with the error `Placement is unconfirmed; still checking: …` and a recovery retry every 60 s. A
`placing` record with no evidence at all is `failed` ("Nothing was escrowed"), as before. A legacy
`failed` record with a placement hash is recovered the same way; it must not fall to `unfilled`,
where a reclaim would cancel and mark `reclaimed` an order that was never placed.

## Recovery takes the strongest evidence available

`recoverPlacement` settles a record as `failed` only once the placement can no longer land:

1. **A known tx**: its receipt is read. `recordPlacement` takes the `OrderPlaced` log whose
   `session` is `placeSession` — a UserOp receipt is a bundle that can carry other users'
   placements — and stores the canonical order as `awaiting_bids`. A mined receipt that carries no
   such log, or reverted, is settled: `Placement tx … was mined but placed no order. Nothing was
   escrowed.` A record from before the session was stored falls back to matching the log's `user`
   against this filler's account, so a bundled receipt still resolves to ours. A receipt that
   cannot be read falls through to the scan.
2. **A submitted UserOp**: `TokenSender.userOpReceipt` (backed by
   `UserOpSender.lookupUserOpReceipt`, one `eth_getUserOperationReceipt`) is the fast path. A
   successful receipt becomes `placeTxHash` and is applied as (1); a reverted one settles as
   `failed`; an error or an unknown op falls through.
3. **The scan**: the paying chain's `OrderPlaced` logs are read from
   `max(placeFromBlock, placeScannedTo)` to latest, in chunks wider than BSC's fill search (halved
   on refusal, down to a floor), and the log whose `session` is ours is taken. A match stores its
   transaction hash and is applied as (1). A read failure, or a node behind the range, throws, so
   the caller retries; neither counts as "not found". A record from before `placeFromBlock` was
   stored skips this step.
4. **Nothing on chain** settles only on proof the batch can no longer land:
   - **a native tx** (no UserOp hash) the node has forgotten (`getTransaction` throws
     `TransactionNotFoundError`) more than 30 minutes after submission is dropped. A tx the node
     returns with a block number has been mined, so its receipt will be readable and it is never
     settled here; one the node returns as pending is settled only when the wallet's nonce, read at
     the height the scan reached (or at latest for a record with no scan), has moved past it.
   - **a UserOp**: its EntryPoint nonce, read at the height the scan reached, has moved past the
     stored one — the nonce was consumed, so had it been ours the scan would have found the
     placement — or it is Permit2-mode and its window (the 3600 s permit deadline plus a ten-minute
     margin) has passed within a range the scan covered. A tx hash that came from the bundler's
     receipt is not settled as dropped: a bundle reorged out can be re-bundled under a different
     hash, which the scan would find, so a `TransactionNotFoundError` on it falls through to these
     rules.

   Anything else — a bootstrap op before its nonce moves, a mined or still-known tx the wallet has
   not passed, a read that failed — throws, keeping the record pending and retrying. A bootstrap op
   has no expiry, so it stays `placing` until the account's next UserOp consumes its nonce or its
   `OrderPlaced` log appears; that is correct, because it could still land.

## A live order is settled only by its deadline

The executor reports transient faults — an unreadable destination chain, an unexpected error — as
`FAILED`, and such a failure while the order is live must not become `unfilled`, which leads to a
reclaim. `BridgeTopUp.deadlinePassed` reads the order's deadline (a BSC block height) with
`bsc().getBlockNumber()`; a failed read counts as not passed. In `drive`, an `awaiting_bids`/`filling`
failure before the deadline keeps the stage with `Tracking is retrying: …` and a `resumeOrder()`
retry every 60 s; after the deadline it is `unfilled`. `applyExecutionUpdate` maps `EXPIRED` to
`unfilled`, throws `value.error` for a pre-deadline `FAILED` (so `drive` retries), and maps a
post-deadline `FAILED` to `unfilled`.

`reconcile` also checks an `unfilled` record for a fill before its refund: the indexer's `FILLED`
status or `gateway.isOrderFilled(order)` moves it to `delivering` (with `fillTxHash`, `filler` and
`stageTimes`, as the `awaiting_bids` branch does) and resumes delivery. A late fill must not be
refunded.

## A retry timer belongs to its record

Every timer `drive` arms carries the record's id and is a no-op once `this.record` is a different
top-up, so a leftover retry from a finished record cannot drive — or settle — the next one. Only an
`awaiting_bids`/`filling` failure moves a record to `unfilled`; any other stage just records the
error.

## The delivery request is this order's, not the first one

`deliveryRequest` reads the fill receipt through `bsc()` first and keeps only `Sent` logs whose
`address` is this chain's BRIDGE token and whose `to` is the filler's substrate public key. When
more than one matches, the amount the order's output call sends picks the right one; if that is
still ambiguous it throws, and the delivery loop retries. A readable receipt with no matching send
throws "The fill … sent no BRIDGE to this filler's Hyperbridge account". The indexer is the fallback
only when the receipt cannot be read, and only when exactly one node comes back.

## A native batch on an undelegated wallet is refused

The native fallback sends a multi-call batch to the account itself (`executeBatch`), which on an EOA
with no EIP-7702 code succeeds and does nothing. `TokenSender` takes an optional `isDelegated(chain)`
and, before the multi-call native send, throws `The wallet is not 7702-delegated on <chain> (or its
delegation could not be read); a batch sent to it would do nothing` when it returns false. The
single-call path still goes straight to the target. `boot.ts` builds one read-only
`DelegationService` and shares it between `TokenSender` and `BridgeTopUp`'s `delegation` dependency.

## The reclaim cancel

A `cancelOrder` submission that was broadcast but unconfirmed stores its `reclaimTxHash` before the
failure is rethrown, so the retry hands the sent cancel back to the SDK instead of sending, and
paying for, a second one. A UserOp-kind submission carries no tx hash yet, so it leaves
`reclaimTxHash` unset and the next attempt submits again.
