# 2026-09-03 — New web markets default to no order cap

Chosen: initialize the operator and setup-wizard maximum-order fields as empty strings. Both web paths
already omit blank `maxOrderSize` values when assembling or persisting a market, and the config model
defines an omitted cap as uncapped. Existing markets retain their configured limits.

Alternative rejected: keeping a numeric default would silently impose an exposure limit on every new
market, despite the field being optional.
