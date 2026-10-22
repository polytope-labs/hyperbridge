# Tesseract traces EVM deliveries with the chain's gas limit and parks what a mined batch did not deliver

`EvmHost.dispatchIncoming` swallows a module callback that reverts, out of gas included, and the
transaction still succeeds. So `eth_estimateGas`, which looks for the lowest limit at which the
transaction does not revert, can return a limit at which some requests are not delivered. Every
nested call also keeps back 1/64 of its remaining gas (EIP-150), so a module several calls deep
needs more headroom than a fixed percentage over the estimate gives.

`submit_batch_messages` and `submit_messages` in `tesseract/messaging/evm/src/tx.rs` now size the
gas limit of every transaction carrying requests or responses with `delivery_gas_limit`:

- The transaction is traced with the chain's gas limit (`get_chain_gas_limit`), which gives the
  commitments it delivers, the gas it uses and how deep its calls nest.
- The limit is `gas used × (64/63)^depth`. It is never below the estimate plus 5% and never above
  the chain's gas limit.
- A second trace at that limit must deliver the same commitments, or the chain's gas limit is used.
- When the transaction cannot be traced, or reverts even with the chain's gas limit, the estimate
  plus 5% is kept.

For Base batch `0xf1a357de…`, the estimate plus 5% was 911,982. The batch used 861,222 gas with
calls nested nine deep and needed 915,009 to deliver its third request. The new rule gives
992,367.

After a transaction is mined, each request or response whose commitment is missing from the
receipt's `PostRequestHandled` and `GetRequestHandled` logs is returned as unsuccessful, so the
outbound and retry paths park it. A request message comes back holding only its undelivered
requests, which the retry path proves again. A response message comes back whole when any of its
responses is missing. Before, a mined batch reported nothing as unsuccessful.

The per-message traces in `estimate_gas_for_tx_requests`, which decide whether a message is
delivered at all, also run with the chain's gas limit. At the estimate plus 5% they failed a
message that only needed more gas, so it was skipped and parked on every retry and never reached
submission. BSC→Base request nonce 70 used 287,164 gas with calls nested eight deep, and ran out of
gas in its trace at 304,586.

On Erigon, traces carry a gas price, so the node can refuse a trace at the chain's gas limit for
the sender's balance. The profitability trace then falls back to the estimate plus 5%, and
`delivery_gas_limit` keeps the estimate plus 5%.
