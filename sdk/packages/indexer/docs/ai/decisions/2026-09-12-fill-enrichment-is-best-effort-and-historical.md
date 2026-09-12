# 2026-09-12 — Fill enrichment is best-effort and historical

Chosen: persist receipt-derived fields only when they can be attributed safely. `userOpHash` comes from the first
canonical EntryPoint `UserOperationEvent` after the fill; a different sender ends that search so an operation later in
the bundle cannot be assigned to the fill. `amountReceived` comes from ERC-20 transfers in the fill's log range, and is
left null when the order beneficiary or a one-to-one transfer attribution cannot be established. Native-token outputs
also remain null.

The host fee token and decimals are read with the `OrderPlaced` block hash, rather than a process-lifetime cache. Host
governance can replace the fee token, so a latest-state lookup would relabel historical fees. The short block-hash cache
only deduplicates reads inside the same indexed block.

Alternative rejected — infer a missing delivery from the event amount or assign the first matching transfer. Both can
silently report the promised amount as an overfill's delivered amount, or attach an unrelated transfer. A nullable
field accurately signals that the receipt did not provide an unambiguous answer.
