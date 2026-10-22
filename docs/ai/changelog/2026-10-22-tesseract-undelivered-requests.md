# Tesseract no longer drops an inbound range whose proofs failed, and an RPC endpoint's error no longer beats another's answer

## Inbound ranges advance only once their events are read

In `handle_update` (`tesseract/messaging/messaging/src/lib.rs`), `previous_height` advances only
after the range's events were read. An event query error leaves the range to be read again on the
next `StateMachineUpdated`; the error log shows `retry_from` and `to`. A proof failure instead parks
the range's post requests: they are stored in the `UnprofitableMessages` table under the sink's
state machine and the update moves on, so the next update joins them to its own proof query and they
are proved at the newer height. That keeps the `eth_getLogs` range bounded, which matters on chains
whose finality makes the range grow. The rows are deleted once the range they belong to is handled.
Those rows are shared by every pipeline that delivers to the same sink, so a request is only joined
to the proof query of the source chain it originates on.
A `GetResponse` whose proof fails is not parked (it has no row to re-prove from), so that range is
read again instead. GET requests of a range are forwarded once, when the range is handled.

Event queries are chunked at `query_batch_size` (5,000 blocks by default) and a chunk that fails now
fails the whole query instead of yielding an empty result, so a partial read can never be mistaken
for "no events".

## Multi-URL EVM providers treat a JSON-RPC error as a failed endpoint

`EvmClient::new` and `create_provider` build their fallback transport with `fallback_service`
(`tesseract/messaging/evm/src/transport.rs`). `ErrorResponseAsFailure` turns a JSON-RPC error
response from one endpoint into that endpoint failing, so a pruned node's `historical state … is not
available` no longer beats an archive node's `eth_getProof`. When every endpoint errors the last
error is returned. A request one endpoint answers with a JSON-RPC error now waits for the others, so
a reverting `eth_call` or `eth_estimateGas` returns at the slowest endpoint's latency; if another
endpoint hangs, it waits up to the HTTP client's timeout and may return that endpoint's transport
error instead.
