# Intent gateway order fields (#1112)

Adds these nullable fields to the indexer:

| Entity | Fields | Meaning |
|---|---|---|
| `IOrderV3` | `userOpHash` | ERC-4337 operation that placed the order |
| `IOrderV3Fill`, `IOrderV3PartialFill` | `userOpHash` | ERC-4337 operation that executed the fill |
| `IOrderV3FillOutputAsset`, `IOrderV3PartialFillOutputAsset` | `amountReceived` | ERC-20 amount delivered to the beneficiary, including surplus |
| `IOrderV3` | `feeToken`, `feeTokenDecimals` | Source host's fee token and decimals at the placement block |

Operation hashes are matched from EntryPoint receipt logs using the placing account or filler and the operation boundaries. Direct transactions and operations that cannot be attributed leave the hash null. Both current and legacy placement events are supported. A placement replay with an unavailable receipt preserves a previously stored hash.

Delivered amounts are matched from transfers within each fill's log range. Native outputs and ambiguous deliveries remain null. When the order has not been indexed yet, its beneficiary is recovered from calldata and verified against the order commitment.

The schema changes are additive. Existing records retain their data and default to null for the new fields; no backfill is performed.
