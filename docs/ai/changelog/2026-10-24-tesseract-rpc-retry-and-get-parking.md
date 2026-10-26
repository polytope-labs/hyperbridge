# Tesseract retries transient EVM RPC failures and keeps GET requests until hyperbridge has their response

## EVM RPC retries

`RetryLayer` (`tesseract/messaging/evm/src/transport.rs`) retries transient JSON-RPC failures with
backoffs of 250 ms, 500 ms, 1 s and 2 s, within a 60 s budget measured from the start of the call.
The last completed result is returned when the budget runs out. Retried errors are rate-limit
responses, `historical state`, `missing trie node`, `header not found`, HTTP 429/5xx (gateway
timeouts arrive as 5xx), and connection errors. `EvmClient`'s 180 s HTTP client timeout already
exceeds the 60 s budget, so its first attempt is not retried, while `create_provider` uses
`reqwest::Client::new()` with no timeout at all. Reverts are never retried, even when their reason
contains a retry-like phrase. Transaction sends are never retried.

A `null` result for `eth_getProof` is never valid: it is retried, and with several URLs it counts as
that endpoint failing so a slower valid proof can win.

A single-URL provider now gets the same retry as a multi-URL one. `evm::create_provider` and
`EvmClient::new` share `build_provider`, which places `RetryLayer` outside the fallback transport, so
each retry is a fresh parallel round over every endpoint. For a single `eth_sendRawTransaction`, an
`already known` error becomes a successful response carrying `keccak256(raw transaction)`.

## GET requests are kept until hyperbridge has their response

In `tesseract/messaging/messaging/src/get_requests.rs`, each GET request is proved and submitted on
its own, so its storage proof covers only its own keys; EVM verification rejects a proof that covers
other contracts. A GET is stored in the `UnprofitableMessages` table under hyperbridge's state
machine once seen, unless it has already timed out or has no destination client. It is stored with
its destination proof when the fetch works and without one otherwise. Each update proves due rows
again at the newer source height and skips rows whose submission is still running. Each submission
attempt and each failed destination-proof fetch counts toward the same caps as parked POSTs: 10
attempts or a day, with a backoff starting at `retry_frequency` and doubling up to an hour; a
source-proof failure does not count. A row is removed once hyperbridge has a response
receipt for it (checked when it is next due), it has timed out, it has used up its retries, or its
destination client is gone. `handle_update` forwards a batch for every handled range, including an
empty one, so the processor gets that next update, and submissions run in a background task so the
processor keeps reading updates while a challenge period or submission is in progress.
