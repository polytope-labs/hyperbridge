# 2026-09-05 — Treasury editor groups vaults per chain and folds non-enabled chains

Chosen: the maintainer reviewed three mocked layouts (grouped by chain; grouped with the non-enabled
chains folded into one "Other networks" block; one chain at a time behind chain tabs) and picked the
folded one. Enabled chains render as groups with a status pill; the rest sit behind a single
collapsed row that opens into the same groups, each with an "Enable chain" link.

Alternatives rejected: the flat list (previous behaviour) buried the two actionable rows under eight
locked ones repeating the same hint; showing every chain's group open at once (option A) is an
equally long scroll; chain tabs (option C) lose the single view of everything connected across
chains, which is what an operator running several chains checks first.
