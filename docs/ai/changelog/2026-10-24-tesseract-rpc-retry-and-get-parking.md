# Tesseract retries transient EVM RPC failures and keeps GET requests whose proofs fail

## EVM RPC retries

`RetryLayer` (`tesseract/messaging/evm/src/transport.rs`) retries transient JSON-RPC failures with
backoffs of 250 ms, 500 ms, 1 s and 2 s, within a 60 s budget measured from the start of the call.
The last completed result is returned when the budget runs out. Retried errors are rate-limit
responses, `historical state`, `missing trie node`, `header not found`, HTTP 429/5xx, and transport
timeouts or connection failures. A `null` result for `eth_getProof` is also retried, because it is
never a valid proof. Deterministic errors such as `execution reverted` and invalid params are
returned unchanged. Transaction sends are never retried.

A single-URL provider now gets the same retry as a multi-URL one. `evm::create_provider` and
`EvmClient::new` share `build_provider`, which places `RetryLayer` outside the fallback transport, so
each retry is a fresh parallel round over every endpoint. For a single `eth_sendRawTransaction`, an
`already known` error becomes a successful response carrying `keccak256(raw transaction)`.

## GET requests are parked until the next update

In `tesseract/messaging/messaging/src/get_requests.rs`, `process_get_request_events` keeps requests
whose source or destination proof failed and retries them on the next `StateMachineUpdated`. Parked
requests are proved at the newest known source height, while the destination proof stays at the
request's own height. A request is dropped once it has timed out. `handle_update` now forwards a
batch for every handled range, including an empty one, so the processor gets that next update.

Parked GET requests live in memory only, so a relayer restart forgets them. A GET whose destination
height no configured RPC can prove is retried on each update until it times out.
