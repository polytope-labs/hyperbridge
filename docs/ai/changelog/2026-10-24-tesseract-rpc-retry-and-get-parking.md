# Tesseract retries transient EVM RPC failures and keeps GET requests until hyperbridge has their response

## EVM RPC retries

`RetryLayer` (`tesseract/messaging/evm/src/transport.rs`) retries transient JSON-RPC failures with
backoffs of 250 ms, 500 ms, 1 s and 2 s, within a 60 s budget measured from the start of the call.
The last completed result is returned when the budget runs out. Retried errors are rate-limit
responses, `historical state`, `missing trie node`, `header not found`, HTTP 429/5xx (gateway
timeouts arrive as 5xx), and connection errors. A call that hit the HTTP client's own timeout has
already used up the 60 s budget, so it is not retried. Reverts are never retried, even when their
reason contains a retry-like phrase. Transaction sends are never retried.

A `null` result for `eth_getProof` is never valid: it is retried, and with several URLs it counts as
that endpoint failing so a slower valid proof can win.

A single-URL provider now gets the same retry as a multi-URL one. `evm::create_provider` and
`EvmClient::new` share `build_provider`, which places `RetryLayer` outside the fallback transport, so
each retry is a fresh parallel round over every endpoint. For a single `eth_sendRawTransaction`, an
`already known` error becomes a successful response carrying `keccak256(raw transaction)`.

## GET requests are kept until hyperbridge has their response

In `tesseract/messaging/messaging/src/get_requests.rs`, each GET request is proved and submitted on
its own, so its storage proof covers only its own keys; EVM verification rejects a proof that covers
other contracts. A GET's destination proof is fetched first. If that fails the GET is dropped, since
the RPC retry layer already covers short misses. Otherwise the GET is stored in the
`UnprofitableMessages` table under hyperbridge's state machine with that proof, so it survives a
restart; if storing fails, the GET is only kept for that update. Each update proves stored GETs again
at the newer source height and skips those whose submission is still running. A stored GET is
removed only once hyperbridge has a response receipt for it, it has timed out, or its destination
client is gone. A failed submission is retried at the first update after the attempt finishes.
`handle_update` forwards a batch for every handled range, including an empty one, so the processor
gets that next update, and submissions run in a background task so the processor keeps reading
updates while a challenge period or submission is in progress.
