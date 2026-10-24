# Tesseract retries transient EVM RPC failures and keeps GET requests whose proofs fail

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

## GET requests are parked until the next update

In `tesseract/messaging/messaging/src/get_requests.rs`, a GET request whose source proof fails is
stored in the `UnprofitableMessages` table under hyperbridge's state machine together with its
destination proof. The row survives a restart while it waits for a source proof, and a later update
fetches only the source proof at its newer height while reusing the stored destination proof. Once
both proofs are built its row is removed; a failure at the submission stage (challenge period, dry
run, submit) drops it, as before. A GET request whose destination proof fails is dropped, since the
RPC retry layer already covers short misses, and a missing destination client drops it. Timed-out
requests are dropped. When the hyperbridge timestamp query fails, the new requests are stored with
their destination proof. `handle_update` forwards a batch for every handled range, including an
empty one, so the processor gets that next update. Submissions run in a background task, so the
processor keeps reading updates while a challenge period or submission is in progress.
