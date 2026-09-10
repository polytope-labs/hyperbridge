# 2026-09-03 — Show configured prices without inventing venue prices

Chosen: derive buy and sell labels from the first valid bid and ask curve points already present in
the operator strategy DTO, and include the `token1/token0` unit. Hide a side when it has no valid
configured curve, and hide all values for venue-priced or reference-only markets because the list API
does not provide a live venue quote.

Alternative rejected: displaying a zero, dash, or stale-looking value for missing/venue prices would
imply a quote the operator list cannot substantiate.
