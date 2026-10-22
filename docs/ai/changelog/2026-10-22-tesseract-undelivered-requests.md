# Tesseract re-queries an inbound range whose proofs failed, and an RPC endpoint's error no longer beats another's answer

## Inbound ranges advance only once handled

In `handle_update` (`tesseract/messaging/messaging/src/lib.rs`), `previous_height` advances only
after the range's events are read and its messages are built. On an event or proof query error the
next `StateMachineUpdated` queries the same range again and proves it at the newer height; the error
log carries `retry_from`. GET requests of a range are forwarded once, when the range is handled. A
range that keeps failing is queried again on every update (the error log shows `retry_from` and
`to`), and the providers' `query_ismp_events` still log and skip an event chunk they cannot read.

## Multi-URL EVM providers treat a JSON-RPC error as a failed endpoint

`EvmClient::new` and `create_provider` build their fallback transport with `fallback_service`
(`tesseract/messaging/evm/src/transport.rs`). `ErrorResponseAsFailure` turns a JSON-RPC error
response from one endpoint into that endpoint failing, so a pruned node's `historical state … is not
available` no longer beats an archive node's `eth_getProof`. When every endpoint errors the last
error is returned. A request one endpoint answers with a JSON-RPC error now waits for the others, so
a reverting `eth_call` or `eth_estimateGas` returns at the slowest endpoint's latency; if another
endpoint hangs, it waits up to the HTTP client's timeout and may return that endpoint's transport
error instead.
