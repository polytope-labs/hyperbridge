# 2026-10-16 — A lost bundler send response is resolved before the caller may resend natively

`eth_sendUserOperation` can fail after the bundler accepted the op: a gateway 504, an HTML error
page, a dropped socket, or the request timeout. `UserOpSender` used to treat any such failure as
"never submitted" and return `null`, which every caller reads as permission to send the same calls
again as a native transaction — paying twice for one dashboard send, or double-executing a top-up
placement or a vault batch.

## Every bundler request is classified

`UserOpSender` funnels its bundler calls (send, estimate, both receipt polls, the by-hash lookup)
through one helper. Each request is bounded — 30 s for the send and estimate, which run the
validation simulation, and 10 s for the polls — and a call that returns no result throws a
`BundlerCallError` whose `outcome` is one of:

- `rejected` — an HTTP 200 whose JSON-RPC `error.code` is a deterministic rejection (`-32700`,
  `-32600`…`-32602`, or `-32599`…`-32500`, the ERC-4337/ERC-7769 range which includes `-32521`).
  A non-200 status with any body, `-32603`, a missing or non-numeric code, or any other code is
  *not* a definite refusal.
- `unreachable` — `fetch` failed in the connect phase, so no request bytes were delivered:
  `ECONNREFUSED`, `ENOTFOUND`, `EAI_AGAIN`, `EHOSTUNREACH`, `ENETUNREACH`,
  `UND_ERR_CONNECT_TIMEOUT`, or a TLS certificate-verification failure.
- `pending` — see the next section.
- `ambiguous` — everything else.

The gas-price lookup is not one of these calls: it goes through the SDK's
`rundlerUserOperationFees`, whose `fetch` has no timeout, so `UserOpSender` bounds the whole lookup
at 10 s. It runs before anything is signed, so a timeout there is "never submitted".

Only `rejected` and `unreachable` count as "not submitted". The rule is deliberate: a proxy's JSON
502/504 forwarded the request, and rundler's -32603 can arrive after the pool admitted the op, so
those stay ambiguous; misclassifying an ambiguous failure as decided costs at most one lookup,
while the reverse costs a double send.

## An op already pending on the nonce is not a rejection

Rundler answers a send with `-32602` (the code of a malformed request) when an op from the same
sender is already pending on that EntryPoint nonce: `ReplacementUnderpriced` carries
`currentMaxPriorityFee`/`currentMaxFee` in its error data, and `OperationAlreadyKnown` has the
message "already known". `UserOpSender` reads either as `pending` and throws
`PendingOpOnNonceError` instead of returning `null`. A plain `-32602` (invalid params) is still a
rejection.

Without this, a send answered `202` and then repeated would be refused as a replacement, read as
`null`, and sent natively — paid twice. `PendingOpOnNonceError` means "not now": `TokenSender`
lets it through, `POST /api/send` reports it, the vault sweep and redeem leave it for the next
cycle, and delegation setup returns `false` without a native delegation tx. A top-up placement
that meets it fails with nothing escrowed.

## The op is looked up before falling back

`trySendSponsored` computes the op's hash locally before sending — the v0.8 EIP-712 digest of the
packed op (`hashTypedData(packedUserOpTypedData(...))`) is its `userOpHash` — and on an ambiguous
send failure calls `eth_getUserOperationByHash` roughly 2 s, 7 s and 17 s after the send (the
default inter-attempt delays are 2 s, 5 s and 10 s):

- **found** — proceed as a successful send from the local hash: wait for the receipt, and throw
  `SubmittedUnconfirmedError` on none.
- **absent** — the bundler answered `null` on every attempt, so the op never entered its mempool.
  Return `null`; the caller may fall back. A residual race remains: a bundler still validating the
  op after the timeout could add it after the last lookup, which is what the widening delays narrow.
- **unknown** — a lookup failed, so absence is not proven. Throw `SubmittedUnconfirmedError` with
  the local hash and `maybeSubmitted = true`; the caller must not fall back.

A successful send whose returned hash differs from the local one is logged, and the bundler's is
used.

The local hash matches what the EntryPoint computes for a v0.9 op as well as v0.8: v0.9 leaves a
trailing paymaster signature out of the hash, and simplex's `paymasterAndData` carries no such
suffix.

## A bundler outage

When every lookup fails, the send is `unknown`. `POST /api/send` answers `202` saying the send *may
have been submitted*. A top-up placement is recorded with the local hash and waits out its window
before it settles. A vault sweep or redeem throws, and the batch stalls, rather than going native,
until the bundler answers again.

## An unconfirmed send is reported as pending, not failed

`POST /api/send` answers a `SubmittedUnconfirmedError` or a `PendingOpOnNonceError` with HTTP 202
`{ pending: true, sponsored, userOpHash?, txHash?, message }` instead of 400. Every other error
stays a 400.

- A native tx has a hash from the moment the node accepts it, so it gets a wallet-history row like a
  confirmed send. A UserOp has none until it lands (`WalletTx.txHash` is required), so it gets no row.
- A sponsored UserOp's message says it may still land and to wait before resending: the wallet
  balance cannot show an op still in the bundler's pool. When the outcome is `unknown` it says the
  send *may have been submitted*. A native tx keeps the explorer wording and link.
- A `PendingOpOnNonceError` answers "an earlier send from this wallet is still pending; wait for it
  before sending again" with no hash, since nothing new was submitted.

The operator UI renders the pending response in a warning tone, clears the amount so the next click
cannot resend, and shows no "Sent" success line. A UserOp hash is shown as plain text, a native tx
hash as an explorer link.
