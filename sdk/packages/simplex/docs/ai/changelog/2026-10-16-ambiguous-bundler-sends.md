# 2026-10-16 — A lost bundler send response is resolved before the caller may resend natively

`eth_sendUserOperation` can fail after the bundler accepted the op: a gateway 504, an HTML error
page, a dropped socket, or the request timeout. `UserOpSender` used to treat any such failure as
"never submitted" and return `null`, which every caller reads as permission to send the same calls
again as a native transaction — paying twice for one dashboard send, or double-executing a top-up
placement or a vault batch.

## Every bundler request is classified

`UserOpSender` funnels its bundler calls (send, estimate, both receipt polls, the by-hash lookup)
through one helper. (`getGasPrice` is not one of them: it goes through the SDK's
`rundlerUserOperationFees`, which fetches the bundler's suggested fees itself.) Each request is
bounded — 30 s for the send and estimate, which run the validation simulation, and 10 s for the
polls — and a call that returns no result throws a `BundlerCallError` whose `outcome` is one of:

- `rejected` — an HTTP 200 whose JSON-RPC `error.code` is a deterministic rejection (`-32700`,
  `-32600`…`-32602`, or `-32599`…`-32500`, the ERC-4337/ERC-7769 range which includes `-32521`).
  A non-200 status with any body, `-32603`, a missing or non-numeric code, or any other code is
  *not* a definite refusal.
- `unreachable` — `fetch` failed in the connect phase, so no request bytes were delivered:
  `ECONNREFUSED`, `ENOTFOUND`, `EAI_AGAIN`, `EHOSTUNREACH`, `ENETUNREACH`,
  `UND_ERR_CONNECT_TIMEOUT`, or a TLS certificate-verification failure.
- `ambiguous` — everything else.

Only `rejected` and `unreachable` count as "not submitted". The rule is deliberate: a proxy's JSON
502/504 forwarded the request, and rundler's -32603 can arrive after the pool admitted the op, so
those stay ambiguous; misclassifying an ambiguous failure as decided costs at most one lookup,
while the reverse costs a double send.

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
  the local hash; the caller must not fall back.

A successful send whose returned hash differs from the local one is logged, and the bundler's is
used.

## An unconfirmed send is reported as pending, not failed

`POST /api/send` answers a `SubmittedUnconfirmedError` with HTTP 202
`{ pending: true, sponsored, userOpHash?, txHash?, message }` instead of 400, and writes no wallet
history row — a `WalletTx.txHash` is required, and an unconfirmed UserOp has none. Every other error
stays a 400. The operator UI renders the pending response in a warning tone, clears the amount so
the next click cannot resend, and shows no "Sent" success line. A sponsored UserOp has no tx hash,
so its message points at the wallet balance or history and its hash is shown as plain text, while a
native tx keeps the explorer wording and link.
