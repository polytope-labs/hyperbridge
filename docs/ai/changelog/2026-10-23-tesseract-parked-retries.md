# Tesseract classifies failed delivery simulations and bounds parked retries

A parked message was re-simulated every `retry_frequency` forever, and the trace only logged
`Error in call frame <addr>: execution reverted` without the revert data. A message the
destination host would never accept looked the same as a module callback that might succeed
later.

The EVM provider now classifies a failed simulation from its call trace into a
`SimulationFailure`:

- A reverting frame on the host/handler spine is `HostRejected`. The deepest such frame is
  used, and `BatchCallFailed`'s inner revert is decoded, so the reason is the host's own error
  (`MessageTimedOut`, `DuplicateMessage`, …) rather than the wrapper. `DuplicateMessage`,
  `MessageTimedOut`, `InvalidMessageDestination` and `UnknownMessage` are permanent; an
  unknown host selector is retried with backoff and aged out. A host call made from inside the
  module is not on the spine, so it stays `ModuleReverted`.
- Any other reverting frame is `ModuleReverted`, the module's callback, and stays retriable.
- Gas exhaustion in any frame is `OutOfGas`, which says nothing about the message.
- A `dispatchIncoming` frame with no error and no child calls is the host's early return when
  the destination module has no code, and is a permanent host rejection. Any other clean trace
  with no handled event is `Unknown` and stays retriable.

The decoded revert reason is logged once per failed simulation instead of per frame. A
permanent host rejection is dropped in `return_successful_queries`; a module revert is parked
again.

Parked rows now carry `attempts`, `created_at` and `next_attempt_at` in a new
`UnprofitableMessages` migration. The first retry is at `retry_frequency`, doubling per failed
delivery up to an hour. Only failed deliveries count: a module revert, or a mined batch that
emitted no handled event. A reschedule for a missing proof, a submit error or the fee gate
keeps the attempt count and age clock unchanged, so a message without a failed delivery
attempt is never aged out. A message is dropped after 10 failed deliveries or a day of them,
which bounds a stale cancellation whose `onAccept` always reverts. Rows written before the
migration are treated as parked now.
