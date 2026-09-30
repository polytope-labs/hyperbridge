# 2026-09-05 — Order summaries are captured at detection and denormalised onto every event

Chosen: the recorder builds an `OrderSummary` when an order is detected and attaches the same
object to that order's later events, stored as JSON per row. The maintainer asked for the activity
feed to match the HyperFX history page (amounts, tokens, chains, user, referrer, links).

Alternatives rejected: looking the order up from the indexer at render time would make the local
dashboard depend on a remote GraphQL service and lag it; storing the summary only on the
detection row would leave a filled row without its order whenever paging split the two; reading
token decimals in the UI would need per-chain RPC access from the browser. The referrer is shown as
the raw 20-byte tag rather than a name: the HyperFX app's referrer names are not in this
repository, so there is nothing to map from.
