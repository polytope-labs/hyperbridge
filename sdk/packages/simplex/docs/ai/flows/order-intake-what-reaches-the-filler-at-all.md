# Order intake: what reaches the filler at all

`ChainScanner` polls `eth_getLogs`, rebuilds each `OrderPlaced` log into an `Order` via
`reconstructOrdersFromLogs`, and hands it to every subscriber. `EventMonitor` is the per-filler
subscriber and applies three filters in order (`src/core/event-monitor.ts`):

1. **Chain.** A shared scanner may carry chains this filler is not configured for, so events are
   matched on `chainId` against the monitor's own set.
2. **Leg count.** The order must have exactly one entry in `inputs` and one in `output.assets`.
   Anything else emits `orderSkipped` with reason `Multi-leg order` and stops here.
3. **De-duplication.** The order id must not be in the seen set (last 5,000 ids). A scanner
   resuming from a cursor re-delivers, and the fill path has no idempotency of its own.

Only what survives all three is emitted as `newOrder`, which `IntentFiller` picks up in its
constructor and passes to `handleNewOrder`. Fills take a separate path: `onFill` narrows on the
filler address — `filler` is `indexed: false` in the ABI, so it can never be a topic filter — and
emits `orderFilledOnChain`.

Phantom orders do not come through here. They are polled from Hyperbridge inside `IntentFiller`,
so the leg-count filter does not apply to them and `quotePhantomLeg` still splits a bundled
phantom order into positional single-pair legs.
