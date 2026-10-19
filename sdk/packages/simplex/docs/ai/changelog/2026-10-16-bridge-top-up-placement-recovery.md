# 2026-10-16 — A BRIDGE top-up submitted but unconfirmed is resolved from on-chain evidence

A placement or reclaim cancel can be broadcast and then fail before a receipt arrives: a sponsored
UserOp the bundler accepted but never receipted, or a native tx whose receipt wait fails. Both
senders throw `SubmittedUnconfirmedError` (`src/services/submission.ts`) with the chain and the
submission, `{ kind: "userOp", userOpHash, nonce, nonceKey, expires, submittedAt }` or
`{ kind: "tx", txHash, nonce, submittedAt }`. A batch never broadcast (a rejected bundler
submission, a refused undelegated batch, a reverted `Send tx`) stays a plain error. The message
text is unchanged, because callers that catch a generic error rely on it.

`TokenSender.submitCalls` reads the wallet's `pending` nonce, sends the native tx with it
explicitly, and returns the same `submission` on success (the sponsored op's, from
`UserOpSender.trySendSponsored`). `TokenSender.send` still reports only `txHash` and `sponsored`.

## The placement record keeps the evidence

`BridgeTopUp.place` records `placeSession` (the session-key address that names this top-up's
`OrderPlaced` log) and `placeFromBlock` (the paying chain's height before it submits). After the
send, success or not, it records `placeSubmittedAt` and the submission: `placeTxHash` with
`placeTxNonce`, or `placeUserOpHash`, `placeUserOpNonce`, `placeUserOpNonceKey` and
`placeUserOpExpires` (Permit2-mode ops expire; a PERMIT bootstrap op never does). Keeping the op
on the success path means a bundle hash that later vanishes is settled by the UserOp rules, not
as a dropped native tx. A legacy record without `placeSubmittedAt` is dated by `stageTimes.placing`,
then `createdAt`, never `updatedAt`, which every save rewrites.

A `placing` record with any evidence stays `placing`, so `start()` refuses another top-up, with
`Placement is unconfirmed; still checking: …` and a retry every 60 s. One with none is `failed`.
A legacy `failed` record with a hash is recovered the same way and never falls to `unfilled`.

## Recovery takes the strongest evidence, read below the tip

1. **A known tx**: its receipt. `recordPlacement` takes the `OrderPlaced` log emitted by this
   chain's IntentGateway whose `session` is `placeSession` (a bundle can carry other ops' logs,
   including forged ones from other contracts), and stores the canonical order as `awaiting_bids`.
   A mined receipt without it, or a reverted one, settles `failed`.
2. **A submitted UserOp**: one `eth_getUserOperationReceipt` (`TokenSender.userOpReceipt`).
3. **The scan** of the paying chain's `OrderPlaced` logs, from `max(placeScannedTo,
   placeFromBlock − 64)` to latest, in halving chunks. `placeScannedTo` is saved after every chunk,
   trailing the tip by the 64-block reorg margin. A failed read throws; it never counts as "not found".
4. **Nothing on chain** settles `failed` only on proof the batch can no longer land. All reads are
   at `scannedTo − 64`, except the permit clock:
   - a native tx: the wallet's nonce at that height is past `placeTxNonce` (or the nonce of the tx
     the node still lists). A tx a node no longer lists proves nothing, since one backend or
     mempool can lack a valid tx, so a record without a nonce keeps waiting;
   - a UserOp: its EntryPoint nonce at that height is past the stored one, or it is Permit2-mode
     and the block timestamp at `scannedTo` is past `placeSubmittedAt` plus the permit window
     (deadline plus a ten-minute margin). The host clock is not used.

   A bundler-sourced tx hash that the node does not list is not settled as dropped: the bundle may
   be re-bundled under another hash, which the scan would find. A bootstrap op has no expiry and
   stays pending until its nonce moves or its log appears.

## A flow writes only to its own record

Every background flow (placement, recovery, tracking, delivery, reclaim, sync) runs bound to the id
of the record it started for. `update` does nothing once `this.record` is a different top-up, and
`trackOrder` and `recordPlacement` stop, so a recovery that finishes after `start()` replaced a
`failed` record cannot land its stage, hashes or errors in the new one. Retry timers carry the id too.

## A live order is settled only by its deadline

An executor `FAILED` before the order's BSC deadline (`deadlinePassed`) keeps the stage with
`Tracking is retrying: …` and a 60 s retry; after the deadline it is `unfilled`. `reconcile`
checks an `unfilled` record for a fill (indexer `FILLED`, or `gateway.isOrderFilled`) before its
refund, and moves it to `delivering` if one landed.

## The delivery request is this order's

`deliveryRequest` reads the fill receipt and keeps `Sent` logs from the BRIDGE token addressed to
this filler's substrate key; the order's output amount picks among several. The indexer is the
fallback only when the receipt cannot be read and returns exactly one request.

## A native batch on an undelegated wallet is refused

`TokenSender` takes an optional `isDelegated(chain)` and refuses a multi-call native batch when it
returns false, since the batch goes to the account itself and would do nothing. `boot.ts` shares one
read-only `DelegationService` between `TokenSender` and `BridgeTopUp`.

## The reclaim cancel

A cancel sent without a receipt is stored as `reclaimSubmission` (the tx hash and nonce, or the
UserOp's hash, nonce and key), not as `reclaimTxHash`. Before the next attempt hands it to the SDK
it is checked: a successful receipt hands the hash back (a UserOp's receipt comes from the
bundler); a reverted receipt, or no receipt with the wallet's or EntryPoint's nonce past it (read
below the tip), clears it and sends a new cancel; otherwise the attempt waits and retries. A
UserOp-kind cancel that timed out is therefore never resent while it can still land.
